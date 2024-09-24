use std::{env, sync::OnceLock};

use crate::utils::consts::{Mass, TotalMass};

#[derive(Debug)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub logpath: String,
    pub food_mass: Mass,
    pub fire_food: Mass,
    pub limit_split: u32,
    pub split_min_mass: Mass,
    pub default_player_mass: Mass,
    pub virus: VirusConfig, // Ensure VirusConfig is also publicly accessible if needed
    pub game_width: u32,
    pub game_height: u32,
    pub food_capacity_q: u32,
    pub admin_pass: String,
    pub game_mass: TotalMass,
    pub max_food: usize,
    pub max_virus: usize,
    pub slow_base: u32,
    pub log_chat: bool,
    pub network_update_factor: u32,
    pub max_heartbeat_interval: i64,
    pub food_uniform_disposition: bool,
    pub new_player_initial_position: String,
    pub mass_loss_rate: f32,
    pub min_mass_loss: Mass,
}

impl Config {
    pub fn get_init_mass_log(&self) -> f32 {
        (self.default_player_mass as f32).log(self.slow_base as f32)
    }

    pub fn min_cell_mass(&self) -> Mass {
        self.split_min_mass.saturating_add(self.fire_food)
    }
}

pub fn get_current_config() -> &'static Config {
    static CONFIG: OnceLock<Config> = OnceLock::new();

    CONFIG.get_or_init(|| Config::default())
}

#[derive(Debug)]
pub struct VirusConfig {
    pub fill: String,
    pub stroke: String,
    pub stroke_width: f32,
    pub default_mass: DefaultMass,
    pub split_mass: Mass,
    pub uniform_disposition: bool,
}

#[derive(Debug)]
pub struct DefaultMass {
    pub from: Mass,
    pub to: Mass,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            host: "0.0.0.0".to_string(),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8000),
            logpath: "logger.php".to_string(),
            food_mass: 1,
            fire_food: 20,
            limit_split: 16,
            split_min_mass: 17,
            default_player_mass: 10,
            virus: VirusConfig::default(),
            game_width: 15000,
            game_height: 15000,
            food_capacity_q: 10,
            admin_pass: "DEFAULT".to_string(),
            game_mass: 500000,
            max_food: 4_000,
            max_virus: 100,
            slow_base: 50,
            log_chat: false,
            network_update_factor: 30,
            max_heartbeat_interval: 50000,
            food_uniform_disposition: true,
            new_player_initial_position: "farthest".to_string(),
            mass_loss_rate: 1.0,
            min_mass_loss: 50,
        }
    }
}

impl Default for VirusConfig {
    fn default() -> Self {
        VirusConfig {
            fill: "#33ff33".to_string(),
            stroke: "#19D119".to_string(),
            stroke_width: 20.0,
            default_mass: DefaultMass {
                from: 100,
                to: 150,
            },
            split_mass: 180,
            uniform_disposition: false,
        }
    }
}
