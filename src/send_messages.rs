use std::{borrow::Cow, fmt::Display};

use rust_socketio::{Event, Payload};
use serde::{Deserialize, Serialize};
use serde_json::json;
use socketioxide::socket::Sid;
use uuid::Uuid;

use crate::{
    map::{cell::Cell, food::Food, mass_food::MassFood, virus::Virus},
    recv_messages::Target,
};

pub enum SendEvent {
    TellPlayerSplit,
    RIP,
    PlayerDied,
    KickPlayer,
    PlayerKicked,
    Leaderboard,
    Respawned,
    Welcome,
    PongCheck,
    ServerPlayerChat,
    PlayerJoin,
}

impl Display for SendEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match &self {
            SendEvent::Welcome => "welcome",
            SendEvent::TellPlayerSplit => "tellPlayerSplit",
            SendEvent::RIP => "RIP",
            SendEvent::PlayerDied => "playerDied",
            SendEvent::PlayerKicked => "kicked",
            SendEvent::KickPlayer => "kick",
            SendEvent::Leaderboard => "leaderboard",
            SendEvent::PongCheck => "pongcheck",
            SendEvent::ServerPlayerChat => "serverSendPlayerChat",
            SendEvent::Respawned => "respawned",
            SendEvent::PlayerJoin => "playerJoin",
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
pub struct ServerTellPlayerMove {
    pub playerData: PlayerData,
    pub updates: UpdateData,
}

#[derive(Serialize, Default, Clone)]
pub struct UpdateData {
    pub visiblePlayers: Vec<PlayerData>,
    pub visibleFood: Vec<Food>,
    pub visibleMass: Vec<MassFood>,
    pub visibleViruses: Vec<Virus>,
}

#[derive(Serialize, Clone, Deserialize)]
pub struct PlayerData {
    pub admin: bool,
    pub cells: Vec<Cell>,
    pub hue: u16,
    pub id: Uuid,
    pub imgUrl: Option<String>,
    pub lastHeartbeat: i64,
    pub massTotal: f32,
    pub name: String,
    pub ratio: f32,
    pub screenHeight: f32,
    pub screenWidth: f32,
    pub target: Target,
    pub timeToMerge: Option<i64>,
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize)]
pub struct PlayerJoinMessage {
    pub name: Option<String>,
}

#[derive(Serialize)]
pub struct KickMessage {
    pub name: Option<String>,
    pub id: Uuid,
}
#[derive(Serialize)]
pub struct KickedMessage {
    pub socketId: Sid,
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
    pub id: Uuid,
    pub name: Option<String>,
    pub mass: f32,
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
}
