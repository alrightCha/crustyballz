use super::player::{Player, PlayerManager};
use super::virus::{Virus, VirusManager};
use super::mass_food::{MassFood, MassFoodManager};
use super::food::{Food, FoodManager};
use crate::utils::quad_tree::{QuadTree, Rectangle};
use crate::utils::util::{is_visible_entity, get_visible_area};
use crate::config::Config;

pub struct Map {
    pub food: FoodManager,
    pub viruses: VirusManager,
    pub mass_food: MassFoodManager,
    pub players: PlayerManager,
    pub food_count: usize,
}

pub struct VisibleEntities<'a> {
    pub players: Vec<&'a Player>,  
    pub food: Vec<Food>,
    pub viruses: Vec<&'a Virus>,  
    pub mass_food: Vec<&'a MassFood>,  
}

impl Map{
    pub fn new() -> Self {
        Map {
            food: FoodManager::new(Config::default().food_mass, QuadTree::new(Rectangle::new(0.0, 0.0, 10000.0, 10000.0), 100)),
            viruses: VirusManager::new(),
            mass_food: MassFoodManager::new(),
            players: PlayerManager::new(),
            food_count: 0,
        }
    }

    pub fn get_food_in_view(&self, user: Player, ratio: &mut f32) -> Vec<Food> {
        // Assuming get_visible_area is a function that returns a Rectangle or similar structure
        let visible_zone = get_visible_area(&user, ratio);  // This line calls the function and stores its result in visible_area
        let mut found_foods: Vec<Food> = Vec::new();
    
        // Now use the visible_area variable that you've defined above
        self.food.quad_tree.retrieve(&visible_zone, &mut found_foods);
    
        found_foods
    }
    

    pub fn balance_mass(&mut self, game_mass: f32, max_food: usize, max_virus: usize) {
        // Calculate the total mass based on food and player mass
        let total_mass = self.food_count as f32 * Config::default().food_mass + self.players.get_total_mass();
        let mass_diff = game_mass - total_mass;

        // Calculate the amount of food that can be added based on available capacity and needed mass
        let food_free_capacity = max_food - self.food_count;
        let food_diff = mass_diff / Config::default().food_mass;
        let food_to_add = food_diff.floor().min(food_free_capacity as f32) as usize;

        // Add food if there is a need
        if food_to_add > 0 {
            println!("[DEBUG] Adding {} food", food_to_add);
            self.food.add_new(food_to_add);
            self.food_count += food_to_add;
        }
        let viruses_to_add = max_virus - self.viruses.count();
        if viruses_to_add > 0 {
            self.viruses.add_new(viruses_to_add);
        }
    }

    pub fn enumerate_what_player_sees(&self, player: &Player, ratio: &mut f32) -> VisibleEntities {
        let visible_food = self.get_food_in_view(player.clone(), ratio);
        // Get visible viruses
        let visible_viruses = self.viruses.data.iter()
        .filter(|virus| is_visible_entity(virus.get_position(), player.clone()))
        .collect::<Vec<&Virus>>(); // Assuming Virus is the type, adjust as necessary

        // Get visible mass food
        let visible_mass_food = self.mass_food.data.iter()
            .filter(|mass| is_visible_entity(mass.point, player.clone()))
            .collect::<Vec<&MassFood>>(); // Assuming MassFood is the type, adjust as necessary

        // Get visible players
        let visible_players = self.players.players.iter()
            .filter(|p| p.cells.iter().any(|cell| is_visible_entity(cell.position, player.clone())))
            .collect::<Vec<&Player>>();

        VisibleEntities {
            players: visible_players,
            food: visible_food,
            viruses: visible_viruses,
            mass_food: visible_mass_food,
        }
    }
}