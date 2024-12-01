use std::{borrow::Cow, fmt::Display};

use log::error;
use rust_socketio::Event;
use serde::{Deserialize, Serialize};

use crate::send_messages::SendEvent;

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
    Teleport,
    Cashout
}

impl From<u8> for RecvEvent {
    fn from(value: u8) -> Self {
        unsafe { std::mem::transmute::<_, RecvEvent>(value) }
    }
}

impl From<&str> for RecvEvent {
    fn from(value: &str) -> Self {
        match value {
            "respawn" => RecvEvent::Respawn,
            "pingcheck" => RecvEvent::PingCheck,
            "let_me_in" => RecvEvent::LetMeIn,
            "0" => RecvEvent::PlayerMousePosition,
            "1" => RecvEvent::PlayerSendingMass,
            "2" => RecvEvent::PlayerSplit,
            "playerChat" => RecvEvent::PlayerChat,
            "gotit" => RecvEvent::PlayerGotIt,
            "3" => RecvEvent::Teleport,
            event => {
                error!("RecvEvent not implement from string for: {}", event);
                todo!()
            },
            "4" => RecvEvent::Cashout
        }
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
            RecvEvent::Teleport => "3",
            RecvEvent::Cashout => "4"
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

#[derive(Debug, Serialize, Deserialize)]
pub struct AnyEventPacket {
    pub event: String,
    pub value: Option<serde_json::Value>,
}

impl AnyEventPacket {
    pub fn new<T: serde::Serialize>(send_event: SendEvent, data: T) -> AnyEventPacket {
        AnyEventPacket {
            event: send_event.to_string(),
            value: Some(serde_json::to_value(data).unwrap()),
        }
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let packet_buffer = serde_json::to_vec(&self).unwrap();

        let len_bytes = (packet_buffer.len() as u64).to_be_bytes().to_vec();

        // TODO: benchmark the best way of adding "len_bytes" at the start of packet_buffer

        // Method 1
        // packet_buffer.splice(0..0, len_bytes.iter().cloned()); 

        // Method 2
        // [len_bytes, packet_buffer].concat()

        [len_bytes, packet_buffer].concat()
    }
}

#[derive(Deserialize, Debug)]
pub struct TargetMessage {
    pub target: Target,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Target {
    pub x: f32,
    pub y: f32,
}

#[derive(Deserialize)]
pub struct UserIdMessage {
    pub user_id: Option<String>,
}

#[derive(Deserialize)]
pub struct LetMeInMessage {
    pub name: Option<String>,
    pub img_url: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UsernameMessage {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChatMessage {
    message: String,
    sender: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AmountMessage {
    pub address: String,
    pub amount: u64,
    pub id: i64,
    pub uid: u8,
}