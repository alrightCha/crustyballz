use uuid::Uuid;

use crate::map::{mass_food::MassFood, point::Point};

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
        for mass_food in self.data.iter_mut() {
            if mass_food.speed.is_some() {
                mass_food.move_self(game_width, game_height);
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
