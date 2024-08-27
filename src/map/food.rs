use super::point::{AsPoint, Point};
use crate::utils::consts::Mass;
use crate::utils::id::FoodID;
use crate::utils::util::mass_to_radius;
use rand::Rng;

// #[derive(Serialize)]
// pub struct FoodData {
//     pub id: FoodID,
//     // pub x: f32,
//     // pub y: f32,
//     // pub mass: Mass,
//     pub hue: u16,
// }

pub type FoodData = (FoodID, u16);

#[derive(Debug, Clone, Copy)]
pub struct Food {
    pub id: FoodID,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub mass: Mass,
    pub hue: u16,
}

impl Food {
    pub fn new(food_id: FoodID, point: &Point) -> Self {
        let mut rng = rand::thread_rng();
        let mass = rng.gen_range(2..3);
        Food {
            id: food_id,
            x: point.x,
            y: point.y,
            radius: mass_to_radius(mass),
            mass,
            hue: rng.gen_range(0..360),
        }
    }

    pub fn generate_data(&self) -> FoodData {
        (self.id, self.hue)
        // FoodData {
        //     id: self.id,
        //     // x: self.x,
        //     // y: self.y,
        //     // mass: self.mass,
        //     hue: self.hue,
        // }
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
