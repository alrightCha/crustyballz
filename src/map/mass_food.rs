

use super::point::{Point};
use crate::utils::util::mass_to_radius;
use super::player::Player;
use uuid::Uuid;
use crate::utils::game_logic::adjust_for_boundaries;

#[derive(Debug, Clone)]
pub struct MassFood {
    id: Uuid,
    num: usize,
    mass: f32,
    hue: u16,
    direction: Point,
    pub point: Point,
    speed: Option<f32>,
}

impl MassFood {
    pub fn new(player_firing: &Player, cell_index: usize, mass: f32) -> Self {
        let cell = &player_firing.cells[cell_index];
        let direction = Point {
            x: 0.4 * (player_firing.x - cell.position.x) + player_firing.target_x,
            y: 0.4 * (player_firing.y - cell.position.y) + player_firing.target_y,
            radius: 0.0,  // Radius doesn't participate in direction calculation
        }.normalize();

        MassFood {
            id: Uuid::new_v4(),
            num: cell_index,
            mass,
            hue: player_firing.hue,
            direction,
            point: Point{x: cell.position.x + direction.x * cell.position.radius, y: cell.position.y + direction.y * cell.position.radius, radius: mass_to_radius(mass)},
            speed: Some(20.0),
        }
    }

    pub fn get_player_target(&self) -> Point {
        self.direction
    }

    pub fn move_self(&mut self, game_width: f32, game_height: f32) {
        if let Some(ref mut speed) = self.speed {
            let delta_x = *speed * self.direction.x;
            let delta_y = *speed * self.direction.y;

            *speed -= 0.5;
            if *speed < 0.0 {
                self.speed = None;
            }
            self.point.x += delta_x;
            self.point.y += delta_y;

            adjust_for_boundaries(&mut self.point.x, &mut self.point.y, self.point.radius, 5.0, game_width, game_height);
        }
    }
}

#[derive(Default, Debug)]
pub struct MassFoodManager {
    pub data: Vec<MassFood>,
}

impl MassFoodManager {
    pub fn new() -> Self {
        MassFoodManager { data: Vec::new() }
    }

    pub fn get_player_target(&self, id: Uuid) -> Option<Point> {
        self.data.iter().find(|&food| food.id == id.try_into().unwrap()).map(|food| food.get_player_target())
    }

    pub fn add_new(&mut self, player_firing: &Player, cell_index: usize, mass: f32) {
        self.data.push(MassFood::new(player_firing, cell_index, mass));
    }

    pub fn move_food(&mut self, game_width: f32, game_height: f32) {
        for food in self.data.iter_mut() {
            if food.speed.is_some() {
                food.move_self(game_width, game_height);
            }
        }
    }

    pub fn remove(&mut self, indexes: Vec<usize>) {
        let mut offset = 0;
        for index in indexes {
            self.data.remove(index - offset);
            offset += 1;
        }
    }
}
