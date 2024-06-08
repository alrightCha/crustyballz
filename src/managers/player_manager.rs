use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{map::player::Player, send_messages::LeaderboardPlayer};

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

    pub async fn get_top_players(&self) -> Vec<LeaderboardPlayer> {
        // First, clone the players to a mutable local variable to sort

        let mut players = vec![];
        for player in self.players.values() {
            let player = player.read().await;
            players.push(LeaderboardPlayer {
                id: player.id,
                name: player.name.clone(),
                mass: player.total_mass,
            });
        }

        players.sort_by(|a, b| b.mass.total_cmp(&a.mass));

        players.into_iter().take(10).collect()
    }

    pub async fn get_total_mass(&self) -> f32 {
        let mut sum: f32 = 0.0;
        for player in self.players.values() {
            sum += player.read().await.total_mass;
        }
        sum
    }
}
