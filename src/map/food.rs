use super::point::{AsPoint, Point};
use crate::utils::quad_tree::QuadTree;
use crate::utils::util::{get_position, mass_to_radius};
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

pub struct FoodManager {
    food_mass: f32,
    pub quad_tree: RwLock<QuadTree>,
}

impl FoodManager {
    pub fn new(food_mass: f32, quad_tree: QuadTree) -> Self {
        FoodManager {
            food_mass,
            quad_tree: RwLock::new(quad_tree),
        }
    }

    pub async fn create_many_foods(&self, number: usize) {
        let mut quad_tree = self.quad_tree.write().await;

        let radius = mass_to_radius(self.food_mass);
        for _ in 0..number {
            let position = get_position(false, radius, None);
            let food = Food::new(position);
            quad_tree.insert(food); // Ensure QuadTree accepts Point
        }
    }

    pub async fn delete_many_foods(&self, foods_to_delete: Vec<&Food>) {
        let mut quad_tree = self.quad_tree.write().await;
        for food in foods_to_delete {
            quad_tree.remove(&food);
        }
    }
}
