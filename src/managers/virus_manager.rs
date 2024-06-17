use crate::{
    config::VirusConfig,
    map::{
        point::Point,
        virus::{Virus, VirusData},
    },
    utils::{
        consts::Mass,
        id::VirusID,
        util::{create_random_position, mass_to_radius, random_in_range},
    },
};

pub struct VirusManager {
    pub data: Vec<Virus>,
    virus_config: VirusConfig,
    id_counter: VirusID,
}

impl VirusManager {
    pub fn new() -> Self {
        VirusManager {
            data: Vec::new(),
            virus_config: VirusConfig::default(), // Correctly assign the `virus` field from `config`
            id_counter: VirusID::MAX,
        }
    }

    fn get_new_id(&mut self) -> VirusID {
        self.id_counter = self.id_counter.wrapping_add(1);
        self.id_counter
    }

    fn create_virus(&mut self, position: Point, mass: Mass, direction: Option<Point>) -> Virus {
        let virus_id = self.get_new_id();

        Virus::new(virus_id, position, mass, direction)
    }

    pub fn push_new(&mut self, virus: Virus) {
        self.data.push(virus);
    }

    pub fn create_many_virus(&mut self, number: usize) -> Vec<VirusData>{
        let mut new_virus_data = vec![];

        for _ in 0..number {
            let mass = random_in_range(
                self.virus_config.default_mass.from..self.virus_config.default_mass.to,
            );
            let radius = mass_to_radius(mass);
            let position =
                create_random_position(self.virus_config.uniform_disposition, radius, None);
            let new_virus = self.create_virus(position, mass, None);

            new_virus_data.push(new_virus.generate_data());

            self.data.push(new_virus);
        }

        new_virus_data
    }

    //Divides a virus by reducing its mass and creating a new virus with the initial position being the center of the original virus,
    //and the new direction being the last direction aimed by the player right before the split
    pub fn shoot_one(&mut self, position: Point, direction: Point) -> VirusData {
        let mass =
            random_in_range(self.virus_config.default_mass.from..self.virus_config.default_mass.to);

        let mut new_virus = self.create_virus(position, mass, Some(direction));

        new_virus.set_speed(50.0);

        let virus_data = new_virus.generate_data();

        self.push_new(new_virus);

        virus_data
    }

    pub fn delete(&mut self, virus_id: VirusID) {
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

    pub fn get_virus_data(&self) -> Vec<VirusData> {
        self.data.iter().map(|v| v.generate_data()).collect()
    }
}
