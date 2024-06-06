use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use uuid::Uuid;

/// A struct to manage user sockets
pub struct UserSockets(RwLock<HashMap<Uuid, Arc<String>>>);

impl UserSockets {
    /// Creates a new UserSockets instance
    pub fn new() -> Self {
        Self(RwLock::new(HashMap::new()))
    }

    /// Adds a user and their socket to the map
    pub fn add_user(&self, user_id: Uuid, socket: String) {
        let socket = Arc::new(socket);
        self.0.write().unwrap().insert(user_id, socket);
    }

    /// Retrieves a user's socket by their ID
    pub fn get_user_socket(&self, user_id: Uuid) -> Option<Arc<String>> {
        self.0.read().unwrap().get(&user_id).cloned()
    }

    /// Removes a user from the map
    pub fn remove_user(&self, user_id: Uuid) {
        self.0.write().unwrap().remove(&user_id);
    }
}