use std::{
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
        player::Player,
        point::{AsPoint, Point},
        virus::{Virus, VirusManager},
    },
    send_messages::{KickMessage, PlayerData, ServerTellPlayerMove, UpdateData},
    utils::{
        quad_tree::{QuadTree, Rectangle},
        util::{are_colliding, get_current_timestamp, is_visible_entity},
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
    pub virus_manager: VirusManager,
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
            virus_manager: VirusManager::new(),
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

        for p_cell in player.cells.iter_mut() {
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

            for virus in eaten_virus {
                p_cell.add_mass(virus.mass);
                player.mass_total += virus.mass;
                // TODO: delete virus
                // TODO: split cell
            }

            // TODO: delete mass fodd

            let mass_gained_with_food: f32 = eaten_food.len() as f32;
            let mass_gained_with_mass_food: f32 = eaten_mass_food.iter().map(|f| f.mass).sum();

            let mut mass_food_manager = self.mass_food_manager.write().await;

            for mass_food in eaten_mass_food {
                mass_food_manager.remove_food(mass_food.id);
            }
            drop(mass_food_manager);

            p_cell.add_mass(mass_gained_with_food + mass_gained_with_mass_food);
            player.mass_total += mass_gained_with_food + mass_gained_with_mass_food;

            // TODO: delete food
            self.food_manager.delete_many_foods(eaten_food).await;
            // TODO: change leaderboard
        }

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

    // equivalent to tick_game in node.js backend
    pub async fn tick_game(&self) {
        let mut last_game_loop: i64 = 0;
        let config = get_current_config();

        loop {
            sleep(Duration::from_secs_f32(1.0 / TICKER_LOOP_FPS)).await;

            if (get_current_timestamp() - last_game_loop) >= GAME_LOOP_INTERVAL {
                last_game_loop = get_current_timestamp();

                self.balance_mass(
                    config.game_mass as f32,
                    config.max_food as usize,
                    config.max_virus as usize,
                )
                .await;
                // TODO: calculate leaderboard
            }

            // execute tick_player for each player
            let players_manager = self.player_manager.read().await;

            for player in players_manager.players.values() {
                let mut player = player.write().await;

                self.tick_player(&mut player).await;
            }

            // execute the mass_move at the MassFoodManager
            self.mass_food_manager
                .write()
                .await
                .move_food(config.game_width as f32, config.game_height as f32);

            // execute tick_virus for each virus

            // execute collision check
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

        let viruses_to_add = max_virus - self.virus_manager.count().await;
        if viruses_to_add > 0 {
            self.virus_manager.add_new(viruses_to_add).await;
        }
    }

    pub async fn enumerate_what_player_sees(&self, player: &Player) -> VisibleEntities {
        let mut visible_food = self.get_food_in_view(player).await;

        // Get visible viruses
        let visible_viruses = self
            .virus_manager
            .data
            .read()
            .await
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
