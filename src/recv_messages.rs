use std::{borrow::Cow, fmt::Display};

use rust_socketio::Event;
use serde::{Deserialize, Serialize};

pub enum RecvEvent {
    Respawn,
    PingCheck,
    PlayerMousePosition,
    PlayerSendingMass,
    PlayerSplit,
    PlayerChat,
    PlayerGotIt,
    LetMeIn
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
        })
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
pub struct LetMeInMessage {
    pub name: Option<String>,
    pub img_url: Option<String>,
    pub user_id: i8
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
