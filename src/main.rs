mod config;
mod game;
mod managers;
mod map;
mod recv_messages;
mod send_messages;
mod utils;

use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use config::get_current_config;
use game::Game;
use map::player::Player;
use recv_messages::{
    AmountMessage, ChatMessage, LetMeInMessage, RecvEvent, TargetMessage, UserIdMessage,
};
use rust_socketio::asynchronous::{Client, ClientBuilder};
use rust_socketio::Payload;
use send_messages::{MassFoodAddedMessage, PlayerJoinMessage, SendEvent, WelcomeMessage};
use time::OffsetDateTime;
use tokio::sync::{Mutex, RwLock};
//Debugging
use dotenv::dotenv;
use log::{error, info, warn};
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Duration;
use std::{net::SocketAddr, path::PathBuf};
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use utils::amount_queue::AmountQueue;
use utils::id::PlayerID;
use wtransport::endpoint::IncomingSession;
use wtransport::{Endpoint, Identity, ServerConfig};
//JSON RESP
use serde_json::{from_value, json};
//Server routing
use axum::routing::get;
use axum::Router;
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::{env, fs};
use utils::queue_message::QueueMessage;
use utils::util::{get_current_timestamp_micros, valid_nick};

//For socket reference
use std::sync::{Arc, OnceLock};

//Websockets Client
use futures_util::FutureExt;
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

async fn setup_matchmaking_service(
    amount_queue: Arc<Mutex<VecDeque<AmountQueue>>>,
) -> Option<Client> {
    let url_domain = Cli::try_parse().expect("Error parsing CLI args").sub_domain;

    let client = ClientBuilder::new(url_domain)
        .on("userAmount", {
            let amount_queue = amount_queue.clone();
            move |payload: Payload, _: Client| {
                info!("RECEIVED USERAMOUNT RESPONSE");
                let amount_queue = amount_queue.clone(); // Clone the Arc to be used inside async block
                async move {
                    match payload {
                        Payload::Text(json_vec) => {
                            if let Some(json_str) = json_vec.get(0) {
                                info!("Data received: {:?}", json_str);
                                match from_value::<AmountMessage>(json_str.clone()) {
                                    Ok(data) => {
                                        amount_queue.lock().await.push_back(
                                            AmountQueue::AddAmount {
                                                id: data.id,
                                                amount: data.amount,
                                                uid: data.uid,
                                            },
                                        );
                                    }
                                    Err(e) => {
                                        // Handle deserialization error
                                        eprintln!(
                                            "ERROR$$)$)$$)$)$()$) parsing JSON data: {:?}",
                                            e
                                        );
                                    }
                                }
                            }
                        }
                        Payload::Binary(_) => {
                            info!("Received binary data for userAmount, expected JSON string.");
                        }
                        _ => info!("Unexpected payload type."),
                    }
                }
                .boxed()
            }
        })
        .on("open", |err, _| {
            async move { info!("MATCHMAKING OPEN: {:#?}", err) }.boxed()
        })
        .on("error", |err, _| {
            async move { error!("MATCHMAKING ERROR: {:#?}", err) }.boxed()
        })
        .on("close", |err, _| {
            async move { info!("MATCHMAKING CLOSE: {:#?}", err) }.boxed()
        })
        .connect()
        .await
        .expect("Matchmaking websockets connection failed");

    Some(client)
}

async fn start_webtransport_server(game_ref: Arc<Game>) -> anyhow::Result<()> {
    info!("webtransport test");
    let config = ServerConfig::builder()
        .with_bind_default(4433)
        .with_identity(Identity::load_pemfiles("test_cert.pem", "test_key.pem").await?)
        .keep_alive_interval(Some(Duration::from_secs(3)))
        .build();

    let server = Endpoint::server(config)?;

    info!("WebTransport Server is ready at : [{}]", server.local_addr().unwrap());

    for _id in 0.. {
        let incoming_session = server.accept().await;
        tokio::spawn(handle_connection(game_ref.clone(), incoming_session));
    }

    Ok(())
}

trait WebTransportEmit {
    // async fn wt_bi_emit(&mut self, send_event: SendEvent, data: &str) {}
    async fn wt_bi_emit<T: serde::Serialize>(&mut self, send_event: SendEvent, data: T) {}
}

impl WebTransportEmit for wtransport::SendStream {
    async fn wt_bi_emit<T: serde::Serialize>(&mut self, send_event: SendEvent, data: T) {
        // self.write_all(data.as_bytes()).await.unwrap();
    }
}

async fn handle_connection(
    game_ref: Arc<Game>,
    incoming_session: IncomingSession,
) -> anyhow::Result<()> {
    let mut buffer = vec![0; 10000].into_boxed_slice();

    info!("Waiting for session request...");

    // Awaits session request
    let session_request = incoming_session.await?;
    // Accepts request & Awaits ready session
    let connection = session_request.accept().await?;

    info!("Waiting for data from client...");

    let (mut s_send, mut s_recv) = connection.accept_bi().await?;

    info!("Accepted BI stream");

    let fake_id = socketioxide::socket::Sid::ZERO;
    let player = Player::new(PlayerID::MAX, fake_id);
    let player_ref: Arc<RwLock<Player>> = Arc::new(RwLock::new(player));

    loop {
        let is_disconnected: bool = false;

        if is_disconnected {
            let player = player_ref.read().await;

            game_ref
                .update_queue
                .lock()
                .await
                .push_back(QueueMessage::KickPlayer {
                    name: player.name.clone(),
                    id: player.id,
                    socket_id: player.socket_id,
                });

            break;
        }

        let bytes_read = match s_recv.read(&mut buffer).await? {
            Some(bytes_read) => bytes_read,
            None => continue,
        };
        let str_data = std::str::from_utf8(&buffer[..bytes_read])?;
        info!("Received (bi) '{str_data}' from client");
        s_send.write_all(b"ACK").await?;

        let recv_event_number: u8 = str_data.parse().unwrap();
        let recv_event = RecvEvent::from(recv_event_number);

        match recv_event {
            RecvEvent::LetMeIn => {
                let data: LetMeInMessage = todo!();
                let config = get_current_config();

                if let Some(ref name) = data.name {
                    if !valid_nick(name) {
                        // kick_player
                        let _ = s_send
                            .wt_bi_emit(SendEvent::KickPlayer, "invalid username.")
                            .await;
                        error!("Invalid username");
                    }
                }

                let mut player = player_ref.write().await;

                player.setup(data.name, data.img_url);
                drop(player);

                let _ = s_send.wt_bi_emit(
                    SendEvent::Welcome,
                    WelcomeMessage {
                        height: config.game_height,
                        width: config.game_width,
                        default_player_mass: config.default_player_mass,
                        default_mass_food: config.food_mass,
                        default_mass_mass_food: config.fire_food,
                    },
                );
            }
            RecvEvent::PlayerGotIt => {
                let data: UserIdMessage = todo!();

                game_ref.add_player(player_ref.clone()).await;

                let player = player_ref.read().await;
                let player_init_data = player.generate_init_player_data();

                let _ = s_send.wt_bi_emit(SendEvent::PlayerInitData, player_init_data.clone());

                let _ = game_ref.emit_broadcast(
                    SendEvent::NotifyPlayerJoined,
                    PlayerJoinMessage(player_init_data),
                );

                info!("Player[{:?} / {}] joined", player.name, player.id);
                //MARK: Added newly
                if let Some(socket_mtchmkng) = &game_ref.matchmaking_socket {
                    if let Some(ref user_id) = data.user_id {
                        info!("User id game received {}", user_id);
                        let json_payload = json!({"id": user_id, "uid": player.id});
                        let _ = socket_mtchmkng.emit("getAmount", json_payload).await;
                    }
                }
            }
            RecvEvent::Respawn => {
                game_ref.respawn_player(player_ref.clone()).await;
            }
            RecvEvent::PingCheck => {
                let _ = s_send.wt_bi_emit(SendEvent::PongCheck, get_current_timestamp_micros());
            }
            RecvEvent::PlayerMousePosition => {
                let data: TargetMessage = todo!();
                let mut player = player_ref.write().await;
                player.target_x = data.target.x;
                player.target_y = data.target.y;
            }
            RecvEvent::PlayerSendingMass => {
                let config = get_current_config();
                let mut player = player_ref.write().await;

                if player.total_mass < config.min_cell_mass() as usize {
                    continue;
                }

                let player_position = player.get_position_point();
                let player_target = player.get_target_point();
                let player_hue = player.hue;

                let mut mass_food_manager = game_ref.mass_food_manager.write().await;
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

                        let _ = game_ref.emit_broadcast(
                            SendEvent::MassFoodAdded,
                            MassFoodAddedMessage(mass_food_init_data),
                        );
                    }
                }
            }
            RecvEvent::Teleport => {
                let points = game_ref
                    .player_manager
                    .read()
                    .await
                    .collect_and_clone_all_pos()
                    .await;
                let spawn_point = game_ref.create_player_spawn_point(points);

                {
                    let mut player = player_ref.write().await;
                    player.teleport(&spawn_point);
                }
            }
            RecvEvent::PlayerSplit => {
                let config = get_current_config();

                {
                    let mut player = player_ref.write().await;
                    player.user_split(config.limit_split as usize, config.split_min_mass);
                }

                let _ = s_send.wt_bi_emit(SendEvent::NotifyPlayerSplit, ());
            }
            RecvEvent::PlayerChat => {
                let data: ChatMessage = todo!();

                let _ = game_ref
                    .emit_broadcast(SendEvent::PlayerMessage, data);
            }
            _ => {}
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenv().unwrap();
    setup_logger().unwrap();

    let (layer, io_socket) = SocketIo::new_layer();

    let mode = env::var("MODE").unwrap_or("DEBUG".to_string());
    //MARK: ADDED NEWLY
    let amount_queue: Arc<Mutex<VecDeque<AmountQueue>>> = Arc::new(Mutex::new(VecDeque::new()));
    let shared_queue = Arc::clone(&amount_queue);
    let match_making_socket: Option<Client> = match mode.as_str() {
        "DEBUG" => None,
        _ => setup_matchmaking_service(amount_queue).await,
    };
    let game = Arc::new(Game::new(
        io_socket.clone(), // No need to clone, assuming io_socket is already of type SocketIo
        match_making_socket,
        shared_queue,
    ));
    let game_cloned = game.clone();

    // tokio spawn game loop
    tokio::spawn(async move {
        game_cloned.tick_game().await;
    });

    let game_cloned = game.clone();

    tokio::spawn(start_webtransport_server(game_cloned.clone()));

    info!("Game started! Waiting for players");

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
