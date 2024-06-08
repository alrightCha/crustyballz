use std::sync::atomic::AtomicUsize;

use tokio::sync::RwLock;

use crate::{
    map::food::Food,
    utils::{
        quad_tree::QuadTree,
        util::{get_position, mass_to_radius},
    },
};

pub struct FoodManager {
    food_mass: f32,
    pub quad_tree: RwLock<QuadTree>,
    food_count: AtomicUsize,
}

impl FoodManager {
    pub fn new(food_mass: f32, quad_tree: QuadTree) -> Self {
        FoodManager {
            food_mass,
            quad_tree: RwLock::new(quad_tree),
            food_count: AtomicUsize::new(0),
        }
    }

    pub fn get_food_count(&self) -> usize {
        self.food_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn set_food_count(&self, new_count: usize) {
        self.food_count
            .store(new_count, std::sync::atomic::Ordering::Relaxed);
    }

    fn sub_food_count(&self, sub_amount: usize) {
        self.set_food_count(
            self.get_food_count()
                .checked_sub(sub_amount)
                .unwrap_or_default(),
        );
    }

    fn add_food_count(&self, add_amount: usize) {
        self.set_food_count(self.get_food_count() + add_amount);
    }

    pub async fn create_many_foods(&self, food_amount: usize) {
        let mut quad_tree = self.quad_tree.write().await;

        let radius = mass_to_radius(self.food_mass);
        for _ in 0..food_amount {
            let position = get_position(false, radius, None);
            let food = Food::new(position);
            quad_tree.insert(food); // Ensure QuadTree accepts Point
        }

        self.add_food_count(food_amount)
    }

    pub async fn delete_many_foods(&self, foods_to_delete: Vec<&Food>) {
        let mut quad_tree = self.quad_tree.write().await;
        for food in foods_to_delete.iter() {
            quad_tree.remove(&food);
        }
        self.sub_food_count(foods_to_delete.len())
    }
}
