use serde::{Deserialize, Serialize};

use crate::utils::{
    consts::{Mass, MIN_DISTANCE, MIN_SPEED, SPLIT_CELL_SPEED},
    util::{lerp_deg, lerp_move, mass_to_radius, math_log},
};

use super::point::Point;

#[derive(Serialize)]
pub struct CellData {
    // pub id: CellId,
    pub mass: Mass,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Cell {
    pub position: Point,
    pub mass: Mass,
    speed: f32,
    can_move: bool,
    direction_shot: Option<Point>,
    pub to_be_removed: bool,
}

impl Serialize for Cell {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        CellData {
            mass: self.mass,
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
        mass: Mass,
        speed: f32,
        can_move: bool,
        direction_shot: Option<Point>,
    ) -> Self {
        Self {
            position: Point {
                x,
                y,
                radius: mass_to_radius(mass),
            },
            mass,
            speed,
            can_move,
            direction_shot,
            to_be_removed: false,
        }
    }

    pub fn mark_for_removal(&mut self) {
        self.to_be_removed = true;
    }

    pub fn set_mass(&mut self, new_mass: Mass) {
        self.mass = new_mass.max(0);
        self.recalculate_radius();
    }

    pub fn remove_mass(&mut self, to_remove: Mass) {
        self.set_mass(self.mass.saturating_sub(to_remove));
    }

    pub fn add_mass(&mut self, to_add: Mass) {
        self.set_mass(self.mass.saturating_add(to_add));
    }

    fn recalculate_radius(&mut self) {
        self.position.radius = mass_to_radius(self.mass);
    }

    pub fn move_cell(
        &mut self,
        player_position: &Point,
        mouse_x: f32,
        mouse_y: f32,
        slow_base: f32,
        init_mass_log: f32,
    ) {
        let target_x = player_position.x - self.position.x + mouse_x;
        let target_y = player_position.y - self.position.y + mouse_y;
        let dist = (target_y.powi(2) + target_x.powi(2)).sqrt();
        let deg = target_y.atan2(target_x);

        let mut slow_down = 0.03;
        let (mut delta_y, mut delta_x);

        if self.can_move {
            if self.speed <= MIN_SPEED {
                slow_down = (self.mass as f32).log(slow_base * 3.0) - init_mass_log + 1.0;
            }
            delta_y = self.speed * deg.sin() / slow_down;
            delta_x = self.speed * deg.cos() / slow_down;

            if dist < (MIN_DISTANCE + self.position.radius) {
                let ratio = dist / (MIN_DISTANCE + self.position.radius);
                delta_y *= ratio;
                delta_x *= ratio;
            }
        } else {
            self.speed = lerp_move(self.speed, math_log(self.speed, Some(7.5), 5.0), 0.06);
            if self.speed <= MIN_SPEED {
                self.can_move = true;
                self.speed = MIN_SPEED;
            }
            if let Some(direction_shot) = self.direction_shot {
                let not_dis = f32::hypot(direction_shot.y, direction_shot.x);
                let not_deg = direction_shot.y.atan2(direction_shot.x);
                let real_deg = lerp_deg(not_deg, deg, 0.1 * SPLIT_CELL_SPEED / self.speed); // Assuming lerp_deg function exists
                delta_y = self.speed * real_deg.sin();
                delta_x = self.speed * real_deg.cos();
                if not_dis < MIN_DISTANCE + self.position.radius {
                    let ratio =
                        not_dis / (MIN_DISTANCE + (self.position.radius * 0.001)) / slow_down;
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
        // info!("speed: {}", self.speed);
    }
}
