use super::cell::Cell;
use super::point::Point;
use crate::config::get_current_config;
use crate::utils::consts::{
    Mass, TotalMass, MERGE_TIMER, MIN_SPEED, PUSHING_AWAY_SPEED, SPLIT_CELL_SPEED,
};
use crate::utils::game_logic::adjust_for_boundaries;
use crate::utils::id::PlayerID;
use crate::utils::quad_tree::Rectangle;
use crate::utils::util::{
    check_overlap, check_who_ate_who, get_current_timestamp, lerp, total_mass_to_radius,
};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use socketioxide::socket::Sid;

#[derive(Serialize, Clone, Deserialize)]
pub struct PlayerUpdateData {
    pub id: PlayerID,
    pub cells: Vec<Cell>,
    pub x: f32,
    pub y: f32,
    pub bet: u64,
    pub won: u64
}

#[derive(Serialize, Clone, Deserialize)]
pub struct PlayerInitData {
    pub admin: bool,
    pub id: PlayerID,
    pub hue: u16,
    pub img_url: Option<String>,
    pub name: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct Player {
    pub id: PlayerID,
    pub socket_id: Sid,
    pub hue: u16,
    pub name: Option<String>,
    admin: bool,
    pub screen_width: f32,
    pub screen_height: f32,
    pub time_to_merge: Option<i64>,
    pub img_url: Option<String>,
    pub last_heartbeat: i64,
    // Properties to be initialized later
    pub cells: Vec<Cell>,
    pub total_mass: TotalMass,
    pub x: f32,
    pub y: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub ratio: f32,
    pub bet: u64,
    pub won: u64,
    pub bet_set: bool
}

impl Player {
    pub fn new(player_id: PlayerID, socket_id: Sid) -> Self {
        Self {
            id: player_id,
            socket_id: socket_id,
            hue: rand::random::<u16>() % 360,
            name: None,
            admin: false,
            screen_width: 800.0,
            screen_height: 600.0,
            time_to_merge: None,
            img_url: None,
            last_heartbeat: get_current_timestamp(),
            // Initial states for properties to be initialized later
            cells: Vec::new(),
            total_mass: get_current_config().default_player_mass as usize,
            x: 0.0,
            y: 0.0,
            target_x: 0.0,
            target_y: 0.0,
            ratio: 1.03,
            bet: 0,
            won: 0,
            bet_set: false
        }
    }
    
    pub fn get_id(&self) -> u8 {
        self.id
    }

    pub fn reset(&mut self, new_position: &Point, new_mass: Mass) {
        self.x = new_position.x;
        self.y = new_position.y;
        self.target_x = 0.0;
        self.target_y = 0.0;

        self.cells = vec![
            Cell::new(new_position.x, new_position.y, new_mass, MIN_SPEED, true, None)
        ];

        self.recalculate_total_mass();
        self.recalculate_ratio();
    }

    pub fn setup(
        &mut self,
        name: Option<String>,
        img_url: Option<String>,
    ) {
        self.name = name;
        self.img_url = img_url.clone();
    }

    pub fn player_is_dead(&self) -> bool {
        self.cells.len() <= 0
    }

    pub fn get_position_point(&self) -> Point {
        Point {
            x: self.x,
            y: self.y,
            radius: total_mass_to_radius(self.total_mass),
        }
    }

    pub fn get_target_point(&self) -> Point {
        Point {
            x: self.target_x,
            y: self.target_y,
            radius: total_mass_to_radius(self.total_mass),
        }
    }

    pub fn recalculate_ratio(&mut self) {
        let new_val = lerp(
            self.ratio,
            0.7 - 0.2 * ((self.total_mass as f32) / 500.0).ln()
                - 0.3 * (self.cells.len() as f32) / 18.0,
            0.1,
        );
        if new_val > 0.0 {
            self.ratio = new_val;
        }
    }

    pub fn recalculate_total_mass(&mut self) {
        self.total_mass = self.cells.iter().map(|c| c.mass as TotalMass).sum();
    }

    pub fn generate_init_player_data(&self) -> PlayerInitData {
        PlayerInitData {
            admin: false,
            id: self.id,
            name: self.name.clone(),
            hue: self.hue,
            img_url: self.img_url.clone(),
        }
    }

    pub fn generate_update_player_data(&self) -> PlayerUpdateData {
        PlayerUpdateData {
            id: self.id,
            cells: self.cells.clone(),
            x: self.x,
            y: self.y,
            bet: self.bet,
            won: self.won
        }
    }

    pub fn get_visible_area(&self) -> Rectangle {
        let half_width = (self.screen_width / self.ratio) / 2.0;
        let half_height = (self.screen_height / self.ratio) / 2.0;

        Rectangle::new(
            self.x - half_width,
            self.y - half_height,
            half_width * 2.0,
            half_height * 2.0,
        )
    }

    // pub fn client_provided_data(
    //     &mut self,
    //     img_url: String,
    //     name: String,
    //     screen_width: f32,
    //     screen_height: f32,
    // ) {
    //     self.img_url = Some(img_url);
    //     self.name = Some(name);
    //     self.screen_width = screen_width;
    //     self.screen_height = screen_height;
    //     self.set_last_heartbeat();
    // }

    pub fn lose_mass_if_needed(
        &mut self,
        mass_loss_rate: f32,
        default_player_mass: Mass,
        min_mass_loss: Mass,
    ) {
        for i in 0..self.cells.len() {
            if (self.cells[i].mass as f32) * (1.0 - (mass_loss_rate / 1000.0))
                > (default_player_mass as f32)
                && self.total_mass > (min_mass_loss as TotalMass)
            {
                let mass_loss = ((self.cells[i].mass as f32) * (mass_loss_rate / 1000.0)) as Mass;
                self.reduce_cell_mass(i as u8, mass_loss);
            }
        }
    }

    pub fn set_last_heartbeat(&mut self) {
        self.last_heartbeat = get_current_timestamp();
    }

    pub fn set_last_split(&mut self) {
        // let merge_duration = 1000.0 * MERGE_TIMER + self.total_mass / 100.0;
        let merge_duration = MERGE_TIMER + (self.total_mass as f32) / 100.0;
        self.time_to_merge = Some(get_current_timestamp() + merge_duration as i64);
    }

    pub fn reduce_cell_mass(&mut self, cell_index: u8, mass: Mass) {
        self.cells[cell_index as usize].remove_mass(mass);
        self.total_mass = self.total_mass.saturating_add(mass as usize);
    }

    fn split_cell(
        &mut self,
        cell_index: usize,
        max_requested_pieces: u8,
        default_player_mass: Mass,
        split_dir: Option<f32>,
    ) {
        if cell_index >= self.cells.len() {
            return; // Early return if the cell index is out of bounds
        }

        // Extract all necessary data from the cell
        let (cell_pos_x, cell_pos_y, cell_mass) = {
            let cell = &self.cells[cell_index];
            (cell.position.x, cell.position.y, cell.mass)
        };

        let max_allowed_pieces = (cell_mass / default_player_mass) as u8;
        let pieces_to_create = max_requested_pieces.min(max_allowed_pieces);

        // info!(
        //     "max is : {} /// i'm adding : {}",
        //     max_requested_pieces, pieces_to_create
        // );

        if pieces_to_create == 0 {
            return;
        }

        let new_cells_mass = cell_mass / (pieces_to_create.saturating_add(1) as Mass);
        let angle_increment = 1.6 * std::f32::consts::PI / pieces_to_create as f32;

        let mut directions = Vec::new();

        if let Some(angle_base) = split_dir {
            for i in 0..pieces_to_create {
                let angle = angle_base + angle_increment * i as f32;
                directions.push(Point {
                    x: angle.cos(),
                    y: angle.sin(),
                    radius: 0.0,
                });
            }
        } else {
            let target_direction = self.calculate_target_direction(); // A method to calculate and normalize the target direction
            debug!("we are here: {:?}", target_direction);
            directions.resize(pieces_to_create as usize, target_direction);
        }

        // Update the original cell mass before creating new cells
        self.cells[cell_index].set_mass(new_cells_mass);

        // Create new cells
        for direction in directions {
            let new_cell = Cell::new(
                cell_pos_x,
                cell_pos_y,
                new_cells_mass,
                SPLIT_CELL_SPEED, // Assuming a fixed speed for new cells
                false,            // Can move
                Some(direction),
            );
            self.cells.push(new_cell);
        }
        // Set last split time, assuming such a method exists
        self.set_last_split();
        self.recalculate_total_mass();
        self.recalculate_ratio();
    }

    // pub fn split_random(
    //     &mut self,
    //     cell_index: usize,
    //     max_requested_pieces: u8,
    //     default_player_mass: Mass,
    // ) {
    //     if cell_index >= self.cells.len() {
    //         return; // Early return if the cell index is out of bounds
    //     }

    //     let cell_mass = self.cells[cell_index].mass;
    //     if cell_mass < default_player_mass {
    //         return; // Cannot split cells smaller than the minimum cell mass.
    //     }

    //     let max_allowed_pieces = (cell_mass / default_player_mass) as u8;
    //     let pieces_to_create: usize = max_requested_pieces.min(max_allowed_pieces).into();
    //     if pieces_to_create <= 1 {
    //         return; // Not enough mass to split into more than one piece
    //     }

    //     let masses =
    //         self.distribute_mass_randomly(cell_mass, pieces_to_create, default_player_mass);

    //     let cell_position = (
    //         self.cells[cell_index].position.x,
    //         self.cells[cell_index].position.y,
    //     );
    //     // Update the original cell's mass to the last piece's mass before creating new cells
    //     self.cells[cell_index].set_mass(masses[pieces_to_create as usize - 1]);

    //     // Create new cells with the distributed masses
    //     for &mass in &masses[..pieces_to_create as usize - 1] {
    //         self.cells.push(Cell::new(
    //             cell_position.0,
    //             cell_position.1,
    //             mass,
    //             SPLIT_CELL_SPEED,
    //             true,
    //             None, // Assuming no direction needed or using default direction
    //         ));
    //     }

    //     self.set_last_split();
    // }

    //returns the direction based on the current position of the player and the target used with the mouse
    fn calculate_target_direction(&self) -> Point {
        let dx = self.target_x;
        let dy = self.target_y;
        Point {
            x: dx,
            y: dy,
            radius: 0.0,
        }
        .normalize()
    }

    // pub fn distribute_mass_randomly(
    //     &mut self,
    //     total_mass: TotalMass,
    //     pieces: usize,
    //     min_mass: Mass,
    // ) -> Vec<f32> {
    //     let mut rng = rand::thread_rng();
    //     let mut masses = vec![min_mass; pieces];
    //     let mut remaining_mass = total_mass - min_mass * pieces as f32;

    //     let mut i = 0;
    //     while remaining_mass > 0.0 {
    //         let add_mass = (rng.gen::<f32>() * remaining_mass)
    //             .floor()
    //             .min(remaining_mass);
    //         masses[i] += add_mass;
    //         remaining_mass -= add_mass;
    //         i = (i + 1) % pieces;
    //     }

    //     // Shuffle the array to randomize which cells get which mass
    //     for i in (1..masses.len()).rev() {
    //         let j = rng.gen_range(0..=i);
    //         masses.swap(i, j);
    //     }

    //     masses
    // }

    pub fn virus_split(
        &mut self,
        cell_indexes: &[usize],
        max_cells: usize,
        default_player_mass: Mass,
    ) {
        for &cell_index in cell_indexes {
            if cell_index < self.cells.len() {
                // Safety check to ensure the index is valid
                let max_requested_pieces =
                    max_cells.checked_sub(self.cells.len()).unwrap_or_default();

                if max_requested_pieces == 0 {
                    continue;
                }

                self.split_cell(
                    cell_index,
                    max_requested_pieces as u8,
                    default_player_mass,
                    Some(std::f32::consts::PI),
                ); // Example split angle of PI
            }
        }
    }

    //function triggered when player hits "space"
    pub fn user_split(&mut self, max_cells: usize, default_player_mass: Mass) {
        let cells_to_create = if self.cells.len() > max_cells / 2 {
            max_cells.checked_sub(self.cells.len()).unwrap_or_default()
        } else {
            self.cells.len()
        };

        if cells_to_create == 0 {
            return;
        }

        // Sort cells by mass in descending order
        self.cells.sort_by(|a, b| {
            b.mass
                .partial_cmp(&a.mass)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        info!(
            "player mass: {} - max_cells: {} - we want to create: {}",
            self.total_mass,
            max_cells,
            cells_to_create.min(self.cells.len())
        );

        for i in 0..cells_to_create.min(self.cells.len()) {
            if self.cells[i].mass < default_player_mass * 2 {
                break; // break because the cells are sorted by mass, the next cells are smaller than this one
            }
            self.split_cell(i, 1, default_player_mass, None);
        }
        self.recalculate_total_mass();
        info!("player mass after split: {}", self.total_mass);
    }

    fn sort_by_left(&mut self) {
        self.cells.sort_by(|a, b| {
            (a.position.x - a.position.radius)
                .partial_cmp(&(b.position.x - b.position.radius))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    pub fn merge_colliding_cells(&mut self) {
        self.enumerate_colliding_cells(|cell_a, cell_b| {
            if check_overlap(&cell_a.position, &cell_b.position) {
                cell_a.add_mass(cell_b.mass);
                cell_b.mark_for_removal();
            }
        });

        self.cells.retain(|cell| !cell.to_be_removed);
    }

    //loops through the players with a sort and sweep algorithm and checks for collision between them
    pub fn enumerate_colliding_cells<T>(&mut self, callback: T)
    where
        T: Fn(&mut Cell, &mut Cell),
    {
        self.sort_by_left();

        for i in 0..self.cells.len() - 1 {
            let (split_a, split_b) = self.cells.split_at_mut(i + 1);
            let cell_a = &mut split_a[i];

            for cell_b in split_b {
                if (cell_b.position.x - cell_b.position.radius)
                    > (cell_a.position.x + cell_a.position.radius)
                {
                    break;
                }

                if cell_a.position.distance(&cell_b.position)
                    <= (cell_a.position.radius + cell_b.position.radius)
                {
                    callback(cell_a, cell_b);
                }
            }
        }
    }

    //pushes cells when they are in contact in case the user is still split
    pub fn push_away_colliding_cells(&mut self) {
        self.enumerate_colliding_cells(|cell_a, cell_b| {
            let vector = Point {
                x: cell_b.position.x - cell_a.position.x,
                y: cell_b.position.y - cell_a.position.y,
                radius: 0.0,
            }
            .normalize()
            .scale(PUSHING_AWAY_SPEED);
            cell_a.position.x -= vector.x;
            cell_a.position.y -= vector.y;
            cell_b.position.x += vector.x;
            cell_b.position.y += vector.y;
        });
    }

    pub fn move_cells(
        &mut self,
        slow_base: f32,
        game_width: i32,
        game_height: i32,
        init_mass_log: f32,
    ) {
        let current_time = get_current_timestamp();

        if self.cells.len() > 1 {
            if let Some(time_to_merge) = self.time_to_merge {
                if current_time > time_to_merge {
                    self.merge_colliding_cells();
                } else {
                    self.push_away_colliding_cells();
                }
            }
        }

        let mut x_sum = 0.0;
        let mut y_sum = 0.0;

        let player_position = self.get_position_point();

        for cell in self.cells.iter_mut() {
            // TODO: remove the enumerate
            // Assume cell has a method `move` taking necessary parameters
            // info!("Cell {}", i);
            cell.move_cell(
                &player_position,
                self.target_x,
                self.target_y,
                slow_base,
                init_mass_log,
            );
            adjust_for_boundaries(
                &mut cell.position.x,
                &mut cell.position.y,
                cell.position.radius / 3.0,
                0.0,
                game_width as f32,
                game_height as f32,
            );

            x_sum += cell.position.x;
            y_sum += cell.position.y;
        }

        if !self.cells.is_empty() {
            self.x = x_sum / self.cells.len() as f32;
            self.y = y_sum / self.cells.len() as f32;
        }
    }

    pub fn check_for_collisions(
        player_a: &Player,
        player_b: &Player,
        player_a_index: usize,
        player_b_index: usize,
        callback: &dyn Fn((usize, usize), (usize, usize)),
    ) {
        for (cell_a_index, cell_a) in player_a.cells.iter().enumerate() {
            for (cell_b_index, cell_b) in player_b.cells.iter().enumerate() {
                let who_ate_who = check_who_ate_who(cell_a, cell_b);
                match who_ate_who {
                    0 => (),
                    1 => callback(
                        (player_b_index, cell_b_index),
                        (player_a_index, cell_a_index),
                    ),
                    2 => callback(
                        (player_a_index, cell_a_index),
                        (player_b_index, cell_b_index),
                    ),
                    _ => (),
                }
            }
        }
    }
}