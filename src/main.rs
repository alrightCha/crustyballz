mod config;
mod game;
mod managers;
mod map;
mod player_connection;
mod recv_messages;
mod send_messages;
mod utils;

use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use config::get_current_config;
use game::Game;
use map::player::Player;
use player_connection::PlayerConnection;
use recv_messages::{
    AmountMessage, AnyEventPacket, ChatMessage, LetMeInMessage, RecvEvent, TargetMessage,
    UserIdMessage,
};
use rust_socketio::asynchronous::{Client, ClientBuilder};
use rust_socketio::Payload;
use send_messages::{MassFoodAddedMessage, PlayerJoinMessage, SendEvent, WelcomeMessage};
use time::OffsetDateTime;
use tokio::select;
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
        // .chain(std::io::stdout())
        // .chain(
        //     OpenOptions::new()
        //         .write(true)
        //         .create(true)
        //         .open(format!("{}/default_output.log", logs_folder))?,
        // )
        .chain(fern::log_file(format!(
            "{}/{}.log",
            logs_folder, &log_name
        ))?)
        .apply()?;

    info!("Log File: {}", log_name);
    println!("Log File: {}", log_name);
    Ok(())
}

pub fn get_server_port() -> &'static u16 {
    static PORT: OnceLock<u16> = OnceLock::new();

    PORT.get_or_init(|| match Cli::try_parse() {
        Ok(cli) => cli.port,
        Err(err) => {
            error!("Error parsing CLI args: {:?}", err);
            warn!("Websockets port not passed, using default port: 4433");
            4433
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

async fn start_webtransport_server(game_ref: Arc<Game>, server_port: u16) -> anyhow::Result<()> {
    info!("webtransport test");

    let config = {
        let identify = match env::var("MODE").unwrap_or("DEBUG".to_string()).as_str() {
            "DEBUG" => Identity::load_pemfiles("test_cert.pem", "test_key.pem").await?,

            _ => {
                let pemfiles_folder = PathBuf::from(
                    env::var("CERTIFICATE_DIR").expect("Certificate directory not defined"),
                );

                Identity::load_pemfiles(
                    pemfiles_folder.join("fullchain.pem"),
                    pemfiles_folder.join("privkey.pem"),
                )
                .await?
            }
        };

        ServerConfig::builder()
            .with_bind_default(server_port)
            .with_identity(identify)
            .keep_alive_interval(Some(Duration::from_secs(5)))
            .build()
    };

    let server = Endpoint::server(config)?;

    info!(
        "WebTransport Server is ready at : [{}]",
        server.local_addr().unwrap()
    );

    for _ in 0.. {
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
    info!("Waiting for session request...");

    // Awaits session request
    let session_request = incoming_session.await?;
    // Accepts request & Awaits ready session
    let connection = session_request.accept().await?;

    info!("Waiting for data from client...");

    let (s_send, mut s_recv) = connection.accept_bi().await?;

    let player_connection = PlayerConnection::new(connection.clone(), Mutex::new(s_send));
    let player_connection = Arc::new(player_connection);

    info!("Accepted BI stream");

    let player = Player::new(PlayerID::MAX);
    let player_ref: Arc<RwLock<Player>> = Arc::new(RwLock::new(player));

    let mut player_welcome: bool = false;

    let mut is_disconnected: bool = false;

    // Buffer of U16 MAX ( 65535 bytes )
    let mut buffer = vec![0; u16::MAX.into()].into_boxed_slice();
    let mut tmp_buffer: Vec<u8> = vec![];

    let mut packet_length: usize = 0;

    let mut new_buffer_len: usize = 0;
    let mut current_offset: usize = 0;
    loop {
        new_buffer_len = 0;
        current_offset = 0;

        select! {
            read_result = s_recv.read(&mut buffer) => {
                match read_result {
                    Ok(bytes_read) => {
                        new_buffer_len = bytes_read.unwrap_or_default();
                    },
                    Err(err) => {
                        is_disconnected = true;
                        error!("Error Reading Packet, err={:?}", err);
                    }
                }
            },
            closed_result = player_connection.w_connection.closed() => {
                is_disconnected = true;
                error!("Player Connection was closed, err={:?}", closed_result);
            }
        }

        if is_disconnected {
            let player = player_ref.read().await;

            game_ref
                .update_queue
                .lock()
                .await
                .push_back(QueueMessage::KickPlayer {
                    name: player.name.clone(),
                    id: player.id,
                });

            break;
        }

        if new_buffer_len == 0 {
            continue;
        }

        // JOIN BUFFER WITH TMP_BUFFER
        tmp_buffer.extend_from_slice(&buffer[..new_buffer_len]);

        let mut packets: Vec<AnyEventPacket> = vec![];

        while tmp_buffer.len() - current_offset >= 2 {
            if packet_length == 0 {
                packet_length = u16::from_be_bytes([
                    tmp_buffer[current_offset],
                    tmp_buffer[current_offset + 1],
                ]) as usize;

                current_offset += 2;
            }

            // Check if tmp_buffer enough bytes
            if (tmp_buffer.len() - current_offset) < packet_length {
                break;
            }

            match serde_json::from_slice(
                &tmp_buffer[current_offset..(current_offset + (packet_length as usize))],
            ) {
                Ok(packet) => {
                    packets.push(packet);
                }
                Err(err) => {
                    error!("Error parsing event packet: {:?}", err);
                    // let string = core::str::from_utf8(&tmp_buffer[current_offset..(current_offset + (packet_length as usize))]).unwrap();
                    // error!("Content: {:?}", string);
                }
            }

            current_offset += packet_length;
            packet_length = 0;
        }

        tmp_buffer.drain(..current_offset);

        for packet in packets {
            let recv_event = RecvEvent::from(packet.event.as_str());

            if player_welcome {
                match recv_event {
                    RecvEvent::Respawn => {
                        game_ref.respawn_player(player_ref.clone()).await;
                    }
                    RecvEvent::PingCheck => {
                        let _ = player_connection
                            .emit_bi(SendEvent::PongCheck, get_current_timestamp_micros())
                            .await;
                    }
                    RecvEvent::PlayerMousePosition => {
                        if packet.value.is_none() {
                            continue;
                        }

                        let data: TargetMessage =
                            match serde_json::from_value(packet.value.unwrap()) {
                                Ok(d) => d,
                                Err(err) => {
                                    error!("Error parsing packet [TargetMessage]: {:?}", err);
                                    continue;
                                }
                            };

                        let mut player = player_ref.write().await;
                        // info!("Player[{:?}] - {:?}", player.name, data);
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

                                let _ = game_ref
                                    .emit_bi_broadcast(
                                        SendEvent::MassFoodAdded,
                                        MassFoodAddedMessage(mass_food_init_data),
                                    )
                                    .await;
                            }
                        }
                    }

                    RecvEvent::Cashout => {
                        info!("Cashing out user...");
                        game_ref.cash_out_player(player_ref.clone()).await;
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

                        let _ = player_connection
                            .emit_bi(SendEvent::NotifyPlayerSplit, ())
                            .await;
                    }
                    RecvEvent::PlayerChat => {
                        if packet.value.is_none() {
                            continue;
                        }

                        let data: ChatMessage = match serde_json::from_value(packet.value.unwrap())
                        {
                            Ok(d) => d,
                            Err(err) => {
                                error!("Error parsing packet [ChatMessage]: {:?}", err);
                                continue;
                            }
                        };

                        let _ = game_ref
                            .emit_bi_broadcast(SendEvent::PlayerMessage, data)
                            .await;
                    }
                    _ => {}
                }
            } else {
                match recv_event {
                    RecvEvent::LetMeIn => {
                        if packet.value.is_none() {
                            continue;
                        }
                        let data: LetMeInMessage =
                            match serde_json::from_value(packet.value.unwrap()) {
                                Ok(d) => d,
                                Err(err) => {
                                    error!("Error parsing packet [LetMeInMessage]: {:?}", err);
                                    continue;
                                }
                            };

                        let config = get_current_config();
                        if let Some(ref uid) = data.user_id {
                            // Attempt to parse the user ID string as an integer
                            match uid.parse::<usize>() {
                                Ok(numeric_id) => {
                                    if numeric_id >= 10000 {
                                        // Kick the player for having an ID that is too high
                                        let _ = player_connection
                                            .emit_bi(SendEvent::KickPlayer, "User ID too high.")
                                            .await;
                                        error!("Player kicked for too-high user ID: {}", uid);
                                    }
                                }
                                Err(_) => {
                                    // Handle the case where the user ID is not a valid number
                                    let _ = player_connection
                                        .emit_bi(SendEvent::KickPlayer, "Invalid user ID.")
                                        .await;
                                    error!("Player kicked for invalid user ID: {}", uid);
                                }
                            }
                        }
                        //kicking if url too long for image
                        if let Some(ref img_url) = data.img_url {
                            // Define a more generous maximum acceptable length for the URL
                            let max_url_length = 1000; // Adjusted to 1000 characters

                            // Check if the URL exceeds this length
                            if img_url.len() > max_url_length {
                                // Kick the player for having a too-long URL
                                let _ = player_connection
                                    .emit_bi(SendEvent::KickPlayer, "URL too long.")
                                    .await;
                                error!("Player kicked for too-long URL: {}", img_url);
                            }
                        }

                        if let Some(ref name) = data.name {
                            if !valid_nick(name) {
                                // kick_player
                                let _ = player_connection
                                    .emit_bi(SendEvent::KickPlayer, "invalid username.")
                                    .await;
                                error!("Invalid username");
                            }
                        }

                        {
                            let mut player = player_ref.write().await;
                            player.setup(data.name, data.img_url);
                        }
                        let start = game_ref.game_start;
                        let _ = player_connection
                            .emit_bi(
                                SendEvent::Welcome,
                                WelcomeMessage {
                                    height: config.game_height,
                                    width: config.game_width,
                                    default_player_mass: config.default_player_mass,
                                    default_mass_food: config.food_mass,
                                    default_mass_mass_food: config.fire_food,
                                    start: start,
                                },
                            )
                            .await;
                    }
                    RecvEvent::PlayerGotIt => {
                        if packet.value.is_none() {
                            continue;
                        }

                        let data: UserIdMessage =
                            match serde_json::from_value(packet.value.unwrap()) {
                                Ok(d) => d,
                                Err(err) => {
                                    error!("Error parsing packet [UserIdMessage]: {:?}", err);
                                    continue;
                                }
                            };

                        game_ref
                            .add_player(player_ref.clone(), player_connection.clone())
                            .await;

                        let player = player_ref.read().await;
                        let player_init_data = player.generate_init_player_data();

                        let _ = player_connection
                            .emit_bi(SendEvent::PlayerInitData, player_init_data.clone())
                            .await;

                        player_welcome = true;

                        let _ = game_ref
                            .emit_bi_broadcast(
                                SendEvent::NotifyPlayerJoined,
                                PlayerJoinMessage(player_init_data),
                            )
                            .await;

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
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

// async fn handle_any_event_packet(packet: AnyEventPacket) {
//     let recv_event = RecvEvent::from(packet.event);

//     match recv_event {
//         RecvEvent::LetMeIn => {
//             if packet.value.is_none() {
//                 // continue;
//             }
//             let data: LetMeInMessage = match serde_json::from_value(packet.value.unwrap()) {
//                 Ok(d) => d,
//                 Err(err) => {
//                     error!("Error parsing packet [LetMeInMessage]: {:?}", err);
//                     // continue;
//                 }
//             };

//             let config = get_current_config();

//             if let Some(ref name) = data.name {
//                 if !valid_nick(name) {
//                     // kick_player
//                     let _ = player_connection
//                         .emit_bi(SendEvent::KickPlayer, "invalid username.")
//                         .await;
//                     error!("Invalid username");
//                 }
//             }

//             {
//                 let mut player = player_ref.write().await;
//                 player.setup(data.name, data.img_url);
//             }

//             let _ = player_connection
//                 .emit_bi(
//                     SendEvent::Welcome,
//                     WelcomeMessage {
//                         height: config.game_height,
//                         width: config.game_width,
//                         default_player_mass: config.default_player_mass,
//                         default_mass_food: config.food_mass,
//                         default_mass_mass_food: config.fire_food,
//                     },
//                 )
//                 .await;
//         }
//         RecvEvent::PlayerGotIt => {
//             if packet.value.is_none() {
//                 // continue;
//             }

//             let data: UserIdMessage = match serde_json::from_value(packet.value.unwrap()) {
//                 Ok(d) => d,
//                 Err(err) => {
//                     error!("Error parsing packet [UserIdMessage]: {:?}", err);
//                     // continue;
//                 }
//             };

//             game_ref
//                 .add_player(player_ref.clone(), player_connection.clone())
//                 .await;

//             let player = player_ref.read().await;
//             let player_init_data = player.generate_init_player_data();

//             let _ = player_connection
//                 .emit_bi(SendEvent::PlayerInitData, player_init_data.clone())
//                 .await;

//             let _ = game_ref
//                 .emit_bi_broadcast(
//                     SendEvent::NotifyPlayerJoined,
//                     PlayerJoinMessage(player_init_data),
//                 )
//                 .await;

//             info!("Player[{:?} / {}] joined", player.name, player.id);
//             //MARK: Added newly
//             if let Some(socket_mtchmkng) = &game_ref.matchmaking_socket {
//                 if let Some(ref user_id) = data.user_id {
//                     info!("User id game received {}", user_id);
//                     let json_payload = json!({"id": user_id, "uid": player.id});
//                     let _ = socket_mtchmkng.emit("getAmount", json_payload).await;
//                 }
//             }
//         }
//         RecvEvent::Respawn => {
//             game_ref.respawn_player(player_ref.clone()).await;
//         }
//         RecvEvent::PingCheck => {
//             let _ = player_connection
//                 .emit_bi(SendEvent::PongCheck, get_current_timestamp_micros())
//                 .await;
//         }
//         RecvEvent::PlayerMousePosition => {
//             if packet.value.is_none() {
//                 // continue;
//             }

//             let data: TargetMessage = match serde_json::from_value(packet.value.unwrap()) {
//                 Ok(d) => d,
//                 Err(err) => {
//                     error!("Error parsing packet [TargetMessage]: {:?}", err);
//                     // continue;
//                 }
//             };

//             let mut player = player_ref.write().await;
//             // info!("Player[{:?}] - {:?}", player.name, data);
//             player.target_x = data.target.x;
//             player.target_y = data.target.y;
//         }
//         RecvEvent::PlayerSendingMass => {
//             let config = get_current_config();
//             let mut player = player_ref.write().await;

//             if player.total_mass < config.min_cell_mass() as usize {
//                 // continue;
//             }

//             let player_position = player.get_position_point();
//             let player_target = player.get_target_point();
//             let player_hue = player.hue;

//             let mut mass_food_manager = game_ref.mass_food_manager.write().await;
//             for cell in player.cells.iter_mut() {
//                 if cell.mass >= config.min_cell_mass() {
//                     cell.remove_mass(config.fire_food);
//                     let mass_food_init_data = mass_food_manager.add_new(
//                         &player_position,
//                         &player_target,
//                         &cell.position,
//                         player_hue,
//                         config.fire_food,
//                     );

//                     let _ = game_ref
//                         .emit_bi_broadcast(
//                             SendEvent::MassFoodAdded,
//                             MassFoodAddedMessage(mass_food_init_data),
//                         )
//                         .await;
//                 }
//             }
//         }
//         RecvEvent::Teleport => {
//             let points = game_ref
//                 .player_manager
//                 .read()
//                 .await
//                 .collect_and_clone_all_pos()
//                 .await;
//             let spawn_point = game_ref.create_player_spawn_point(points);

//             {
//                 let mut player = player_ref.write().await;
//                 player.teleport(&spawn_point);
//             }
//         }
//         RecvEvent::PlayerSplit => {
//             let config = get_current_config();

//             {
//                 let mut player = player_ref.write().await;
//                 player.user_split(config.limit_split as usize, config.split_min_mass);
//             }

//             let _ = player_connection
//                 .emit_bi(SendEvent::NotifyPlayerSplit, ())
//                 .await;
//         }
//         RecvEvent::PlayerChat => {
//             if packet.value.is_none() {
//                 // continue;
//             }

//             let data: ChatMessage = match serde_json::from_value(packet.value.unwrap()) {
//                 Ok(d) => d,
//                 Err(err) => {
//                     error!("Error parsing packet [ChatMessage]: {:?}", err);
//                     // continue;
//                 }
//             };

//             let _ = game_ref
//                 .emit_bi_broadcast(SendEvent::PlayerMessage, data)
//                 .await;
//         }
//         _ => {}
//     }
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenv().unwrap();
    setup_logger().unwrap();

    let mode = env::var("MODE").unwrap_or("DEBUG".to_string());
    // if mode == "DEBUG" {
    //     println!("Debugging Mode: Verbose logging enabled.");
    //     setup_logger()?;
    // }

    let (layer, io_socket) = SocketIo::new_layer();

    //MARK: ADDED NEWLY
    let amount_queue: Arc<Mutex<VecDeque<AmountQueue>>> = Arc::new(Mutex::new(VecDeque::new()));
    let shared_queue = Arc::clone(&amount_queue);
    let match_making_socket: Option<Client> = setup_matchmaking_service(amount_queue).await;
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
    let server_port: u16 = *get_server_port();

    tokio::spawn(start_webtransport_server(game_cloned.clone(), server_port));

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

    let ip_address = Ipv4Addr::from_str(
        env::var("HOST_IPV4")
            .unwrap_or("127.0.0.1".to_string())
            .as_str(),
    )
    .unwrap();
    let addr = SocketAddr::from((ip_address, server_port));

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
