use uuid::Uuid;

use crate::{config::VirusConfig, map::{point::Point, virus::Virus}, utils::util::{create_random_position, mass_to_radius, random_in_range}};

pub struct VirusManager {
    pub data: Vec<Virus>,
    virus_config: VirusConfig,
}

impl VirusManager {
    pub fn new() -> Self {
        VirusManager {
            data: Vec::new(),
            virus_config: VirusConfig::default(), // Correctly assign the `virus` field from `config`
        }
    }

    pub fn push_new(&mut self, virus: Virus) {
        self.data.push(virus);
    }

    pub fn add_new(&mut self, number: usize) {
        for _ in 0..number {
            let mass = random_in_range(
                self.virus_config.default_mass.from,
                self.virus_config.default_mass.to,
            );
            let radius = mass_to_radius(mass);
            let position = create_random_position(self.virus_config.uniform_disposition, radius, None);
            let new_virus = Virus::new(position, mass, None);
            self.data.push(new_virus);
        }
    }

    //Divides a virus by reducing its mass and creating a new virus with the initial position being the center of the original virus,
    //and the new direction being the last direction aimed by the player right before the split
    pub fn shoot_one(&mut self, position: Point, direction: Point) {
        let mass = random_in_range(
            self.virus_config.default_mass.from,
            self.virus_config.default_mass.to,
        );
        let mut new_virus = Virus::new(position, mass, Some(direction));
        new_virus.set_speed(50.0);
        self.push_new(new_virus);
    }

    pub fn delete(&mut self, virus_id: Uuid) {
        match self.data.iter().position(|x| x.id == virus_id) {
            Some(index) => {
                self.data.remove(index);
            }
            None => {}
        }
    }

    pub fn count(&self) -> usize {
        self.data.len()
    }
}