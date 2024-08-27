use std::collections::HashMap;

use log::info;
//MARK: ADDED NEWLY
pub struct AmountManager {
    id_to_user_id: HashMap<u8, i64>,
    id_to_address: HashMap<i64, String>,
    user_balances: HashMap<i64, f64>,
    user_collected: HashMap<i64, Vec<f64>>,
}

impl AmountManager {
    /// Constructs a new `UserData`.
    pub fn new() -> Self {
        AmountManager {
            id_to_user_id: HashMap::new(),
            id_to_address: HashMap::new(),
            user_balances: HashMap::new(),
            user_collected: HashMap::new(),
        }
    }

    pub fn set_address(&mut self, id: i64, address: String){
        self.id_to_address.insert(id, address);
    }

    pub fn get_address(&self, id: i64) -> Option<String>{
        self.id_to_address.get(&id).cloned()
    }

    pub fn get_user_id(&self, id: u8) -> Option<i64>{
        self.id_to_user_id.get(&id).cloned()
    }

    pub fn set_user_id(&mut self, id: u8, user_id: i64) {
        self.id_to_user_id.insert(id, user_id);
    }

    /// Sets the amount for a given user ID in `user_balances`.
    pub fn set_amount(&mut self, user_id: i64, amount: f64) {
        self.user_balances.insert(user_id, amount);
        info!("Balance has been updated: {} {}", user_id, amount);
    }

    /// Gets the amount for a given user ID from `user_balances`.
    pub fn get_amount(&self, user_id: i64) -> Option<f64> {
        self.user_balances.get(&user_id).cloned()
    }

    /// Pushes a new value to the array of a given user ID in `user_data`.
    pub fn push_value(&mut self, user_id: i64, value: f64) {
        self.user_collected.entry(user_id).or_insert_with(Vec::new).push(value);
    }
  
    /// Calculates the total amount of all floats in the array for a given user ID in `user_data`.
    pub fn calculate_total(&self, user_id: i64) -> f64 {
        self.user_collected.get(&user_id).unwrap_or(&vec![]).iter().sum()
    }

    /// Clears the array for a user ID in `user_data`.
    pub fn clear_data(&mut self, user_id: i64) {
        if let Some(data) = self.user_collected.get_mut(&user_id) {
            data.clear();
        }
    }
}
