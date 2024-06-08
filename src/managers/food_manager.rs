use tokio::sync::RwLock;

use crate::{map::food::Food, utils::{quad_tree::QuadTree, util::{get_position, mass_to_radius}}};

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
