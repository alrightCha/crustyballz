use serde::Serialize;
use socketioxide::socket::Sid;
use uuid::Uuid;

use crate::{map::{food::Food, mass_food::MassFood, player::Cell, virus::Virus}, recv_messages::Target};


#[derive(Serialize)]
pub struct ServerTellPlayerMove {
    pub playerData : PlayerData,
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
    pub id: Uuid
}
#[derive(Serialize)]
pub struct KickedMessage {
    pub socketId: Sid,
    pub port: u16
}