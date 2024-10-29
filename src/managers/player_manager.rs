use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::{
    map::{
        player::{Player, PlayerInitData},
        point::Point,
    },
    send_messages::LeaderboardPlayer,
    utils::{
        consts::{Mass, TotalMass},
        id::PlayerID,
    },
};

pub struct PlayerManager {
    pub players: HashMap<PlayerID, Arc<RwLock<Player>>>,
    id_counter: PlayerID,
}

impl PlayerManager {
    pub fn new() -> Self {
        PlayerManager {
            players: HashMap::new(),
            id_counter: PlayerID::MAX,
        }
    }

    pub async fn collect_and_clone_all_pos(&self) -> Vec<Point> {
        let mut all_pos = Vec::new();
        if self.players.is_empty() {
            return all_pos;
        }
        for player in self.players.values() {
            let player = player.read().await; // Acquire read lock asynchronously
            for cell in &player.cells {
                // Assuming `cells` is accessible and is a Vec<Cell>
                all_pos.push(cell.position.clone()); // Clone each cell's position and push it to the vector
            }
        }
        all_pos
    }

    pub fn get_new_id(&mut self) -> PlayerID {
        loop {
            self.id_counter = self.id_counter.wrapping_add(1);
            if self.players.contains_key(&self.id_counter) {
                continue;
            }
            return self.id_counter;
        }
    }

    pub async fn insert_if_not_in(&mut self, player: Arc<RwLock<Player>>) -> bool {
        let player_id = player.read().await.id;

        if self.players.contains_key(&player_id) {
            return false;
        }

        self.players.insert(player_id, player);
        true
    }

    pub async fn insert_with_new_id(&mut self, player: Arc<RwLock<Player>>) -> PlayerID {
        let player_id = self.get_new_id();
        player.write().await.id = player_id;
        // TODO: check limit
        self.players.insert(player_id, player);
        
        player_id
    }

    // fn async find_index_by_id(&self, id: PlayerID) -> Option<usize> {
    //     for player in self.players {
    //         return
    //         player.read().await.id == id
    //     }
    //     self.players.iter().position(|p| p.id == id)
    // }

    pub fn remove_player_by_id(&mut self, id: &PlayerID) {
        self.players.remove(&id);
    }

    pub async fn shrink_cells(
        &self,
        mass_loss_rate: f32,
        default_player_mass: Mass,
        min_mass_loss: Mass,
    ) {
        for player in self.players.values() {
            let mut player = player.write().await;
            player.lose_mass_if_needed(mass_loss_rate, default_player_mass, min_mass_loss);
        }
    }

    pub async fn get_players_init_data(&self) -> Vec<PlayerInitData> {
        let mut players_data = vec![];
        for player in self.players.values() {
            players_data.push(player.read().await.generate_init_player_data());
        }
        players_data
    }

    pub async fn get_top_players(&self) -> Vec<LeaderboardPlayer> {
        // First, clone the players to a mutable local variable to sort

        let mut players = vec![];
        for player in self.players.values() {
            let player = player.read().await;
            players.push(LeaderboardPlayer {
                id: player.id,
                mass: player.total_mass,
            });
        }

        players.sort_by(|a, b| {
            b.mass
                .partial_cmp(&a.mass)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        players.into_iter().take(10).collect()
    }

    pub async fn get_total_mass(&self) -> TotalMass {
        let mut sum: TotalMass = 0;
        for player in self.players.values() {
            sum = sum.saturating_add(player.read().await.total_mass as TotalMass);
        }
        sum
    }

    pub async fn set_bet(&self, id: PlayerID, bet: u64) -> Option<()> {
        if let Some(player) = self.players.get(&id) {
            let mut player = player.write().await;
            // Perform modifications
            player.bet = bet;
            player.bet_set = true;
            return Some(());
        }
        None
    }
}
