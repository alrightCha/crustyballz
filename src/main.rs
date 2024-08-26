mod config;
mod game;
mod managers;
mod map;
mod recv_messages;
mod send_messages;
mod utils;

use crate::utils::util::{create_random_position, mass_to_radius};
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use config::{get_current_config, Config};
use game::Game;
use managers::amount_manager::{self, AmountManager};
use map::food::Food;
use map::player::{self, Player, PlayerInitData};
use map::point::Point;
use recv_messages::{ChatMessage, LetMeInMessage, RecvEvent, TargetMessage, UsernameMessage};
use rust_socketio::asynchronous::{Client, ClientBuilder};
use rust_socketio::{Payload, RawClient};
use send_messages::{
    MassFoodAddedMessage, PlayerJoinMessage, PlayerRespawnedMessage, SendEvent, WelcomeMessage,
};
use time::OffsetDateTime;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
//Debugging
use dotenv::dotenv;
use log::{debug, error, info, warn};
use utils::id::{id_from_position, PlayerID};

use std::env::args;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Instant;
use std::{net::SocketAddr, path::PathBuf};
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
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
use utils::util::{get_current_timestamp_micros, valid_nick};

//For socket reference
use std::sync::{Arc, OnceLock};

//Websockets Client
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
//Websockets Server
use socketioxide::{
    extract::{Data, SocketRef},
    SocketIo,
};

#[derive(Parser)]
#[command(about, long_about = None)]
struct Cli {
    pub port: u16,
    pub sub_domain: String,
}

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

pub fn get_websockets_port() -> &'static u16 {
    static PORT: OnceLock<u16> = OnceLock::new();

    PORT.get_or_init(|| match Cli::try_parse() {
        Ok(cli) => cli.port,
        Err(err) => {
            error!("Error parsing CLI args: {:?}", err);
            warn!("Websockets port not passed, using default port: 8000");
            8000
        }
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenv().unwrap();
    setup_logger().unwrap();

    let (layer, io_socket) = SocketIo::new_layer();

    let mode = env::var("MODE").unwrap_or("DEBUG".to_string());
    //MARK: ADDED NEWLY
    let amount_manager = Arc::new(Mutex::new(AmountManager::new()));

    async fn setup_matchmaking_service(
        amount_manager: Arc<Mutex<AmountManager>>,
    ) -> Option<Client> {
        let url_domain = Cli::try_parse().expect("Error parsing CLI args").sub_domain;
        let amount_manager = amount_manager.clone();
        
        // Since the callback function needs to be asynchronous, use `tokio::spawn`
        tokio::spawn(async move {
            match payload {
                Payload::String(json_string) => {
                    if let Ok(data) = serde_json::from_str::<Value>(&json_string) {
                        if let Some(amount) = data["amount"].as_f64() {
                            if let Some(address) = data["address"].as_str() {
                                if let Some(id) = data["id"].as_i64() {
                                    if let Ok(id) = i8::try_from(id) {
                                        let mut manager = amount_manager.lock().await; // Use asynchronous lock
                                        manager.set_amount(id, amount);
                                        manager.set_address(id, address.to_string());
                                    }
                                }
                            }
                        }
                    } else {
                        info!("Failed to parse payload as JSON: {}", json_string);
                    }
                }
                Payload::Binary(_) => {
                    info!("Received binary data for userAmount, expected JSON string.");
                }
                _ => info!("Unexpected payload type."),
            }
        });
        info!("URL DOMAIN FOR MATCHMAKING : {:?}", url_domain);
        Some(
            ClientBuilder::new(url_domain)
                .on("userAmount", callback)
                .connect()
                .await
                .expect("Matchmaking websockets connection failed"),
        )
    }

    let mut match_marking_socket = match mode.as_str() {
        "DEBUG" => None,
        _ => setup_matchmaking_service(amount_manager.clone()).await,
    };

    let mut manager = amount_manager.lock().await;

    let game = Arc::new(Game::new(
        *manager,
        io_socket.clone(),
        match_marking_socket,
    ));
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

        // create a player with a id place holder
        let player = Player::new(PlayerID::MAX, s.id);

        let player_ref: Arc<RwLock<Player>> = Arc::new(RwLock::new(player));
        let game_ref = game_cloned;

        let player_ref_cloned = player_ref.clone();
        let game_ref_cloned = game_ref.clone();
        s.on(
            RecvEvent::LetMeIn,
            |socket: SocketRef, Data::<LetMeInMessage>(data)| async move {
                let config = get_current_config();

                if let Some(ref name) = data.name {
                    if !valid_nick(name) {
                        // kick_player
                        let _ = socket.emit(SendEvent::KickPlayer, "invalid username.");
                        error!("Invalid username");
                    }
                }

                let mut player = player_ref_cloned.write().await;

                //MARK: Added newly
                if let Some(socket_mtchmkng) = game_ref_cloned.matchmaking_socket {
                    let json_payload = json!({"id": data.user_id});
                    let _ = socket_mtchmkng.emit("getAmount", json_payload).await;
                }
                player.setup(data.name, data.img_url);
                drop(player);

                let _ = socket.emit(
                    SendEvent::Welcome,
                    WelcomeMessage {
                        height: config.game_height,
                        width: config.game_width,
                        default_player_mass: config.default_player_mass,
                        default_mass_food: config.food_mass,
                        default_mass_mass_food: config.fire_food,
                    },
                );
            },
        );

        let player_ref_cloned = player_ref.clone();
        let game_ref_cloned = game_ref.clone();
        s.on(RecvEvent::PlayerGotIt, |socket: SocketRef| async move {
            //
            game_ref_cloned.add_player(player_ref_cloned.clone()).await;

            let player = player_ref_cloned.read().await;
            let player_init_data = player.generate_init_player_data();

            let _ = socket.emit(SendEvent::PlayerInitData, player_init_data.clone());

            let _ = game_ref_cloned.io_socket.emit(
                SendEvent::NotifyPlayerJoined,
                PlayerJoinMessage(player_init_data),
            );

            info!("Player[{:?} / {}] joined", player.name, player.id);
        });

        let player_ref_cloned = player_ref.clone();
        let game_ref_cloned = game_ref.clone();

        s.on(RecvEvent::Respawn, |_: SocketRef| async move {
            game_ref_cloned.respawn_player(player_ref_cloned).await;
        });

        s.on(RecvEvent::PingCheck, |socket: SocketRef| {
            let _ = socket.emit(SendEvent::PongCheck, get_current_timestamp_micros());
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

                if player.total_mass < config.min_cell_mass() as usize {
                    return ();
                }

                let player_position = player.get_position_point();
                let player_target = player.get_target_point();
                let player_hue = player.hue;

                let mut mass_food_manager = game_ref_cloned.mass_food_manager.write().await;
                for cell in player.cells.iter_mut() {
                    if cell.mass >= config.min_cell_mass() {
                        cell.remove_mass(config.fire_food);
                        let mass_food_init_data = mass_food_manager.add_new(
                            &player_position,
                            &player_target,
                            &cell.position,
                            player_hue,
                            config.fire_food,
                        );

                        let _ = game_ref_cloned.io_socket.emit(
                            SendEvent::MassFoodAdded,
                            MassFoodAddedMessage(mass_food_init_data),
                        );
                    }
                }
            },
        );

        let new_player_clone = player_ref.clone();
        s.on(RecvEvent::PlayerSplit, |socket: SocketRef| async move {
            let config = get_current_config();
            let mut player = new_player_clone.write().await;

            player.user_split(config.limit_split as usize, config.split_min_mass);
            let _ = socket.emit(SendEvent::NotifyPlayerSplit, ());
        });

        let game_ref_cloned = game_ref.clone();
        s.on(
            RecvEvent::PlayerChat,
            move |_: SocketRef, Data::<ChatMessage>(data)| {
                let _ = game_ref_cloned
                    .io_socket
                    .within(&*main_room)
                    .emit(SendEvent::PlayerMessage, data);
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
                    socket_id: player.socket_id,
                })
        });
    });

    let compression_layer: CompressionLayer = CompressionLayer::new().deflate(true);

    let app = Router::new()
        .route("/", get(|| async { "wow much big ballz" }))
        .layer(
            ServiceBuilder::new()
                .layer(compression_layer)
                .layer(CorsLayer::permissive())
                .layer(layer),
        );

    let ws_port: u16 = *get_websockets_port();

    let ip_address = Ipv4Addr::from_str(
        env::var("HOST_IPV4")
            .unwrap_or("127.0.0.1".to_string())
            .as_str(),
    )
    .unwrap();
    let addr = SocketAddr::from((ip_address, ws_port));

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
