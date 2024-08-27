use crate::{
    map::{
        mass_food::{MassFood, MassFoodInitData, MassFoodUpdateData},
        point::Point,
    },
    utils::{consts::Mass, id::MassFoodID},
};

#[derive(Default, Debug)]
pub struct MassFoodManager {
    pub data: Vec<MassFood>,
    id_counter: MassFoodID,
}

impl MassFoodManager {
    pub fn new() -> Self {
        MassFoodManager {
            data: Vec::new(),
            id_counter: 0,
        }
    }

    pub fn get_new_id(&mut self) -> MassFoodID {
        self.id_counter = self.id_counter.wrapping_add(1);
        self.id_counter
    }

    pub fn add_new(
        &mut self,
        player_position: &Point,
        player_target: &Point,
        cell_transform: &Point,
        hue: u16,
        mass: Mass,
    ) -> MassFoodInitData {
        let id = self.get_new_id();
        let mass_food = MassFood::new(
            id,
            &player_position,
            &player_target,
            hue,
            cell_transform,
            mass,
        );
        let mass_food_init_data = mass_food.generate_init_data();
        self.data.push(mass_food);

        mass_food_init_data
    }

    //moves the mass until the speed is 0
    pub fn move_food(&mut self, game_width: f32, game_height: f32) -> Vec<MassFoodUpdateData> {
        self.data
            .iter_mut()
            .filter_map(|mass_food| {
                if mass_food.speed.is_some() {
                    mass_food.move_self(game_width, game_height);
                    return Some(mass_food.generate_update_data());
                }
                None
            })
            .collect()
    }

    pub fn remove_food(&mut self, mass_id: MassFoodID) {
        match self.data.iter().position(|x| x.id == mass_id) {
            Some(index) => {
                self.data.remove(index);
            }
            None => {}
        }
    }

    pub fn get_mass_food_init_data(&self) -> Vec<MassFoodInitData> {
        self.data.iter().map(|m| m.generate_init_data()).collect()
    }
}
