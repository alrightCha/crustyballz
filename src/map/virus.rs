use super::point::Point;
use crate::config::VirusConfig;
use crate::utils::game_logic::adjust_for_boundaries;
use crate::utils::util::{are_colliding, get_position, mass_to_radius};
use rand::Rng;
use serde::Serialize;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone, Serialize)]
pub struct Virus {
    id: Uuid,
    x: f32,
    y: f32,
    radius: f32,
    pub mass: f32,
    stroke: String,
    stroke_width: f32,
    direction: Option<Point>,
    speed: Option<f32>,
    fill: String,
}

impl Virus {
    pub fn new(point: Point, mass: f32, direction: Option<Point>) -> Self {
        let virus_config = VirusConfig::default();
        Virus {
            id: Uuid::new_v4(),
            x: point.x,
            y: point.y,
            radius: point.radius,
            mass: mass,
            speed: Some(0.0),
            direction: direction,
            stroke_width: virus_config.stroke_width,
            stroke: virus_config.stroke,
            fill: virus_config.fill,
        }
    }

    pub fn can_be_eat_by(&self, cell_mass: f32, cell_point: Point) -> bool {
        cell_mass > 1.1 * self.mass && are_colliding(&self.get_position(), &cell_point)
    }

    pub fn get_position(&self) -> Point {
        return Point {
            x: self.x,
            y: self.y,
            radius: self.radius,
        };
    }

    fn set_speed(&mut self, new_speed: f32) {
        self.speed = Some(new_speed);
    }

    fn move_virus(&mut self, game_width: f32, game_height: f32) {
        if let Some(speed) = self.speed {
            if let Some(dir) = &self.direction {
                let delta_x = speed * dir.x;
                let delta_y = speed * dir.y;
                self.speed = Some(speed - 0.5);
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

    fn set_mass(&mut self, new_mass: f32) {
        self.mass = new_mass;
        self.recalculate_radius();
    }

    fn add_mass(&mut self, to_add: f32) {
        self.set_mass(self.mass + to_add)
    }

    fn recalculate_radius(&mut self) {
        self.radius = mass_to_radius(self.mass);
    }
}

pub struct VirusManager {
    pub data: RwLock<Vec<Virus>>,
    virus_config: VirusConfig,
}

impl VirusManager {
    pub fn new() -> Self {
        VirusManager {
            data: RwLock::new(Vec::new()),
            virus_config: VirusConfig::default(), // Correctly assign the `virus` field from `config`
        }
    }

    pub async fn push_new(&self, virus: Virus) {
        self.data.write().await.push(virus);
    }

    fn random_in_range(&self, low: f32, high: f32) -> f32 {
        rand::thread_rng().gen_range(low..high)
    }

    pub async fn add_new(&self, number: usize) {
        let mut data = self.data.write().await;
        for _ in 0..number {
            let mass = self.random_in_range(
                self.virus_config.default_mass.from,
                self.virus_config.default_mass.to,
            );
            let radius = mass_to_radius(mass);
            let position = get_position(self.virus_config.uniform_disposition, radius, None);
            let new_virus = Virus::new(position, mass, None);
            data.push(new_virus);
        }
    }

    //Divides a virus by reducing its mass and creating a new virus with the initial position being the center of the original virus, 
    //and the new direction being the last direction aimed by the player right before the split
    pub fn shoot_one(&mut self, position: Point, direction: Point) {
        let mass = self.random_in_range(
            self.virus_config.default_mass.from,
            self.virus_config.default_mass.to,
        );
        let mut new_virus = Virus::new(position, mass, Some(direction));
        new_virus.set_speed(25.0);
        self.push_new(new_virus);
    }

    // pub fn delete(&mut self, index: usize) {
    //     if index < self.data.len() {
    //         self.data.remove(index);
    //     }
    // }

    pub async fn count(&self) -> usize {
        self.data.read().await.len()
    }
}
