use std::{borrow::Cow, fmt::Display};

use serde::{Deserialize, Serialize};

pub enum RecvEvent {
    Respawn,
    PingCheck,
    PlayerMousePosition,
    PlayerSendingMass,
    PlayerSplit,
    PlayerWindowResized,
    PlayerChat,
    PlayerGoit
}

impl Display for RecvEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match &self {
            RecvEvent::Respawn => "respawn",
            RecvEvent::PingCheck => "pingcheck",
            RecvEvent::PlayerMousePosition => "0",
            RecvEvent::PlayerSendingMass => "1",
            RecvEvent::PlayerSplit => "2",
            RecvEvent::PlayerWindowResized => "windowResized",
            RecvEvent::PlayerChat => "playerChat",
            RecvEvent::PlayerGoit => "gotit",
        }) 
    }
}

impl Into<Cow<'static, str>> for RecvEvent {
    fn into(self) -> Cow<'static, str> {
        self.to_string().into()
    }
}

#[derive(Deserialize)]
pub struct TargetMessage {
    pub target : Target
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Target {
    pub x: f32,
    pub y: f32
}

#[derive(Deserialize)]
pub struct WindowResizedMessage {
    pub screenHeight: i32,
    pub screenWidth: i32
}

#[derive(Deserialize)]
pub struct GotItMessage {
    pub name: Option<String>,
    pub imgUrl: Option<String>,
    pub screenHeight: i32,
    pub screenWidth: i32
}