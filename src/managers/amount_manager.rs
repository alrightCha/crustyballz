use std::collections::HashMap;

use log::info;
//MARK: ADDED NEWLY
pub struct AmountManager {
    id_to_user_id: HashMap<u8, i64>,
}

impl AmountManager {
    /// Constructs a new `UserData`.
    pub fn new() -> Self {
        AmountManager {
            id_to_user_id: HashMap::new(),
        }
    }

    pub fn get_user_id(&self, id: u8) -> Option<i64> {
        self.id_to_user_id.get(&id).cloned()
    }

    pub fn set_user_id(&mut self, id: u8, user_id: i64) {
        info!("setting player id {id} to matchmaking id {user_id}");
        self.id_to_user_id.insert(id, user_id);
    }
}
