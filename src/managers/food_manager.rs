use std::sync::atomic::AtomicUsize;

use tokio::sync::RwLock;

use crate::{
    map::{
        food::{Food, FoodData},
        point::Point,
    },
    utils::{
        consts::Mass,
        id::from_position,
        quad_tree::QuadTree,
        util::{create_random_number_u32, create_random_position, mass_to_radius},
    },
};

pub struct FoodManager {
    default_food_mass: Mass,
    pub quad_tree: RwLock<QuadTree>,
    food_count: AtomicUsize,
}

impl FoodManager {
    pub fn new(food_mass: Mass, quad_tree: QuadTree) -> Self {
        FoodManager {
            default_food_mass: food_mass,
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

    pub async fn create_many_foods(&self, food_amount: usize) -> Vec<FoodData> {
        let mut new_foods_data: Vec<FoodData> = vec![];
        let mut quad_tree = self.quad_tree.write().await;

        let radius = mass_to_radius(self.default_food_mass);
        for _ in 0..food_amount {
            let mut food_id;
            let position;

            loop {
                let x = create_random_number_u32(0, u16::MAX);
                let y = create_random_number_u32(0, u16::MAX);

                food_id = from_position(x, y);

                if quad_tree.contains_food(food_id) {
                    continue;
                }

                position = Point {
                    x: x as f32,
                    y: y as f32,
                    radius,
                };
                break;
            }

            let food = Food::new(food_id, position);
            new_foods_data.push(food.generate_data());
            quad_tree.insert(food); // Ensure QuadTree accepts Point
        }

        self.add_food_count(food_amount);

        new_foods_data
    }

    pub async fn delete_many_foods(&self, foods_to_delete: Vec<&Food>) {
        let mut quad_tree = self.quad_tree.write().await;
        for food in foods_to_delete.iter() {
            quad_tree.remove(&food);
        }
        self.sub_food_count(foods_to_delete.len())
    }

    pub async fn get_foods_init_data(&self) -> Vec<FoodData> {
        let mut foods_data = vec![];

        for food in self.quad_tree.read().await.get_all_foods() {
            foods_data.push(food.generate_data());
        }

        foods_data
    }
}
