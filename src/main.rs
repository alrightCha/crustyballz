mod config;
mod game;
mod managers;
mod map;
mod recv_messages;
mod send_messages;
mod utils;

use crate::utils::util::{create_random_position, mass_to_radius};
use axum_server::tls_rustls::RustlsConfig;
use config::{get_current_config, Config};
use game::Game;
use map::food::Food;
use map::player::{self, Player};
use map::point::Point;
use recv_messages::{ChatMessage, RecvEvent, UsernameMessage};
use recv_messages::{GotItMessage, TargetMessage, WindowResizedMessage};
use rust_socketio::asynchronous::{Client, ClientBuilder};
use send_messages::{PlayerJoinMessage, SendEvent, WelcomeMessage};
use time::OffsetDateTime;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
//Debugging
use dotenv::dotenv;
use log::{error, info};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use std::{net::SocketAddr, path::PathBuf};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

//JSON RESP
use serde_json::json;
use serde_json::Value;
//Server routing
use axum::routing::get;
use axum::Router;
use std::fs::OpenOptions;
use std::{env, fs};
use utils::queue_message::QueueMessage;
use utils::util::valid_nick;

//For socket reference
use once_cell::sync::Lazy;
use std::sync::Arc;

//Websockets Client
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
//Websockets Server
use socketioxide::{
    extract::{Data, SocketRef},
    SocketIo,
};

fn setup_logger() -> Result<(), fern::InitError> {
    let logs_folder = "logs";
    let _ = fs::create_dir(logs_folder);
    let _ = fs::remove_file(format!("{}/default_output.log", logs_folder));

    let log_name = format!("output_{}", chrono::Utc::now().timestamp());

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                OffsetDateTime::now_utc(),
                // humantime::format_rfc3339_seconds(SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(
            OpenOptions::new()
                .write(true)
                .create(true)
                .open(format!("{}/default_output.log", logs_folder))?,
        )
        .chain(fern::log_file(format!(
            "{}/{}.log",
            logs_folder, &log_name
        ))?)
        .apply()?;

    info!("Log File: {}", log_name);

    Ok(())
}

// pub type ClientWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn setup_matchmaking_service() -> Option<Client> {
    let url = "https://eu.cryptoballz.xyz:443";

    Some(ClientBuilder::new(url)
        .connect()
        .await
        .expect("Matchmaking websockets connection failed"))
}

// async fn setup_matchmaking_service() -> Option<Mutex<ClientWebSocket>> {
//     let mode = env::var("MODE").unwrap_or("DEBUG".to_string());

//     if mode == "DEBUG" {
//         return None;
//     }

//     let url = "https://127.0.0.1:443";

//     let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
//     println!("WebSocket handshake has been successfully completed");

//     Some(Mutex::new(ws_stream))
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenv().unwrap();
    setup_logger().unwrap();

    let (layer, io_socket) = SocketIo::new_layer();

    let mode = env::var("MODE").unwrap_or("DEBUG".to_string());

    let match_marking_socket = match mode.as_str() {
        "DEBUG" => None,
        _ => setup_matchmaking_service().await
    };

    let game = Arc::new(Game::new(io_socket.clone(), match_marking_socket));
    let game_cloned = game.clone();

    // tokio spawn game loop
    tokio::spawn(async move {
        game_cloned.tick_game().await;
    });

    let game_cloned = game.clone();

    io_socket.ns("/", |s: SocketRef| {
        info!("Socket connected: {}", s.id);
        let main_room: &'static str = "main"; //main room that holds all the users
        let _ = s.leave_all();
        let _ = s.join(main_room);

        let player = Player::new(s.id);
        let player_ref: Arc<RwLock<Player>> = Arc::new(RwLock::new(player));
        let game_ref = game_cloned;

        let player_ref_cloned = player_ref.clone();
        let game_ref_cloned = game_ref.clone();

        s.on(
            RecvEvent::Respawn,
            |socket: SocketRef, Data::<UsernameMessage>(data)| async move {
                let config = get_current_config();
                let player_data = player_ref_cloned.read().await.generate_player_data();
                let _ = socket.emit(
                    SendEvent::Welcome,
                    (
                        player_data,
                        WelcomeMessage {
                            height: config.game_height,
                            width: config.game_width,
                        },
                    ),
                );

                let _ = game_ref_cloned
                    .io_socket
                    .within("main")
                    .emit(SendEvent::Respawned, json!({ "name": data.name }));
                info!("Received respawn for user: {:?}", data.name);
                /*
                            map.players.remove_player_by_id(new_player.id);
                // Emit 'welcome' back to the socket with configuration details

                // If a name was provided, emit a global 'respawned' event
                info!("[INFO] User {} has respawned", name.name);
                 */
            },
        );

        s.on(RecvEvent::PingCheck, |socket: SocketRef| {
            let _ = socket.emit(SendEvent::PongCheck, ());
        });

        let new_player_clone = player_ref.clone();
        s.on(
            RecvEvent::PlayerMousePosition,
            |socket: SocketRef, Data::<TargetMessage>(data)| async move {
                let mut player = new_player_clone.write().await;
                player.target_x = data.target.x;
                player.target_y = data.target.y;
            },
        );

        let game_ref_cloned = game_ref.clone();
        let new_player_clone = player_ref.clone();
        s.on(
            RecvEvent::PlayerSendingMass,
            |socket: SocketRef| async move {
                let config = get_current_config();
                let mut player = new_player_clone.write().await;

                if player.total_mass < config.min_cell_mass() {
                    return ();
                }

                let player_position = player.get_position_point();
                let player_target = player.get_target_point();
                let player_hue = player.hue;

                let mut mass_food_manager = game_ref_cloned.mass_food_manager.write().await;
                for cell in player.cells.iter_mut() {
                    if cell.mass >= config.min_cell_mass() {
                        cell.remove_mass(config.fire_food as f32);
                        mass_food_manager.add_new(
                            &player_position,
                            &player_target,
                            &cell.position,
                            player_hue,
                            config.fire_food as f32,
                        );
                    }
                }
            },
        );

        let new_player_clone = player_ref.clone();
        s.on(RecvEvent::PlayerSplit, |socket: SocketRef| async move {
            let config = get_current_config();
            let mut player = new_player_clone.write().await;

            player.user_split(config.limit_split as usize, config.split_min_mass as f32);
            let _ = socket.emit(SendEvent::TellPlayerSplit, ());
        });

        let new_player_clone = player_ref.clone();
        let game_ref_cloned = game_ref.clone();
        s.on(
            RecvEvent::PlayerGoIt,
            |socket: SocketRef, Data::<GotItMessage>(data)| async move {
                if let Some(ref name) = data.name {
                    if !valid_nick(name) {
                        // kick_player
                        let _ = socket.emit(SendEvent::KickPlayer, "invalid username.");
                        error!("Invalid username");
                    }
                }

                let mut player = new_player_clone.write().await;

                player.init(
                    game_ref_cloned.create_player_spawn_point(),
                    get_current_config().default_player_mass,
                    data.name,
                    data.screenWidth as f32,
                    data.screenHeight as f32,
                    data.imgUrl,
                );
                drop(player);

                game_ref_cloned.add_player(new_player_clone.clone()).await;

                let player = new_player_clone.read().await;

                let _ = game_ref_cloned.io_socket.emit(
                    SendEvent::PlayerJoin,
                    PlayerJoinMessage {
                        name: player.name.clone(),
                    },
                );

                info!("Player[{:?} / {}] spawned", player.name, player.id);
            },
        );

        let new_player_clone = player_ref.clone();
        s.on(
            RecvEvent::PlayerWindowResized,
            |socket: SocketRef, Data::<WindowResizedMessage>(data)| async move {
                let mut player = new_player_clone.write().await;
                player.screen_height = data.screenHeight as f32;
                player.screen_width = data.screenWidth as f32;
            },
        );

        s.on(
            RecvEvent::PlayerChat,
            |socket: SocketRef, Data::<ChatMessage>(data)| {
                info!("Received data: {:?}", data);
                let _ = socket
                    .within(&*main_room)
                    .emit(SendEvent::ServerPlayerChat, data);
            },
        );

        let new_player_clone = player_ref.clone();
        let game_ref_cloned = game_ref.clone();
        s.on_disconnect(|| async move {
            let player = new_player_clone.read().await;

            game_ref_cloned
                .update_queue
                .lock()
                .await
                .push_back(QueueMessage::KickPlayer {
                    name: player.name.clone(),
                    id: player.id,
                    socket_id: player.socket_id
                })
        });
    });

    let app = Router::new()
        .route("/", get(|| async { "wow much big ballz" }))
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
                .layer(layer),
        );

    let ws_port: u16 = env::args()
        .nth(1)
        .unwrap_or("8000".to_string())
        .parse()
        .expect("Error parsing ws port, invalid argument.");

    let addr = SocketAddr::from(([0, 0, 0, 0], ws_port));

    info!("Starting Server [{}] at: {}", mode, addr);

    if mode == "PRODUCTION" {
        // configure certificate and private key used by https
        let config = RustlsConfig::from_pem_file(
            PathBuf::from(env::var("CERTIFICATE_DIR").expect("Certificate directory not defined"))
                .join("fullchain.pem"),
            PathBuf::from(env::var("CERTIFICATE_DIR").expect("Certificate directory not defined"))
                .join("privkey.pem"),
        )
        .await
        .unwrap();

        axum_server::bind_rustls(addr, config)
            .serve(app.into_make_service())
            .await
            .unwrap();
    } else {
        // DEBUG MODE
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    }
    Ok(())
}
