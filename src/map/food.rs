use uuid::Uuid;
use rand::Rng;
use crate::utils::util::{get_position, mass_to_radius};
use super::point::{Point, AsPoint};
use crate::utils::quad_tree::QuadTree;

#[derive(Debug, Clone, Copy)]
pub struct Food {
    pub id: Uuid,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub mass: f32,
    pub hue: u16,
}

impl Food {
    pub fn new(point: Point, radius: f32) -> Self {
        let mut rng = rand::thread_rng();
        Food {
            id: Uuid::new_v4(),
            x: point.x,
            y: point.y,
            radius: radius,
            mass: rng.gen_range(2.0..3.0),
            hue: rng.gen_range(0..360),
        }
    }
}

impl AsPoint for Food {
    fn as_point(&self) -> Point {
        Point { x: self.x, y: self.y, radius: self.radius}
    }
}


pub struct FoodManager {
    food_mass: f32,
    pub quad_tree: QuadTree
}

impl FoodManager {
    pub fn new(food_mass: f32, quad_tree: QuadTree) -> Self {
        FoodManager { food_mass, quad_tree }
    }

    //adds a new food with a random positioning
    pub fn add_new(&mut self, number: usize) {
        let radius = mass_to_radius(self.food_mass);
        for _ in 0..number {
            let position = get_position(false, radius, None);
            let food = Food::new(position, radius);
            self.quad_tree.insert(food);  // Ensure QuadTree accepts Point
        }
    }

    pub fn delete(&mut self, foods_to_delete: Vec<Food>) {
        for food in foods_to_delete {
            self.quad_tree.remove(&food);
        }
    }
}
