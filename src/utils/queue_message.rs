use socketioxide::socket::Sid;
use uuid::Uuid;

pub enum QueueMessage {
    KickPlayer{
        name: Option<String>,
        id: Uuid,
        socket_id: Sid
    }
}