use std::{borrow::Cow, fmt::Display};

use rust_socketio::{Event, Payload};
use serde::Serialize;
use socketioxide::socket::Sid;
use tokio_tungstenite::tungstenite::Message;

use crate::{
    map::{
        food::FoodData,
        mass_food::{MassFoodInitData, MassFoodUpdateData},
        player::{PlayerInitData, PlayerUpdateData},
        point::Point,
        virus::VirusData,
    },
    utils::{
        consts::{Mass, TotalMass},
        id::{FoodID, MassFoodID, PlayerID, VirusID},
    },
};

pub enum SendEvent {
    Welcome,
    PlayerInitData,
    AllInitData,
    NotifyPlayerJoined,
    NotifyPlayerSplit,
    RIP,
    PlayerDied,
    KickPlayer,
    PlayerKicked,
    Leaderboard,
    NotifyPlayerRespawn,
    PongCheck,
    PlayerMessage,
    GameUpdate,
    FoodsAdded,
    VirusAdded,
    MassFoodAdded,
    Respawned,
    TransferSol
}

// Notify means that we are going to emit this message globaly
impl Display for SendEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match &self {
            SendEvent::Welcome => "welcome",
            SendEvent::PlayerInitData => "player_init_data",
            SendEvent::NotifyPlayerJoined => "player_joined",
            SendEvent::NotifyPlayerSplit => "player_splited",
            SendEvent::NotifyPlayerRespawn => "player_respawned",
            SendEvent::Respawned => "respawned",
            SendEvent::RIP => "RIP",
            SendEvent::PlayerDied => "playerDied",
            SendEvent::PlayerKicked => "kicked",
            SendEvent::KickPlayer => "kick",
            SendEvent::Leaderboard => "leaderboard",
            SendEvent::PongCheck => "pong_check",
            SendEvent::PlayerMessage => "player_message",
            SendEvent::AllInitData => "all_init_data",
            SendEvent::GameUpdate => "game_update",
            SendEvent::FoodsAdded => "foods_added",
            SendEvent::VirusAdded => "virus_added",
            SendEvent::MassFoodAdded => "mass_food_added",
            SendEvent::TransferSol => "transfer",
        })
    }
}

impl Into<Cow<'static, str>> for SendEvent {
    fn into(self) -> Cow<'static, str> {
        self.to_string().into()
    }
}

impl Into<Event> for SendEvent {
    fn into(self) -> Event {
        self.to_string().into()
    }
}
#[derive(Serialize)]
pub struct AllInitData {
    pub players: Vec<PlayerInitData>,
    pub virus: Vec<VirusData>,
    pub mass_foods: Vec<MassFoodInitData>,
    pub foods: Vec<FoodData>,
}

#[derive(Serialize)]
pub struct GameUpdateData {
    pub players: Vec<PlayerUpdateData>,
    pub virus: Vec<VirusData>,
    pub mass_food: Vec<MassFoodUpdateData>,
    pub removed_foods: Vec<FoodID>,
    pub removed_mass: Vec<MassFoodID>,
    pub removed_virus: Vec<VirusID>
}

#[derive(Serialize)]
pub struct KickMessage {
    pub name: Option<String>,
    pub id: PlayerID,
}
#[derive(Serialize)]
pub struct KickedMessage {
    pub socket_id: Sid,
    pub port: u16,
}

impl Into<Payload> for KickedMessage {
    fn into(self) -> Payload {
        serde_json::to_value(self).unwrap().into()
    }
}

impl Into<Message> for KickedMessage {
    fn into(self) -> Message {
        let json_payload = serde_json::json!({
            "event": SendEvent::PlayerKicked.to_string(),
            "data": self
        });
        
        Message::Text(serde_json::to_string(&json_payload).unwrap())
    }
}

#[derive(Serialize)]
pub struct LeaderboardMessage {
    pub leaderboard: Vec<LeaderboardPlayer>
}


#[derive(Serialize)]
pub struct LeaderboardPlayer {
    pub id: PlayerID,
    pub mass: TotalMass,
}

#[derive(Serialize)]
pub struct KillMessage {
    pub killed: PlayerID,
    pub eater: PlayerID,
}

#[derive(Serialize)]
pub struct WelcomeMessage {
    pub width: u32,
    pub height: u32,
    pub default_player_mass: Mass,
    pub default_mass_food: Mass,
    pub default_mass_mass_food: Mass
}

#[derive(Serialize)]
pub struct TransferInfo{
    pub id: i64,
    pub amount: u64
}


impl Into<Payload> for TransferInfo {
    fn into(self) -> Payload {
        serde_json::to_value(self).unwrap().into()
    }
}

impl Into<Message> for TransferInfo {
    fn into(self) -> Message {
        let json_payload = serde_json::json!({
            "event": SendEvent::TransferSol.to_string(),
            "data": self
        });
        
        Message::Text(serde_json::to_string(&json_payload).unwrap())
    }
}

#[derive(Serialize)]
pub struct PlayerJoinMessage(pub PlayerInitData);

#[derive(Serialize)]
pub struct PlayerRespawnedMessage(pub PlayerID);

#[derive(Serialize)]
pub struct RespawnedMessage(pub Point);

#[derive(Serialize)]
pub struct MassFoodAddedMessage(pub MassFoodInitData);

#[derive(Serialize)]
pub struct VirusAddedMessage {
    pub viruses: Vec<VirusData>
}

#[derive(Serialize)]
pub struct FoodAddedMessage {
    pub foods: Vec<FoodData>,
}