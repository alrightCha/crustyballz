mod config;
mod game;
mod managers;
mod map;
mod recv_messages;
mod send_messages;
mod utils;

use crate::utils::util::{get_position, mass_to_radius};
use config::{get_current_config, Config};
use game::Game;
use map::food::Food;
use map::player::Player;
use map::point::Point;
use recv_messages::RecvEvent;
use recv_messages::{GotItMessage, TargetMessage, WindowResizedMessage};
use send_messages::SendEvent;
use tokio::sync::RwLock;
//Debugging
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::FmtSubscriber;

//JSON RESP
use serde_json::json;
use serde_json::Value;
//Server routing
use axum::routing::get;
use axum::Router;
use std::env;
use std::net::SocketAddr;

//For socket reference
use once_cell::sync::Lazy;
use std::sync::Arc;

//Websockets
use socketioxide::{
    extract::{Data, SocketRef},
    SocketIo,
};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Username {
    name: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Message {
    message: String,
    sender: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = tracing::subscriber::set_global_default(FmtSubscriber::default())?;
    let _config: Config = Config::default();
    let server_port = env::var("SERVER_PORT").unwrap_or_else(|_| "8000".to_string());
    let (layer, io_socket) = SocketIo::new_layer();
    let mass_init: f32 = _config.default_player_mass;

    let game = Arc::new(Game::new(io_socket.clone()));
    let game_cloned = game.clone();

    // tokio spawn game loop
    tokio::spawn(async move {
        game_cloned.tick_game().await;
    });

    let game_cloned = game.clone();
    let io_socket_cloned = io_socket.clone();

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
            |socket: SocketRef, Data::<Username>(name)| async move {
                let game_width = Config::default().game_width; // Ensure these are available in the scope or via a config struct
                let game_height = Config::default().game_height;
                let welcome_data = json!({
                    "width": game_width,
                    "height": game_height
                });

                let mut player_data = player_ref_cloned.write().await;
                let _ = socket.emit(SendEvent::Welcome, (player_data.clone(), welcome_data));

                let _ = io_socket_cloned
                    .within("main")
                    .emit(SendEvent::Respawned, json!({ "name": name.name }));
                drop(player_data);
                game_ref_cloned.add_player(player_ref_cloned).await;
                info!("Received respawn for user: {:?}", name.name);
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

        let game_ref_cloned = game_ref.clone();
        let new_player_clone = player_ref.clone();
        s.on(RecvEvent::PlayerSplit, |socket: SocketRef| async move {
            let config = get_current_config();
            let mut player = new_player_clone.write().await;

            player.user_split(config.limit_split as usize, config.split_min as f32);
            let _ = socket.emit(SendEvent::TellPlayerSplit, ());
        });

        let new_player_clone = player_ref.clone();
        s.on(
            RecvEvent::PlayerGoit,
            |socket: SocketRef, Data::<GotItMessage>(data)| async move {
                let mut player = new_player_clone.write().await;
                info!("Image got : {:?}", data.imgUrl);
                player.init(
                    Point {
                        x: 0.0,
                        y: 0.0,
                        radius: 0.0,
                    },
                    get_current_config().default_player_mass,
                    data.name,
                    data.screenWidth as f32,
                    data.screenHeight as f32,
                    data.imgUrl,
                );
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
            |socket: SocketRef, Data::<Message>(data)| {
                info!("Received data: {:?}", data);
                let _ = socket
                    .within(&*main_room)
                    .emit(SendEvent::ServerPlayerChat, data);
            },
        );
    });

    let app = Router::new()
        .route("/", get(|| async { "wow much big ballz" }))
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
                .layer(layer),
        );

    // let address = format!("127.0.0.1:{}", server_port).parse().unwrap();
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", server_port))
        .await
        .unwrap();

    info!("Server running {}", server_port);

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
