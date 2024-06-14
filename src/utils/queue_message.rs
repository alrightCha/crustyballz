use socketioxide::socket::Sid;

use super::id::PlayerID;

pub enum QueueMessage {
    KickPlayer{
        name: Option<String>,
        id: PlayerID,
        socket_id: Sid
    }
}