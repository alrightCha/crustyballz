use std::{
    ops::Sub,
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};

use chrono::Utc;
use socketioxide::SocketIo;
use tokio::{sync::RwLock, time::sleep};
use tracing::{debug, info};
use uuid::Uuid;

use crate::{
    config::{get_current_config, Config},
    managers::player_manager::PlayerManager,
    map::{
        food::{Food, FoodManager},
        mass_food::{MassFood, MassFoodManager},
        player::{self, Player},
        point::{AsPoint, Point},
        virus::{Virus, VirusManager},
    },
    send_messages::{
        KickMessage, LeaderboardMessage, PlayerData, ServerTellPlayerMove, UpdateData,
    },
    utils::{
        quad_tree::{QuadTree, Rectangle},
        util::{
            are_colliding, check_who_ate_who, get_current_timestamp, is_visible_entity,
            random_in_range,
        },
    },
};

//Used to return to the player what is visible on his screen
pub struct VisibleEntities {
    pub visible_players: Vec<PlayerData>,
    pub visible_foods: Vec<Food>,
    pub visible_viruses: Vec<Virus>,
    pub visible_mass_food: Vec<MassFood>,
}

const GAME_LOOP_INTERVAL: i64 = 1 * 60;
const TICKER_LOOP_FPS: f32 = 60.0;

pub struct Game {
    pub food_manager: FoodManager,
    pub virus_manager: RwLock<VirusManager>,
    pub mass_food_manager: RwLock<MassFoodManager>,
    pub player_manager: RwLock<PlayerManager>,
    pub food_count: AtomicUsize,
    pub main_room: String,
    pub io_socket: SocketIo,
}

impl Game {
    pub fn new(io_socket: SocketIo) -> Self {
        let config = get_current_config();
        Game {
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
            mass_food_manager: RwLock::new(MassFoodManager::new()),
            player_manager: RwLock::new(PlayerManager::new()),
            food_count: AtomicUsize::new(0),
            main_room: "main".to_string(),
            io_socket,
        }
    }

    fn get_food_count(&self) -> usize {
        self.food_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn set_food_count(&self, new_count: usize) {
        self.food_count
            .store(new_count, std::sync::atomic::Ordering::Relaxed);
    }

    pub async fn add_player(&self, player: Arc<RwLock<Player>>) {
        let player_id = player.read().await.id;
        self.player_manager
            .write()
            .await
            .push_new(player_id, player);
    }

    pub async fn remove_player(&self, player_id: Uuid) {
        self.player_manager
            .write()
            .await
            .remove_player_by_id(player_id);
    }

    pub async fn tick_player(&self, player: &mut Player) {
        let config = get_current_config();

        if player.last_heartbeat < (get_current_timestamp() - config.max_heartbeat_interval) {
            let _ = self.io_socket.emit(
                "kick",
                KickMessage {
                    id: player.id,
                    name: player.name.clone().unwrap_or_default(),
                },
            );

            // remove_player

            // mtchmkng_socket.emit("kicked", KickedMessage {
            // });
        }

        player.move_cells(
            config.slow_base as f32,
            config.game_width as i32,
            config.game_height as i32,
            config.get_init_mass_log(),
        );

        // let cells_to_split = Vec::new();

        let mut player_view = self.enumerate_what_player_sees(player).await;

        let mut cells_to_split: Vec<usize> = vec![];

        for (i, p_cell) in player.cells.iter_mut().enumerate() {
            let eaten_food: Vec<&Food> = player_view
                .visible_foods
                .iter()
                .filter(|food| are_colliding(&p_cell.position, &food.as_point()))
                .collect();

            let eaten_mass_food: Vec<&MassFood> = player_view
                .visible_mass_food
                .iter()
                .filter(|mass| mass.can_be_eat_by(&player.id, p_cell.mass, p_cell.position))
                .collect();

            let eaten_virus: Vec<&Virus> = player_view
                .visible_viruses
                .iter()
                .filter(|virus| virus.can_be_eat_by(p_cell.mass, p_cell.position))
                .collect();

            let mut virus_manager = self.virus_manager.write().await;
            for virus in eaten_virus {
                p_cell.add_mass(virus.mass);
                virus_manager.delete(virus.id);
                cells_to_split.push(i);
            }
            drop(virus_manager);

            let mass_gained_with_food: usize = eaten_food.len();

            // Update the ammount of food in the map
            self.set_food_count(self.get_food_count() - mass_gained_with_food);

            let mass_gained_with_mass_food: f32 = eaten_mass_food.iter().map(|f| f.mass).sum();

            let mut mass_food_manager = self.mass_food_manager.write().await;

            for mass_food in eaten_mass_food {
                mass_food_manager.remove_food(mass_food.id);
            }
            drop(mass_food_manager);

            p_cell.add_mass((mass_gained_with_food as f32 + mass_gained_with_mass_food) * 10.0);

            // TODO: delete food
            self.food_manager.delete_many_foods(eaten_food).await;
            // TODO: change leaderboard
        }

        player.virus_split(
            &cells_to_split,
            config.limit_split as usize,
            config.default_player_mass,
        );

        player.recalculate_total_mass();
        player.recalculate_ratio();

        player_view
            .visible_players
            .push(player.generate_player_data());

        match self.io_socket.get_socket(player.socket_id) {
            Some(l) => {
                let _ = l.emit(
                    "serverTellPlayerMove",
                    ServerTellPlayerMove {
                        playerData: player.generate_player_data(),
                        updates: UpdateData {
                            visiblePlayers: player_view.visible_players,
                            visibleFood: player_view.visible_foods,
                            visibleMass: player_view.visible_mass_food,
                            visibleViruses: player_view.visible_viruses,
                        },
                    },
                );
            }
            None => {}
        }
    }

    pub async fn tick_virus(&self, virus: &mut Virus) -> Option<(Point, Point)> {
        let mass_food_manager = self.mass_food_manager.read().await;

        if virus.speed.is_some() {
            virus.move_virus(
                get_current_config().game_width as f32,
                get_current_config().game_height as f32,
            )
        }

        let virus_point = virus.get_position();
        let mut eaten_mass_food = vec![];
        let mut mass_gained: f32 = 0.0;

        let mut player_direction: Option<Point> = None;

        for mass_food in mass_food_manager.data.iter() {
            if are_colliding(&mass_food.point, &virus_point) {
                eaten_mass_food.push(mass_food.id);
                mass_gained += mass_food.mass;

                if player_direction.is_none() {
                    player_direction = Some(mass_food.direction.clone());
                }
            }
        }

        if eaten_mass_food.len() <= 0 {
            return None;
        }

        virus.add_mass(mass_gained);

        drop(mass_food_manager);
        let mut mass_food_manager = self.mass_food_manager.write().await;

        for mass_food_id in eaten_mass_food {
            mass_food_manager.remove_food(mass_food_id)
        }

        if virus.mass > 320.0 {
            let virus_config = &get_current_config().virus;
            virus.set_mass(random_in_range(
                virus_config.default_mass.from,
                virus_config.default_mass.to,
            ));
            return Some((virus.get_position(), player_direction.unwrap()));
        }
        None
    }

    // equivalent to tick_game in node.js backend
    pub async fn tick_game(&self) {
        let mut last_game_loop: i64 = 0;
        let config = get_current_config();

        loop {
            sleep(Duration::from_secs_f32(1.0 / TICKER_LOOP_FPS)).await;

            let players_manager = self.player_manager.read().await;

            if (get_current_timestamp() - last_game_loop) >= GAME_LOOP_INTERVAL {
                last_game_loop = get_current_timestamp();

                self.balance_mass(
                    config.game_mass as f32,
                    config.max_food as usize,
                    config.max_virus as usize,
                )
                .await;
                // TODO: calculate leaderboard
                let leaderboard = players_manager.get_top_players().await;
                let _ = self
                    .io_socket
                    .emit("leaderboard", LeaderboardMessage { leaderboard });
                // TODO: shrink cells
            }

            // execute tick_player for each player
            let mut players = vec![];
            for player in players_manager.players.values() {
                let mut player = player.write().await;
                self.tick_player(&mut player).await;
                players.push(player)
            }

            let mut who_ate_who_list = vec![];

            for player_a_index in 0..players.len() {
                for player_b_index in player_a_index + 1..players.len() {
                    let player_a = players.get(player_a_index).unwrap();
                    let player_b = players.get(player_b_index).unwrap();

                    for (cell_a_index, cell_a) in player_a.cells.iter().enumerate() {
                        for (cell_b_index, cell_b) in player_b.cells.iter().enumerate() {
                            // 0: nothing happened
                            // 1: A ate B
                            // 2: B ate A
                            match check_who_ate_who(cell_a, cell_b) {
                                1 => who_ate_who_list.push((
                                    (player_a.id, cell_a_index),
                                    (player_b.id, cell_b_index),
                                )),
                                2 => who_ate_who_list.push((
                                    (player_b.id, cell_b_index),
                                    (player_a.id, cell_a_index),
                                )),
                                _ => {}
                            }
                        }
                    }
                }
            }

            // handle collsion
            // player eater
            // player got eaten
            // [x] remove cell from the player got eaten
            // [x] add mass to the player cell who eated
            // check if player died
            //      player socket emit 'RIP'
            //      io emit 'playerDied' with name of who died, and who killed
            //      remove player from player_manager

            drop(players);
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

                let cell_eated_mass = match player_eated.cells.get(cell_eated) {
                    Some(cell_eated) => cell_eated.mass,
                    None => continue,
                };

                match player_who_eat.cells.get_mut(cell_who_eat) {
                    Some(cell_who_eat) => cell_who_eat.add_mass(cell_eated_mass),
                    None => continue,
                };

                player_eated.cells.remove(cell_eated);
                // check if player died
                //      player socket emit 'RIP'
                //      io emit 'playerDied' with name of who died, and who killed
                //      remove player from player_manager
            }

            // execute the mass_move at the MassFoodManager
            self.mass_food_manager
                .write()
                .await
                .move_food(config.game_width as f32, config.game_height as f32);

            // execute tick_virus for each virus
            let mut virus_manager = self.virus_manager.write().await;
            let mut shoot_virus: Vec<(Point, Point)> = vec![];
            for virus in virus_manager.data.iter_mut() {
                match self.tick_virus(virus).await {
                    Some(shoot_points) => {
                        shoot_virus.push(shoot_points);
                    }
                    _ => {}
                }
            }

            for (position, direction) in shoot_virus.into_iter() {
                info!("Shoot from pos {:?} to direction {:?}", position, direction);
                virus_manager.shoot_one(position, direction);
            }
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

    pub async fn balance_mass(&self, game_mass: f32, max_food: usize, max_virus: usize) {
        // Calculate the total mass based on food and player mass
        let food_count = self.get_food_count();
        let mut total_mass = food_count as f32 * get_current_config().food_mass;

        total_mass += self.player_manager.read().await.get_total_mass().await;

        let mass_diff = game_mass - total_mass;

        // Calculate the amount of food that can be added based on available capacity and needed mass
        let food_free_capacity = max_food - food_count;
        let food_diff = mass_diff / get_current_config().food_mass;
        // let food_to_add = food_diff.floor().min(food_free_capacity as f32) as usize;
        let food_to_add = food_free_capacity;

        // Add food if there is a need
        if food_to_add > 0 {
            info!("Adding {} food's", food_to_add);
            self.food_manager.create_many_foods(food_to_add).await;
            self.set_food_count(food_count + food_to_add);
        }

        let mut virus_manager = self.virus_manager.write().await;
        let viruses_to_add = max_virus
            .checked_sub(virus_manager.count())
            .unwrap_or_default();
        if viruses_to_add > 0 {
            virus_manager.add_new(viruses_to_add);
        }
    }

    pub async fn enumerate_what_player_sees(&self, player: &Player) -> VisibleEntities {
        let mut visible_food = self.get_food_in_view(player).await;

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
        let mut visible_players: Vec<PlayerData> = vec![];
        for (p_id, p) in self.player_manager.read().await.players.iter() {
            if p_id == &player.id {
                continue;
            }

            let p = p.read().await;
            if p.cells
                .iter()
                .any(|cell| is_visible_entity(cell.position, &player))
            {
                visible_players.push(p.generate_player_data());
            }
        }

        VisibleEntities {
            visible_players,
            visible_foods: visible_food,
            visible_viruses,
            visible_mass_food,
        }
    }
}
