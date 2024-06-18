use std::sync::atomic::AtomicUsize;

use log::debug;
use tokio::sync::RwLock;

use crate::{
    config::get_current_config, map::{
        food::{Food, FoodData},
        point::Point,
    }, utils::{
        consts::Mass,
        id::id_from_position,
        quad_tree::QuadTree,
        util::{create_random_number_u32, create_random_position, mass_to_radius},
    }
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
        let config = get_current_config();
        
        let min_x = (mass_to_radius(config.food_mass)) as u16;
        let max_x = ((config.game_width as f32) - mass_to_radius(config.food_mass)) as u16;
        let min_y = (mass_to_radius(config.food_mass)) as u16;
        let max_y = ((config.game_height as f32) - mass_to_radius(config.food_mass)) as u16;


        let mut new_foods_data: Vec<FoodData> = vec![];
        let mut quad_tree = self.quad_tree.write().await;

        let radius = mass_to_radius(self.default_food_mass);
        for _ in 0..food_amount {
            let mut food_id;
            let position;

            loop {
                let x = create_random_number_u32(min_x, max_x);
                let y = create_random_number_u32(min_y, max_y);

                food_id = id_from_position(x, y);

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

            let food = Food::new(food_id, &position);
            
            if quad_tree.insert(food) {
                new_foods_data.push(food.generate_data());
            } else {
                debug!("Failed to added food[{}] - {:?}", food_id, position);
            }
        }

        self.add_food_count(food_amount);

        new_foods_data
    }

    pub async fn delete_many_foods(&self, foods_to_delete: impl Iterator<Item=&Food>) {
        let mut quad_tree = self.quad_tree.write().await;
        let mut length = 0;
        for food in foods_to_delete {
            if quad_tree.remove(&food) {
                length += 1;
            }
        }
        self.sub_food_count(length);
    }

    pub async fn get_foods_init_data(&self) -> Vec<FoodData> {
        let mut foods_data = vec![];

        for food in self.quad_tree.read().await.get_all_foods() {
            foods_data.push(food.generate_data());
        }

        foods_data
    }
}