#[cfg(test)]
pub mod test {
    use axum_server::tls_rustls::RustlsConfig;
    // use clap::Parser;
    use socketioxide::socket::Sid;
    use tokio::time::sleep;
    use crate::config::get_current_config;
    use crate::game::Game;
    use crate::map::player::Player;
    use crate::recv_messages::{
        AmountMessage, ChatMessage, LetMeInMessage, RecvEvent, TargetMessage, UserIdMessage,
    };
    use rust_socketio::asynchronous::{Client, ClientBuilder};
    use rust_socketio::Payload;
    use crate::send_messages::{MassFoodAddedMessage, PlayerJoinMessage, SendEvent, WelcomeMessage};
    use time::OffsetDateTime;
    use tokio::sync::{Mutex, RwLock};
    //Debugging
    use dotenv::dotenv;
    use log::{error, info, warn};
    use std::future::Future;
    use std::net::Ipv4Addr;
    use std::str::FromStr;
    use std::{net::SocketAddr, path::PathBuf};
    use tower::ServiceBuilder;
    use tower_http::compression::CompressionLayer;
    use tower_http::cors::CorsLayer;
    use crate::utils::amount_queue::AmountQueue;
    use crate::utils::id::PlayerID;
    //JSON RESP
    use serde_json::{from_value, json};
    //Server routing
    use axum::routing::get;
    use axum::Router;
    use std::collections::VecDeque;
    use std::fs::OpenOptions;
    use std::{env, fs};
    use crate::utils::queue_message::QueueMessage;
    use crate::utils::util::{get_current_timestamp_micros, valid_nick};

    //For socket reference
    use std::sync::{Arc, OnceLock};

    //Websockets Client
    use futures_util::FutureExt;
    //Websockets Server
    use socketioxide::{
        extract::{Data, SocketRef},
        SocketIo,
    };

    use core::time::Duration;

    // #[derive(Parser)]
    // #[command(about, long_about = None)]
    // pub struct Cli {
    //     pub port: u16,
    //     pub sub_domain: String,
    // }

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
    
    pub fn get_websockets_port(port : u16) -> &'static u16 {
        static PORT: OnceLock<u16> = OnceLock::new();
    
        PORT.get_or_init(|| port)
    }
    
    async fn setup_matchmaking_service(
        amount_queue: Arc<Mutex<VecDeque<AmountQueue>>>,
        sub_domain: String,
    ) -> Option<Client> {
    
        let url_domain = sub_domain;
    
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

    async fn run_game<F, Fut>(scenario_callback : F) where 
        F: FnOnce(Arc<Game>) -> Fut,
        Fut: Future<Output=bool>
    {
        // Default setting for test
        let test_port = 8080;

        setup_logger().unwrap();

        let (layer, io_socket) = SocketIo::new_layer();

        let amount_queue: Arc<Mutex<VecDeque<AmountQueue>>> = Arc::new(Mutex::new(VecDeque::new()));
        let shared_queue = Arc::clone(&amount_queue);
        // let match_making_socket: Option<Client> = setup_matchmaking_service(amount_queue, &cli).await;
        let match_making_socket = None;

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
        let game_cloned_test = game.clone();
        info!("Game started! Waiting for players");

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

            s.on(
                RecvEvent::PlayerGotIt,
                |socket: SocketRef, Data::<UserIdMessage>(data)| async move {
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
                    //MARK: Added newly
                    if let Some(socket_mtchmkng) = &game_ref_cloned.matchmaking_socket {
                        if let Some(ref user_id) = data.user_id {
                            info!("User id game received {}", user_id);
                            let json_payload = json!({"id": user_id, "uid": player.id});
                            let _ = socket_mtchmkng.emit("getAmount", json_payload).await;
                        }
                    }
                },
            );

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
                |_socket: SocketRef, Data::<TargetMessage>(data)| async move {
                    let mut player = new_player_clone.write().await;
                    info!("player {:?} position : {}, {}", player.name, data.target.x, data.target.y);
                    player.target_x = data.target.x;
                    player.target_y = data.target.y;
                },
            );

            let game_ref_cloned = game_ref.clone();
            let new_player_clone = player_ref.clone();
            s.on(
                RecvEvent::PlayerSendingMass,
                |_socket: SocketRef| async move {
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

            let teleport_player_clone = player_ref.clone();
            s.on(RecvEvent::Teleport, |_socket: SocketRef| async move {
                let points = game
                    .player_manager
                    .read()
                    .await
                    .collect_and_clone_all_pos()
                    .await;
                let spawn_point = game.create_player_spawn_point(points);
                let mut player = teleport_player_clone.write().await;
                player.teleport(&spawn_point);
            });

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

        scenario_callback(game_cloned_test).await;

        let compression_layer: CompressionLayer = CompressionLayer::new().deflate(true);

        let app = Router::new()
            .route("/", get(|| async { "wow much big ballz" }))
            .layer(
                ServiceBuilder::new()
                    .layer(compression_layer)
                    .layer(CorsLayer::permissive())
                    .layer(layer),
            );

        let ws_port: u16 = *get_websockets_port(test_port);

        let ip_address = Ipv4Addr::from_str(
            env::var("HOST_IPV4")
                .unwrap_or("127.0.0.1".to_string())
                .as_str(),
        )
        .unwrap();
        let addr = SocketAddr::from((ip_address, ws_port));

        // info!("Starting Server [test] at: {}", addr);
        
        // TEST MODE
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();

    }

    async fn move_player_with_mouse(player_lock : Arc<RwLock<Player>>) {
        let mut player = player_lock.write().await;
        player.target_x = 10000.0;
        player.target_y = 10000.0;
        drop(player);

        sleep(Duration::from_secs_f64(1.0)).await;

        let mut player = player_lock.write().await;
        player.target_x = 1000.0;
        player.target_y = 1000.0;
        drop(player);
    }

    async fn create_player(game: Arc<Game>, index : usize) -> Arc<RwLock<Player>> {
        let mut player = Player::new(PlayerID::MAX, Sid::ZERO);
        player.setup(Some("testuser".to_string() + index.to_string().as_str()), None);
        let player_cloned = Arc::new(RwLock::new(player.clone()));
        game.add_player(player_cloned.clone()).await;
        game.respawn_player(player_cloned.clone()).await;
        player_cloned
    }

    #[tokio::test]
    async fn random_collision_scenario() {
        run_game(|game : Arc<Game>| async move {
            const NUM_OF_PLAYER : usize = 3;
            let mut test_players : Vec<Arc<RwLock<Player>>> = vec![];

            for idx in 0..NUM_OF_PLAYER {
                test_players.push(create_player(game.clone(), idx).await);
            }

            sleep(Duration::from_secs_f64(3.0)).await;

            // move players
            for player in test_players.iter() {
                move_player_with_mouse(player.to_owned()).await;
            };
            true
        }).await;
    }
}