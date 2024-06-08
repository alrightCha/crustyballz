use uuid::Uuid;

pub enum QueueMessage {
    KickPlayer{
        name: Option<String>,
        id: Uuid
    }
}