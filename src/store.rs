use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use socketioxide::socket::Sid;
use uuid::Uuid;

/// A struct to manage user sockets
pub struct UserSockets(RwLock<HashMap<Uuid, Sid>>);

//A struct that stores a mapping of the player ids to their respective socket ids to know which id to send a message to when we 
//need to target one single user 
impl UserSockets {
    /// Creates a new UserSockets instance
    pub fn new() -> Self {
        Self(RwLock::new(HashMap::new()))
    }

    /// Adds a user and their socket to the map
    pub fn add_user(&self, user_id: Uuid, socket: Sid) {
        self.0.write().unwrap().insert(user_id, socket);
    }

    /// Retrieves a user's socket by their ID
    pub fn get_user_socket(&self, user_id: Uuid) -> Option<Sid> {
        self.0.read().unwrap().get(&user_id).cloned()
    }

    /// Removes a user from the map
    pub fn remove_user(&self, user_id: Uuid) {
        self.0.write().unwrap().remove(&user_id);
    }
}