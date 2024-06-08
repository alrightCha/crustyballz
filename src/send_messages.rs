use std::{borrow::Cow, fmt::Display};

use serde::Serialize;
use socketioxide::socket::Sid;
use uuid::Uuid;

use crate::{
    map::{food::Food, mass_food::MassFood, cell::Cell, virus::Virus},
    recv_messages::Target,
};

pub enum SendEvent {
    TellPlayerSplit,
    RIP,
    PlayerDied,
    Kicked,
    Leaderboard,
    Respawned,
    Welcome,
    PongCheck,
    ServerPlayerChat
}

impl Display for SendEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match &self {
            SendEvent::Welcome => "welcome",
            SendEvent::TellPlayerSplit => "tellPlayerSplit",
            SendEvent::RIP => "RIP",
            SendEvent::PlayerDied => "playerDied",
            SendEvent::Kicked => "kicked",
            SendEvent::Leaderboard => "leaderboard",
            SendEvent::PongCheck => "pongcheck",
            SendEvent::ServerPlayerChat => "serverSendPlayerChat",
            SendEvent::Respawned => "respawned",
        }) 
    }
}

impl Into<Cow<'static, str>> for SendEvent {
    fn into(self) -> Cow<'static, str> {
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

#[derive(Serialize, Clone)]
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
pub struct KickMessage {
    pub name: String,
    pub id: Uuid,
}
#[derive(Serialize)]
pub struct KickedMessage {
    pub socketId: Sid,
    pub port: u16,
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
    pub eater: Option<String>
}