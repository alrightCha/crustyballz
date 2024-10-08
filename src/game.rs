use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

use log::info;
use rust_socketio::asynchronous::Client;
use socketioxide::{socket::Sid, SocketIo};
use tokio::sync::{Mutex, RwLock};
use tokio_timerfd::sleep;

use crate::{
    config::{get_current_config, Config},
    get_websockets_port,
    managers::{
        amount_manager::AmountManager, food_manager::FoodManager,
        mass_food_manager::MassFoodManager, player_manager::PlayerManager,
        virus_manager::VirusManager,
    },
    map::{
        food::Food,
        mass_food::MassFood,
        player::{Player, PlayerUpdateData},
        point::{AsPoint, Point},
        virus::{Virus, VirusData},
    },
    send_messages::{
        AllInitData, FoodAddedMessage, GameUpdateData, KickMessage, KickedMessage, KillMessage,
        LeaderboardMessage, PlayerRespawnedMessage, RespawnedMessage, SendEvent, TransferInfo,
        VirusAddedMessage,
    },
    utils::{
        amount_queue::AmountQueue,
        consts::{Mass, TotalMass},
        id::{FoodID, MassFoodID, PlayerID, VirusID},
        quad_tree::{QuadTree, Rectangle},
        queue_message::QueueMessage,
        solana_util::transfer_sol,
        util::{
            are_colliding, check_who_ate_who, create_random_position_in_range,
            get_current_timestamp, is_visible_entity, mass_to_radius, random_in_range,
            uniform_position,
        },
    },
};

//Used to return to the player what is visible on his screen
pub struct VisibleEntities {
    // pub visible_players: Vec<PlayerInitData>,
    pub visible_foods: Vec<Food>,
    pub visible_viruses: Vec<Virus>,
    pub visible_mass_food: Vec<MassFood>,
}

const GAME_LOOP_INTERVAL: i64 = 1;
const TICKER_LOOP_FPS: f64 = 1.0 / (30.0 * 1.0);

pub struct Game {
    pub port: u16,
    pub amount_manager: Arc<Mutex<AmountManager>>,
    pub food_manager: FoodManager,
    pub virus_manager: RwLock<VirusManager>,
    pub mass_food_manager: RwLock<MassFoodManager>,
    pub player_manager: RwLock<PlayerManager>,
    pub main_room: String,
    pub io_socket: SocketIo,
    pub matchmaking_socket: Option<Client>,
    pub update_queue: Mutex<VecDeque<QueueMessage>>,
    pub amount_queue: Arc<Mutex<VecDeque<AmountQueue>>>,
}

impl Game {
    pub fn new(
        io_socket: SocketIo,
        matchmaking_socket: Option<Client>,
        amount_queue: Arc<Mutex<VecDeque<AmountQueue>>>,
    ) -> Self {
        let config = get_current_config();
        Game {
            amount_manager: Arc::new(Mutex::new(AmountManager::new())),
            port: *get_websockets_port(),
            food_manager: FoodManager::new(
                config.food_mass,
                QuadTree::new(
                    Rectangle::new(
                        0.0,
                        0.0,
                        config.game_width as f32,
                        config.game_height as f32,
                    ),
                    config.food_capacity_q as usize,
                ),
            ),
            virus_manager: RwLock::new(VirusManager::new()),
            update_queue: Mutex::new(VecDeque::new()),
            mass_food_manager: RwLock::new(MassFoodManager::new()),
            player_manager: RwLock::new(PlayerManager::new()),
            main_room: "main".to_string(),
            io_socket,
            matchmaking_socket,
            amount_queue: amount_queue,
        }
    }

    pub async fn add_player(&self, player: Arc<RwLock<Player>>) {
        self.player_manager
            .write()
            .await
            .insert_with_new_id(player)
            .await;
    }

    pub async fn remove_players(&self, players: impl Iterator<Item = &PlayerID>) {
        let mut player_manager = self.player_manager.write().await;
        for player_id in players {
            player_manager.remove_player_by_id(player_id);
        }
    }

    pub async fn respawn_player(&self, player: Arc<RwLock<Player>>) {
        // check if player is at the game...
        self.player_manager
            .write()
            .await
            .insert_if_not_in(player.clone())
            .await;

        let players_init_data = self
            .player_manager
            .read()
            .await
            .get_players_init_data()
            .await;

        let foods_init_data = self.food_manager.get_foods_init_data().await;
        let virus_init_data = self.virus_manager.read().await.get_virus_data();
        let mass_food_init_data = self
            .mass_food_manager
            .read()
            .await
            .get_mass_food_init_data();

        let mut player = player.write().await;
        let spawn_point = self.create_player_spawn_point();
        player.reset(&spawn_point, get_current_config().default_player_mass);

        // send init data
        if let Some(player_socket) = self.io_socket.get_socket(player.socket_id) {
            let _ = player_socket.emit(
                SendEvent::AllInitData,
                AllInitData {
                    players: players_init_data,
                    virus: virus_init_data,
                    mass_foods: mass_food_init_data,
                    foods: foods_init_data,
                },
            );

            let _ = player_socket.emit(SendEvent::Respawned, RespawnedMessage(spawn_point));
        }

        let _ = self.io_socket.within("main").emit(
            SendEvent::NotifyPlayerRespawn,
            PlayerRespawnedMessage(player.id),
        );
    }

    async fn kick_player(
        &self,
        player_name: Option<String>,
        player_id: PlayerID,
        player_socket_id: Sid,
    ) {
        info!("Kicking player {} - {:?}", player_id, player_name);
        let _ = self.io_socket.emit(
            SendEvent::KickPlayer,
            KickMessage {
                id: player_id,
                name: player_name.clone(),
            },
        );

        if let Some(ref match_making_socket) = self.matchmaking_socket {
            let kicked_message = KickedMessage {
                socket_id: player_socket_id,
                port: self.port,
            };
            let _ = match_making_socket
                .emit(SendEvent::PlayerKicked, kicked_message)
                .await;
        }

        let mut player_manager = self.player_manager.write().await;
        player_manager.remove_player_by_id(&player_id);
    }

    pub async fn tick_player(
        &self,
        player: &mut Player,
        config: &Config,
    ) -> Option<(HashSet<FoodID>, HashSet<MassFoodID>, HashSet<VirusID>)> {
        if player.last_heartbeat < (get_current_timestamp() - config.max_heartbeat_interval) {
            self.update_queue
                .lock()
                .await
                .push_back(QueueMessage::KickPlayer {
                    name: player.name.clone(),
                    id: player.id,
                    socket_id: player.socket_id,
                });
            return None;
        }
        player.move_cells(
            config.slow_base as f32,
            config.game_width as i32,
            config.game_height as i32,
            config.get_init_mass_log(),
        );

        let player_view = self.enumerate_what_player_sees(player).await;

        let mut cells_to_split: Vec<usize> = vec![];

        let mut eated_foods: HashMap<FoodID, &Food> = HashMap::new();
        let mut eated_mass: HashSet<MassFoodID> = HashSet::new();
        let mut eated_virus: HashSet<VirusID> = HashSet::new();

        for (i, p_cell) in player.cells.iter_mut().enumerate() {
            let mut mass_gained: Mass = 0;

            for food in player_view.visible_foods.iter() {
                if are_colliding(&p_cell.position, &food.as_point()) {
                    if eated_foods.insert(food.id, &food).is_none() {
                        mass_gained = mass_gained.saturating_add(1);
                    }
                }
            }

            for mass in player_view.visible_mass_food.iter() {
                if mass.can_be_eat_by(p_cell.mass, &p_cell.position) {
                    if eated_mass.insert(mass.id) {
                        mass_gained = mass_gained.saturating_add(mass.mass);
                    }
                }
            }

            for virus in player_view.visible_viruses.iter() {
                if virus.can_be_eat_by(p_cell.mass, &p_cell.position) {
                    if eated_virus.insert(virus.id) {
                        mass_gained = mass_gained.saturating_add(virus.mass);
                        cells_to_split.push(i);
                        break; // one at time, prevent the same cell to split more than once
                    }
                }
            }

            p_cell.add_mass(mass_gained);
        }

        // delete virus
        {
            let mut virus_manager = self.virus_manager.write().await;
            for virus_id in eated_virus.iter() {
                virus_manager.delete(*virus_id);
            }
        }

        // delete mass_food
        {
            let mut mass_food_manager = self.mass_food_manager.write().await;

            for mass_food_id in eated_mass.iter() {
                mass_food_manager.remove_food(*mass_food_id);
            }
        }

        // delete foods

        let eated_foods_id = eated_foods.keys().map(|i| *i).collect();

        self.food_manager
            .delete_many_foods(eated_foods.into_values())
            .await;

        if cells_to_split.len() > 0 {
            match self.io_socket.get_socket(player.socket_id) {
                Some(player_socket) => {
                    let _ = player_socket.emit(SendEvent::NotifyPlayerSplit, ());
                }
                None => {}
            };

            player.virus_split(
                &cells_to_split,
                config.limit_split as usize,
                config.default_player_mass,
            );
        }

        player.recalculate_total_mass();
        player.recalculate_ratio();

        Some((eated_foods_id, eated_mass, eated_virus))
    }

    pub fn create_player_spawn_point(&self) -> Point {
        let config = get_current_config();
        create_random_position_in_range(
            config.game_width as f32 - mass_to_radius(config.default_player_mass),
            config.game_height as f32 - mass_to_radius(config.default_player_mass),
        )
    }

    // returns the shoot direction if the virus "exploded"
    pub async fn tick_virus(
        &self,
        virus: &mut Virus,
    ) -> (Vec<MassFoodID>, Option<VirusData>, Option<(Point, Point)>) {
        let mut virus_updated: bool = false;

        // move virus if virus.speed > 0
        if virus.speed.unwrap_or_default() > 0.0 {
            virus.move_virus(
                get_current_config().game_width as f32,
                get_current_config().game_height as f32,
            );

            virus_updated = true;
        }

        let virus_point = virus.get_position();
        let mut mass_food_eated: Vec<MassFoodID> = vec![];
        let mut mass_gained: Mass = 0;

        let mut player_direction: Option<Point> = None;

        // get mass eated by the virus
        {
            let mass_food_manager = self.mass_food_manager.read().await;
            for mass_food in mass_food_manager.data.iter() {
                if are_colliding(&mass_food.point, &virus_point) {
                    mass_food_eated.push(mass_food.id);
                    mass_gained = mass_gained.saturating_add(mass_food.mass);
                    virus_updated = true;

                    if player_direction.is_none() {
                        player_direction = Some(mass_food.direction.clone());
                    }
                }
            }
        }

        let mut shoot_direciton = None;
        if mass_food_eated.len() > 0 {
            // add mass eated
            virus.add_mass(mass_gained);

            {
                let mut mass_food_manager = self.mass_food_manager.write().await;

                // remove mass eated
                for mass_food_id in mass_food_eated.iter() {
                    mass_food_manager.remove_food(*mass_food_id)
                }
            }

            // shoot new virus if virus.mass > 320
            if virus.mass > 320 {
                let virus_config = &get_current_config().virus;
                virus.set_mass(random_in_range(
                    virus_config.default_mass.from..virus_config.default_mass.to,
                ));

                shoot_direciton = Some((virus.get_position(), player_direction.unwrap()));
            }
        }

        if virus_updated {
            return (
                mass_food_eated,
                Some(virus.generate_data()),
                shoot_direciton,
            );
        }

        (mass_food_eated, None, None)
    }

    pub async fn game_loop(&self, config: &Config, players_manager: &PlayerManager) {
        self.balance_mass(config.game_mass, config.max_food, config.max_virus)
            .await;

        if players_manager.players.len() > 0 {
            let leaderboard = players_manager.get_top_players().await;
            let _ = self
                .io_socket
                .emit(SendEvent::Leaderboard, LeaderboardMessage { leaderboard });
            players_manager
                .shrink_cells(
                    config.mass_loss_rate,
                    config.default_player_mass,
                    config.min_mass_loss,
                )
                .await;
        }
    }

    // returns a list of (player_who_eat, player_eated) - (id, cell_index)
    pub async fn get_players_collision(
        players_manager: &PlayerManager,
    ) -> Vec<((PlayerID, usize), (PlayerID, usize))> {
        let mut who_ate_who_list: Vec<((_, _), (_, _))> = vec![];

        // handling collision btw players
        let players: Vec<_> = players_manager.players.values().collect();
        for player_a_index in 0..players.len() {
            for player_b_index in player_a_index + 1..players.len() {
                let player_a = players.get(player_a_index).unwrap().read().await;
                let player_b = players.get(player_b_index).unwrap().read().await;

                for (cell_a_index, cell_a) in player_a.cells.iter().enumerate() {
                    for (cell_b_index, cell_b) in player_b.cells.iter().enumerate() {
                        // 0: nothing happened
                        // 1: A ate B
                        // 2: B ate A
                        match check_who_ate_who(cell_a, cell_b) {
                            1 => who_ate_who_list
                                .push(((player_a.id, cell_a_index), (player_b.id, cell_b_index))),
                            2 => who_ate_who_list
                                .push(((player_b.id, cell_b_index), (player_a.id, cell_a_index))),
                            _ => {}
                        }
                    }
                }
            }
        }

        who_ate_who_list
    }

    //Trying with gpt less amount of lock from the amount_queue
    pub async fn handle_amount_queue(&self) {
        let messages = {
            let mut queue = self.amount_queue.lock().await;
            let msgs = queue.drain(..).collect::<Vec<_>>();
            drop(queue); // Explicitly drop the lock early
            msgs
        };

        let mut manager = self.amount_manager.lock().await;
        for message in messages {
            if let AmountQueue::AddAmount { id, amount, uid } = message {
                let players_manager = self.player_manager.read().await;
                players_manager.set_bet(uid, amount).await;
                manager.set_user_id(uid, id);
            }
        }
    }

    pub async fn handle_queue(&self) {
        let mut queue = self.update_queue.lock().await;
        loop {
            match queue.pop_front() {
                Some(message) => match message {
                    QueueMessage::KickPlayer {
                        name,
                        id,
                        socket_id,
                    } => {
                        self.kick_player(name, id, socket_id).await;
                    }
                },
                None => {
                    break;
                }
            }
        }
    }

    // equivalent to tick_game in node.js backend
    pub async fn tick_game(&self) {
        let mut last_game_loop: i64 = 0;
        let config = get_current_config();

        let instant = Instant::now();
        let mut start: Duration;

        info!("Game tick started!");
        loop {
            start = instant.elapsed();
            // let elapsed_handle_queue = instant.elapsed() - start;
            self.handle_queue().await;
            self.handle_amount_queue().await;
            let players_manager = self.player_manager.read().await;
            if (get_current_timestamp() - last_game_loop) >= GAME_LOOP_INTERVAL {
                last_game_loop = get_current_timestamp();
                self.game_loop(&config, &players_manager).await;
            }
            // let elapsed_game_loop = instant.elapsed() - start;

            let mut players_update_data: Vec<PlayerUpdateData> = vec![];
            let mut virus_update_data: Vec<VirusData> = vec![];
            let mut removed_foods: Vec<FoodID> = vec![];
            let mut removed_mass: Vec<MassFoodID> = vec![];
            let mut removed_virus: Vec<VirusID> = vec![];

            // execute the mass_move at the MassFoodManager
            let mass_food_updates = self
                .mass_food_manager
                .write()
                .await
                .move_food(config.game_width as f32, config.game_height as f32);

            // let elapsed_mass_move = instant.elapsed() - start;
            // execute tick_virus for each virus
            let mut shoot_virus: Vec<(Point, Point)> = vec![];

            {
                let mut virus_manager = self.virus_manager.write().await;

                for virus in virus_manager.data.iter_mut() {
                    let (food_mass_eated, virus_data, shoot_points) = self.tick_virus(virus).await;

                    removed_mass.extend(food_mass_eated);

                    if let Some(shoot_points) = shoot_points {
                        shoot_virus.push(shoot_points);
                    }

                    if let Some(virus_data) = virus_data {
                        virus_update_data.push(virus_data);
                    }
                }

                let mut new_viruses = vec![];

                for (position, direction) in shoot_virus.into_iter() {
                    let new_virus_data = virus_manager.shoot_one(position, direction);

                    virus_update_data.push(new_virus_data.clone());
                    new_viruses.push(new_virus_data);
                }

                if !new_viruses.is_empty() {
                    let _ = self.io_socket.emit(
                        SendEvent::VirusAdded,
                        VirusAddedMessage {
                            viruses: new_viruses,
                        },
                    );
                }
            }

            // let elapsed_virus_tick = instant.elapsed() - start;

            // handling collision btw players
            let who_ate_who_list = Self::get_players_collision(&players_manager).await;
            let mut players_who_died: Vec<PlayerID> = vec![];
            for ((player_who_eat, cell_who_eat), (player_eated, cell_eated)) in
                who_ate_who_list.into_iter()
            {
                let mut player_who_eat = match players_manager.players.get(&player_who_eat) {
                    Some(player) => player.write().await,
                    None => continue,
                };

                let mut player_eated = match players_manager.players.get(&player_eated) {
                    Some(player) => player.write().await,
                    None => continue,
                };

                info!(
                    "Player [{:?} {:?}] eat Player [{:?} {:?}]",
                    player_who_eat.id, player_who_eat.name, player_eated.id, player_eated.name
                );

                let cell_eated_mass = match player_eated.cells.get(cell_eated) {
                    Some(cell_eated) => cell_eated.mass,
                    None => continue,
                };

                // add mass to the player cell who eated
                match player_who_eat.cells.get_mut(cell_who_eat) {
                    Some(cell_who_eat) => cell_who_eat.add_mass(cell_eated_mass),
                    None => continue,
                };

                // remove cell from the player who got eaten
                player_eated.cells.remove(cell_eated);

                // check if player died
                if player_eated.player_is_dead() {
                    // player eated socket emit 'RIP'
                    match self.io_socket.get_socket(player_eated.socket_id) {
                        Some(s) => {
                            let _ = s.emit(SendEvent::RIP, ());
                        }
                        None => {
                            continue;
                        }
                    };

                    // io emit 'playerDied' with name of who died, and who killed
                    let _ = self.io_socket.emit(
                        SendEvent::PlayerDied,
                        KillMessage {
                            killed: player_eated.id,
                            eater: player_who_eat.id,
                        },
                    );

                    let mut manager = self.amount_manager.lock().await;

                    let eaten_id = manager.get_user_id(player_eated.id).unwrap_or_default();
                    let eater_id = manager.get_user_id(player_who_eat.id).unwrap_or_default();

                    drop(manager);
                    info!("User ids: {} {}", eaten_id, eater_id);

                    let transfer_amount = player_eated.bet.min(player_who_eat.bet);

                    //Adding eaten sol amount to eater
                    player_who_eat.total_won += transfer_amount;

                    if player_who_eat.bet < player_eated.bet {
                        player_eated.total_won += (player_eated.bet - transfer_amount);
                        // Reduce eaten sol amount
                    }
                    //Transferring balance to eaten
                    if player_eated.total_won > 0 {
                        let transfer_info = TransferInfo {
                            id: eaten_id,
                            amount: player_eated.total_won,
                            port: self.port,
                        };
                        if let Some(ref match_making_socket) = self.matchmaking_socket {
                            // Emit and await the result
                            match match_making_socket
                                .emit(SendEvent::TransferSol, transfer_info)
                                .await
                            {
                                Ok(_) => {
                                    // If emit is successful, proceed to clear the data
                                    player_eated.bet = 0;
                                    player_eated.total_won = 0;
                                }
                                Err(e) => {
                                    // Log the error or handle it appropriately
                                    eprintln!("Failed to send TransferSol event: {:?}", e);
                                }
                            }
                        } else {
                            // Optionally handle the case where there is no matchmaking socket
                            eprintln!("No matchmaking socket available");
                        }
                    }

                    info!("Player [{:?}] was killed !", player_eated.name);

                    // remove player from player_manager
                    players_who_died.push(player_eated.id);
                }
            }
            // let elapsed_killing_players_tick = instant.elapsed() - start;
            for (player_id, player) in players_manager.players.iter() {
                if players_who_died.contains(player_id) {
                    continue;
                }
                let mut player = player.write().await;
                match self.tick_player(&mut player, &config).await {
                    Some((player_eat_foods, player_eat_mass, player_eat_virus)) => {
                        removed_foods.extend(player_eat_foods);
                        removed_mass.extend(player_eat_mass);
                        removed_virus.extend(player_eat_virus);
                        players_update_data.push(player.generate_update_player_data());
                    }
                    None => {}
                }
            }
            drop(players_manager);
            self.remove_players(players_who_died.iter()).await;

            // send chunk data to all players
            let game_data = GameUpdateData {
                players: players_update_data,
                virus: virus_update_data,
                mass_food: mass_food_updates,
                removed_foods,
                removed_mass,
                removed_virus,
            };

            let _ = self.io_socket.emit(SendEvent::GameUpdate, game_data);

            // let elapsed_sent_game_update = instant.elapsed() - start;

            // if elapsed_sent_game_update.as_nanos() / 100_000 >= 3 {
            //     debug!("elaped_handle_queue: {}\nelaped_game_loop: {}\n elaped_mass_move: {}\nelaped_virus_tick: {}\nelaped_killing_players_tick: {}\nelaped_tick_player_tick: {}\nelapsed_sent_game_update: {}",
            //     elapsed_handle_queue.as_nanos(),
            //     elapsed_game_loop.as_nanos(),
            //     elapsed_mass_move.as_nanos(),
            //     elapsed_virus_tick.as_nanos(),
            //     elapsed_killing_players_tick.as_nanos(),
            //     elapsed_tick_player_tick.as_nanos(),
            //     elapsed_sent_game_update.as_nanos()
            // );
            // }

            let sleep_for = Duration::from_secs_f64(
                (TICKER_LOOP_FPS - ((instant.elapsed() - start).as_secs_f64())).max(0.0),
            );

            let _ = sleep(sleep_for).await;
        }
    }

    pub async fn get_food_in_view(&self, player: &Player) -> Vec<Food> {
        // Assuming get_visible_area is a function that returns a Rectangle or similar structure
        let visible_zone = player.get_visible_area(); // This line calls the function and stores its result in visible_area

        let mut found_foods: Vec<Food> = Vec::new();

        // Now use the visible_area variable that you've defined above
        self.food_manager
            .quad_tree
            .read()
            .await
            .retrieve(&visible_zone, &mut found_foods);

        found_foods
    }

    pub async fn balance_mass(&self, game_mass: TotalMass, max_food: usize, max_virus: usize) {
        // Calculate the total mass based on food and player mass
        let food_count = self.food_manager.get_food_count();
        let mut total_mass: TotalMass = food_count * (get_current_config().food_mass as TotalMass);

        total_mass += self.player_manager.read().await.get_total_mass().await;

        let mass_diff = game_mass - total_mass;

        // Calculate the amount of food that can be added based on available capacity and needed mass
        let food_free_capacity = max_food - food_count;
        let food_diff = mass_diff / (get_current_config().food_mass as TotalMass);
        let food_to_add = food_diff.min(food_free_capacity);

        // Add food if there is a need
        if food_to_add > 0 {
            let new_foods_data = self.food_manager.create_many_foods(food_to_add).await;

            let _ = self.io_socket.emit(
                SendEvent::FoodsAdded,
                FoodAddedMessage {
                    foods: new_foods_data,
                },
            );
        }

        let mut virus_manager = self.virus_manager.write().await;
        let viruses_to_add = max_virus
            .checked_sub(virus_manager.count())
            .unwrap_or_default();

        if viruses_to_add > 0 {
            let new_virus_data = virus_manager.create_many_virus(viruses_to_add);

            let _ = self.io_socket.emit(
                SendEvent::VirusAdded,
                VirusAddedMessage {
                    viruses: new_virus_data,
                },
            );
        }
    }

    pub async fn enumerate_what_player_sees(&self, player: &Player) -> VisibleEntities {
        let visible_food = self.get_food_in_view(player).await;

        // Get visible viruses
        let visible_viruses = self
            .virus_manager
            .read()
            .await
            .data
            .iter()
            .filter_map(|virus| {
                if is_visible_entity(virus.get_position(), player) {
                    return Some(virus.clone());
                }
                None
            })
            .collect();

        // Get visible mass food
        let visible_mass_food = self
            .mass_food_manager
            .read()
            .await
            .data
            .iter()
            .filter_map(|mass| {
                if is_visible_entity(mass.point, player) {
                    return Some(mass.clone());
                }
                None
            })
            .collect();

        // Get visible players
        // let mut visible_players: Vec<PlayerInitData> = vec![];
        // for (p_id, p) in self.player_manager.read().await.players.iter() {
        //     if p_id == &player.id {
        //         continue;
        //     }

        //     let p = p.read().await;
        //     if p.cells
        //         .iter()
        //         .any(|cell| is_visible_entity(cell.position, &player))
        //     {
        //         visible_players.push(p.generate_init_player_data());
        //     }
        // }

        VisibleEntities {
            // visible_players,
            visible_foods: visible_food,
            visible_viruses,
            visible_mass_food,
        }
    }
}
