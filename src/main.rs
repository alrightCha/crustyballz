mod config;
mod map;
mod utils;
mod store;

use store::UserSockets;
use config::Config;
use map::player::{Cell, Player};
use map::map::Map;
use crate::utils::util::{get_position, mass_to_radius};
use map::point::Point;
//Debugging 
use tracing::info;
use tracing_subscriber::FmtSubscriber;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

//JSON RESP 
use serde_json::Value;
use serde_json::json;
//Server routing
use axum::{Router, Server};
use axum::routing::get;
use std::net::SocketAddr;
use std::env;

//For socket reference 
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

//Websockets 
use socketioxide::{
    extract::{Data, SocketRef},
     SocketIo
};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Username{
    name: String
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Message {
    message: String, 
    sender: String
}

static USER_SOCKETS: Lazy<Arc<UserSockets>> = Lazy::new(|| {
    Arc::new(UserSockets::new())
});

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = tracing::subscriber::set_global_default(FmtSubscriber::default())?;
    let _config: Config = Config::default();
    let server_port = env::var("SERVER_PORT").unwrap_or_else(|_| "8000".to_string());
    let map = Map::new();
    let (layer, io) = SocketIo::new_layer();
    let mass_init: f32 = _config.default_player_mass;

    let generate_spawnpoint = || {
        let radius = mass_to_radius(mass_init);
        let points: Vec<Point> = {
            let locked_map = map;
            locked_map.players.players.iter()
                .flat_map(|player| player.cells.iter().map(|cell| cell.position))
                .collect()
        };
        // Since 'map' is captured by the closure, you can use it directly inside the closure
        get_position(true, radius, Some(&points))
    };

    io.ns("/", |s: SocketRef| {
        info!("Socket connected: {}", s.id);
        let main = "main"; //main room that holds all the users
        let _ = s.leave_all();
        let _ = s.join(&*main);
    
        let mut new_player = Player::new();
    
        // Directly store the socket reference
        USER_SOCKETS.add_user(new_player.id, s.id.to_string().clone());

        s.on("respawn", |socket: SocketRef, Data::<Username>(name)| {
            // Log that a respawn request was received
            info!("Received respawn for user: {:?}", name.name);

            /*
                        map.players.remove_player_by_id(new_player.id);
            // Emit 'welcome' back to the socket with configuration details
            let game_width = Config::default().game_width;  // Ensure these are available in the scope or via a config struct
            let game_height = Config::default().game_height;
            let welcome_data = json!({
                "width": game_width,
                "height": game_height
            });
            let _ = socket.emit("welcome", (new_player, welcome_data));
            
            // If a name was provided, emit a global 'respawned' event
            let _ = socket.within(&*main).emit("respawned", json!({ "name": name.name }));
            info!("[INFO] User {} has respawned", name.name);
             */

        });

        s.on("gotit", |socket: SocketRef, Data::<Value>(data)| {
            info!("Recieved following user info: {:?}", data);
            info!("Image got : {:?}", data["imgUrl"]);

        });

        s.on("playerChat", |socket: SocketRef, Data::<Message>(data)| {
            info!("Received data: {:?}", data);
            let _ = socket.within(&*main).emit("serverSendPlayerChat", data);
        });
    });

    let app = Router::new()
        .route("/", get(|| async { "wow much big ballz" }))
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
                .layer(layer),
        );


    let address = format!("127.0.0.1:{}", server_port).parse().unwrap();
    
    info!("Server running {}", server_port);

    Server::bind(&address)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
