use serde::{Deserialize, Serialize};

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