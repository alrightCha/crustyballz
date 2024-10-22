use std::{borrow::Cow, fmt::Display};

use rust_socketio::Event;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, PartialOrd, Debug)]
pub enum RecvEvent {
    Respawn,
    PingCheck,
    PlayerMousePosition,
    PlayerSendingMass,
    PlayerSplit,
    PlayerChat,
    PlayerGotIt,
    LetMeIn,
    Teleport
}

impl From<u8> for RecvEvent {
    fn from(value: u8) -> Self {
        unsafe { std::mem::transmute::<_, RecvEvent>(value) }
    }
}

impl Display for RecvEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match &self {
            RecvEvent::Respawn => "respawn",
            RecvEvent::PingCheck => "pingcheck",
            RecvEvent::LetMeIn => "let_me_in",
            RecvEvent::PlayerMousePosition => "0",
            RecvEvent::PlayerSendingMass => "1",
            RecvEvent::PlayerSplit => "2",
            RecvEvent::PlayerChat => "playerChat",
            RecvEvent::PlayerGotIt => "gotit",
            RecvEvent::Teleport => "3"
        })
    }
}

impl Into<String> for RecvEvent {
    fn into(self) -> String {
        self.to_string()
    }
}

impl Into<Cow<'static, str>> for RecvEvent {
    fn into(self) -> Cow<'static, str> {
        self.to_string().into()
    }
}

impl Into<Event> for RecvEvent {
    fn into(self) -> Event {
        self.to_string().into()
    }
}

#[derive(Deserialize)]
pub struct TargetMessage {
    pub target: Target,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Target {
    pub x: f32,
    pub y: f32,
}

#[derive(Deserialize)]
pub struct UserIdMessage {
    pub user_id: Option<String>
}

#[derive(Deserialize)]
pub struct LetMeInMessage {
    pub name: Option<String>,
    pub img_url: Option<String>,
    pub user_id: Option<String>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UsernameMessage {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    message: String,
    sender: String,
}


#[derive(Debug, Deserialize, Serialize)]
pub struct AmountMessage {
    pub address: String,
    pub amount: u64,
    pub id: i64,
    pub uid: u8
}