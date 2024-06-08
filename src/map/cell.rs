use serde::Serialize;

use crate::utils::{
    consts::{MIN_DISTANCE, MIN_SPEED, SPEED_DECREMENT, SPLIT_CELL_SPEED},
    util::{lerp, lerp_deg, mass_to_radius, math_log},
};

use super::point::Point;

#[derive(Serialize)]
pub struct CellData {
    pub canMove: bool,
    pub imgUrl: Option<String>,
    pub mass: f32,
    pub speed: f32,
    pub radius: f32,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub position: Point,
    pub mass: f32,
    speed: f32,
    can_move: bool,
    direction_shot: Option<Point>,
    pub img_url: Option<String>,
    pub to_be_removed: bool,
}

impl Serialize for Cell {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        CellData {
            canMove: self.can_move,
            imgUrl: self.img_url.clone(),
            mass: self.mass,
            radius: self.position.radius,
            speed: self.speed,
            x: self.position.x,
            y: self.position.y,
        }
        .serialize(serializer)
    }
}

impl Cell {
    pub fn new(
        x: f32,
        y: f32,
        mass: f32,
        speed: f32,
        can_move: bool,
        direction_shot: Option<Point>,
        img_url: Option<String>,
    ) -> Self {
        Self {
            position: Point {
                x: x,
                y: y,
                radius: mass_to_radius(mass),
            },
            mass,
            speed,
            can_move,
            direction_shot,
            img_url,
            to_be_removed: false,
        }
    }

    pub fn mark_for_removal(&mut self) {
        self.to_be_removed = true;
    }

    pub fn set_mass(&mut self, new_mass: f32) {
        self.mass = new_mass.max(0.0);
        self.recalculate_radius();
    }

    pub fn remove_mass(&mut self, to_remove: f32) {
        self.set_mass(self.mass - to_remove)
    }

    pub fn add_mass(&mut self, to_add: f32) {
        self.set_mass(self.mass + to_add)
    }

    fn recalculate_radius(&mut self) {
        self.position.radius = mass_to_radius(self.mass);
    }

    pub fn move_cell(&mut self, mouse_x: f32, mouse_y: f32, slow_base: f32, init_mass_log: f32) {
        let target_x = self.position.x - self.position.x + mouse_x;
        let target_y = self.position.y - self.position.y + mouse_y;
        let dist = (target_y.powi(2) + target_x.powi(2)).sqrt();
        let deg = target_y.atan2(target_x);

        let mut slow_down = 1.0;
        let (mut delta_y, mut delta_x);

        if self.can_move {
            if self.speed <= MIN_SPEED {
                slow_down = math_log(self.mass, Some(slow_base * 3.0)) - init_mass_log + 1.0;
            }
            delta_y = self.speed * deg.sin() / slow_down;
            delta_x = self.speed * deg.cos() / slow_down;
            if dist < (MIN_DISTANCE + self.position.radius) {
                let ratio = dist / (MIN_DISTANCE + self.position.radius);
                delta_y *= ratio;
                delta_x *= ratio;
            }
        } else {
            self.speed = lerp(self.speed, self.speed - SPEED_DECREMENT, 0.9); // Assuming lerp function exists
            if self.speed <= MIN_SPEED {
                self.speed = MIN_SPEED;
                self.can_move = true;
            }

            if let Some(direction_shot) = self.direction_shot {
                let not_dis = (direction_shot.y.powi(2) + direction_shot.x.powi(2)).sqrt();
                let not_deg = direction_shot.y.atan2(direction_shot.x);
                let real_deg = lerp_deg(not_deg, deg, 0.08 * SPLIT_CELL_SPEED / self.speed); // Assuming lerp_deg function exists

                delta_y = self.speed * real_deg.sin();
                delta_x = self.speed * real_deg.cos();
                if not_dis < MIN_DISTANCE + self.position.radius {
                    let ratio = not_dis / (MIN_DISTANCE + self.position.radius) / slow_down;
                    delta_y *= ratio;
                    delta_x *= ratio;
                }
            } else {
                delta_y = 0.0;
                delta_x = 0.0;
            }
        }

        self.position.y += delta_y;
        self.position.x += delta_x;
    }
}
