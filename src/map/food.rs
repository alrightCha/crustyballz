use super::point::{AsPoint, Point};
use crate::utils::quad_tree::QuadTree;
use crate::utils::util::{create_random_position, mass_to_radius};
use rand::Rng;
use serde::Serialize;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Food {
    pub id: Uuid,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub mass: i32,
    pub hue: u16,
}

impl Food {
    pub fn new(point: Point) -> Self {
        let mut rng = rand::thread_rng();
        let mass = rng.gen_range(2..3);
        Food {
            id: Uuid::new_v4(),
            x: point.x,
            y: point.y,
            radius: mass_to_radius(mass as f32),
            mass,
            hue: rng.gen_range(0..360),
        }
    }
}

impl AsPoint for Food {
    fn as_point(&self) -> Point {
        Point {
            x: self.x,
            y: self.y,
            radius: self.radius,
        }
    }
}