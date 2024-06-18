use super::point::Point;
use crate::config::VirusConfig;
use crate::utils::consts::Mass;
use crate::utils::game_logic::adjust_for_boundaries;
use crate::utils::id::VirusID;
use crate::utils::util::{are_colliding, check_overlap, create_random_position, mass_to_radius, random_in_range};
use rand::Rng;
use serde::Serialize;
use tokio::sync::RwLock;

#[derive(Serialize, Clone)]
pub struct VirusData {
    pub id: VirusID,
    pub x: f32,
    pub y: f32,
    pub mass: Mass,
}

#[derive(Clone)]
pub struct Virus {
    pub id: VirusID,
    x: f32,
    y: f32,
    radius: f32,
    pub mass: Mass,
    stroke: String,
    stroke_width: f32,
    direction: Option<Point>,
    pub speed: Option<f32>,
    fill: String,
}

impl Virus {
    pub fn new(virus_id: VirusID, point: Point, mass: Mass, direction: Option<Point>) -> Self {
        let virus_config = VirusConfig::default();
        Virus {
            id: virus_id,
            x: point.x,
            y: point.y,
            radius: mass_to_radius(mass),
            mass: mass,
            speed: Some(0.0),
            direction: direction,
            stroke_width: virus_config.stroke_width,
            stroke: virus_config.stroke,
            fill: virus_config.fill,
        }
    }

    pub fn generate_data(&self) -> VirusData {
        VirusData {
            id: self.id,
            x: self.x,
            y: self.y,
            mass: self.mass,
        }
    }

    pub fn can_be_eat_by(&self, cell_mass: Mass, cell_point: &Point) -> bool {
        (cell_mass as f32) > (1.1 * (self.mass as f32))
            && check_overlap(&self.get_position(), &cell_point)
    }

    pub fn get_position(&self) -> Point {
        return Point {
            x: self.x,
            y: self.y,
            radius: self.radius,
        };
    }

    pub fn set_speed(&mut self, new_speed: f32) {
        self.speed = Some(new_speed);
    }

    pub fn move_virus(&mut self, game_width: f32, game_height: f32) {
        if let Some(speed) = self.speed {
            if let Some(dir) = &self.direction {
                let delta_x = speed * dir.x;
                let delta_y = speed * dir.y;
                self.speed = Some(speed - 1);
                if self.speed.unwrap() < 0.0 {
                    self.speed = None;
                }

                self.x += delta_x;
                self.y += delta_y;
            }
        }

        if self.speed.is_some() {
            adjust_for_boundaries(
                &mut self.x,
                &mut self.y,
                self.radius,
                5.0,
                game_width,
                game_height,
            );
        }
    }

    pub fn set_mass(&mut self, new_mass: Mass) {
        self.mass = new_mass;
        self.recalculate_radius();
    }

    pub fn add_mass(&mut self, to_add: Mass) {
        self.set_mass(self.mass.saturating_add(to_add))
    }

    fn recalculate_radius(&mut self) {
        self.radius = mass_to_radius(self.mass);
    }
}
