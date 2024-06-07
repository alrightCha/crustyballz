use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::map::player::Player;

pub struct PlayerManager {
    pub players: HashMap<Uuid, Arc<RwLock<Player>>>,
}

impl PlayerManager {
    pub fn new() -> Self {
        PlayerManager {
            players: HashMap::new(),
        }
    }

    pub fn push_new(&mut self, player_id: Uuid, player: Arc<RwLock<Player>>) {
        self.players.insert(player_id, player);
    }

    // fn async find_index_by_id(&self, id: Uuid) -> Option<usize> {
    //     for player in self.players {
    //         return
    //         player.read().await.id == id
    //     }
    //     self.players.iter().position(|p| p.id == id)
    // }

    pub fn remove_player_by_id(&mut self, id: Uuid) {
        self.players.remove(&id);
    }

    // fn shrink_cells(&mut self, mass_loss_rate: f32, default_player_mass: f32, min_mass_loss: f32) {
    //     for player in &mut self.players {
    //         player.lose_mass_if_needed(mass_loss_rate, default_player_mass, min_mass_loss);
    //     }
    // }

    // fn handle_collisions(&self, callback: &dyn Fn((usize, usize), (usize, usize))) {
    //     for (player_a_index, player_a) in self.players.iter().enumerate() {
    //         for (player_b_index, player_b) in self.players.iter().enumerate().skip(player_a_index + 1) {
    //             Player::check_for_collisions(player_a, player_b, player_a_index, player_b_index, callback);
    //         }
    //     }
    // }

    // fn get_top_players(&self) -> Vec<(Uuid, String)> {
    //     // First, clone the players to a mutable local variable to sort
    //     let mut sorted_players = self.players.clone();
    //     sorted_players.sort_by(|a, b|
    //         b.cells.iter().map(|c| c.mass).sum::<f32>()
    //             .partial_cmp(&a.cells.iter().map(|c| c.mass).sum::<f32>())
    //             .unwrap_or(std::cmp::Ordering::Equal)
    //     );

    //     // Now collect the top 10 players, safely handling Option<String>
    //     sorted_players.iter()
    //         .filter_map(|p| {
    //             Some((
    //                 p.id.clone(), // If id is None, the player will be skipped
    //                 p.name.clone()? // If name is None, the player will also be skipped
    //             ))
    //         })
    //         .take(10)
    //         .collect()
    // }

    pub async fn get_total_mass(&self) -> f32 {
        let mut sum: f32 = 0.0;
        for player in self.players.values() {
            sum += player.read().await.mass_total;
        }
        sum
    }
}
