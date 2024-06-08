use super::player::Player;
use super::point::Point;
use crate::utils::game_logic::adjust_for_boundaries;
use crate::utils::util::{are_colliding, mass_to_radius};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct MassFoodData {
    pub hue: u16,
    pub id: Uuid,
    pub mass: f32,
    pub direction: Point,
    pub radius: f32,
    pub speed: Option<f32>,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub struct MassFood {
    pub id: Uuid,
    pub mass: f32,
    hue: u16,
    pub direction: Point,
    pub point: Point,
    speed: Option<f32>,
}

impl Serialize for MassFood {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        MassFoodData {
            direction: self.direction,
            hue: self.hue,
            id: self.id,
            mass: self.mass,
            radius: self.point.radius,
            speed: self.speed,
            x: self.point.x,
            y: self.point.y,
        }
        .serialize(serializer)
    }
}

impl MassFood {
    pub fn new(
        position: &Point,
        direction: &Point,
        hue: u16,
        cell_transform: &Point,
        mass: f32,
    ) -> Self {
        let direction = Point {
            x: 0.4 * (position.x - cell_transform.x) + direction.x,
            y: 0.4 * (position.y - cell_transform.y) + direction.y,
            radius: 0.0, // Radius doesn't participate in direction calculation
        }
        .normalize();

        MassFood {
            id: Uuid::new_v4(),
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

    pub fn get_player_target(&self) -> Point {
        self.direction
    }

    pub fn can_be_eat_by(&self, player_id: &Uuid, cell_mass: f32, cell_position: Point) -> bool {
        if are_colliding(&cell_position, &self.point) {
            if &self.id == player_id && self.speed.unwrap_or_default() > 0.0 {
                return false;
            }
            return cell_mass > self.mass * 1.1f32 && self.speed.unwrap_or_default() < 18.0;
        }
        false
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

#[derive(Default, Debug)]
pub struct MassFoodManager {
    pub data: Vec<MassFood>,
}

impl MassFoodManager {
    pub fn new() -> Self {
        MassFoodManager { data: Vec::new() }
    }

    // pub fn get_player_target(&self, id: Uuid) -> Option<Point> {
    //     self.data
    //         .iter()
    //         .find(|&food| food.id == id.try_into().unwrap())
    //         .map(|food| food.get_player_target())
    // }

    pub fn add_new(
        &mut self,
        player_position: &Point,
        player_target: &Point,
        cell_transform: &Point,
        hue: u16,
        mass: f32,
    ) {
        self.data.push(MassFood::new(
            &player_position,
            &player_target,
            hue,
            cell_transform,
            mass,
        ));
    }

    //moves the mass until the speed is 0  
    pub fn move_food(&mut self, game_width: f32, game_height: f32) {
        for food in self.data.iter_mut() {
            if food.speed.is_some() {
                food.move_self(game_width, game_height);
            }
        }
    }

    pub fn remove_food(&mut self, mass_id: Uuid) {
        match self.data.iter().position(|x| x.id == mass_id) {
            Some(index) => {
                self.data.remove(index);
            }
            None => {}
        }
    }
}
