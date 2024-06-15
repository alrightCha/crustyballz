use std::{borrow::Cow, fmt::Display};

use rust_socketio::{Event, Payload};
use serde::{Deserialize, Serialize};
use serde_json::json;
use socketioxide::socket::Sid;

use crate::{
    map::{
        cell::Cell,
        food::{Food, FoodData},
        mass_food::{MassFood, MassFoodInitData, MassFoodUpdateData},
        player::{PlayerInitData, PlayerUpdateData},
        point::Point,
        virus::{Virus, VirusData},
    },
    recv_messages::Target,
    utils::{
        consts::Mass,
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
    ServerPlayerChat,
    GameUpdate,
    FoodsAdded,
    VirusAdded,
    Respawned,
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
            SendEvent::ServerPlayerChat => "serverSendPlayerChat",
            SendEvent::GameUpdate => "game_update",
            SendEvent::FoodsAdded => "foods_added",
            SendEvent::VirusAdded => "virus_added",
            SendEvent::AllInitData => "all_init_data",
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

#[derive(Serialize)]
pub struct LeaderboardMessage {
    pub leaderboard: Vec<LeaderboardPlayer>,
}

#[derive(Serialize)]
pub struct LeaderboardPlayer {
    pub id: PlayerID,
    pub mass: usize,
}

#[derive(Serialize)]
pub struct KillMessage {
    pub name: Option<String>,
    pub eater: Option<String>,
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
pub struct PlayerJoinMessage(pub PlayerInitData);

#[derive(Serialize)]
pub struct PlayerRespawnedMessage(pub PlayerID);

#[derive(Serialize)]
pub struct RespawnedMessage(pub Point);
