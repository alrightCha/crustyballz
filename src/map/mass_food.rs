use super::player::Player;
use super::point::Point;
use crate::utils::consts::Mass;
use crate::utils::game_logic::adjust_for_boundaries;
use crate::utils::id::{MassFoodID, PlayerID};
use crate::utils::util::{are_colliding, mass_to_radius};
use serde::Serialize;

#[derive(Serialize)]
pub struct MassFoodUpdateData {
    pub id: MassFoodID,
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize)]
pub struct MassFoodInitData {
    pub id: MassFoodID,
    pub hue: u16,
    // pub mass: Mass,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub struct MassFood {
    pub id: MassFoodID,
    pub mass: Mass,
    hue: u16,
    pub direction: Point,
    pub point: Point,
    pub speed: Option<f32>,
}

impl MassFood {
    pub fn new(
        mass_food_id: MassFoodID,
        position: &Point,
        direction: &Point,
        hue: u16,
        cell_transform: &Point,
        mass: Mass,
    ) -> Self {
        let direction = Point {
            x: 0.4 * (position.x - cell_transform.x) + direction.x,
            y: 0.4 * (position.y - cell_transform.y) + direction.y,
            radius: 0.0, // Radius doesn't participate in direction calculation
        }
        .normalize();

        MassFood {
            id: mass_food_id,
            mass,
            hue,
            direction,
            point: Point {
                x: cell_transform.x + direction.x * cell_transform.radius,
                y: cell_transform.y + direction.y * cell_transform.radius,
                radius: mass_to_radius(mass),
            },
            speed: Some(20.0),
        }
    }

    pub fn generate_init_data(&self) -> MassFoodInitData {
        MassFoodInitData {
            hue: self.hue,
            id: self.id,
            // mass: self.mass,
            x: self.point.x,
            y: self.point.y,
        }
    }

    pub fn generate_update_data(&self) -> MassFoodUpdateData {
        MassFoodUpdateData {
            id: self.id,
            x: self.point.x,
            y: self.point.y,
        }
    }

    pub fn get_player_target(&self) -> Point {
        self.direction
    }

    pub fn can_be_eat_by(&self, cell_mass: Mass, cell_position: &Point) -> bool {
        if are_colliding(&cell_position, &self.point) {
            if self.speed.unwrap_or_default() > 0.0 {
                return false;
            }
            return (cell_mass as f32) > ((self.mass as f32) * 1.1f32)
                && self.speed.unwrap_or_default() < 18.0;
        }
        false
    }

    pub fn move_self(&mut self, game_width: f32, game_height: f32) {
        if let Some(ref mut speed) = self.speed {
            let delta_x = *speed * self.direction.x;
            let delta_y = *speed * self.direction.y;

            *speed -= 1.25;
            if *speed < 0.0 {
                self.speed = None;
            }
            self.point.x += delta_x;
            self.point.y += delta_y;

            adjust_for_boundaries(
                &mut self.point.x,
                &mut self.point.y,
                self.point.radius,
                5.0,
                game_width,
                game_height,
            );
        }
    }
}
