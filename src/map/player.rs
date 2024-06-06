use rand::Rng;
use uuid::Uuid;
use super::point::Point;
use crate::utils::util::{mass_to_radius, lerp, lerp_deg, math_log, check_who_ate_who, are_colliding, check_overlap};
use crate::utils::game_logic::adjust_for_boundaries;
use std::time::{SystemTime, Duration};
const MIN_SPEED: f32 = 4.25;
const SPLIT_CELL_SPEED: f32 = 50.0;
const SPEED_DECREMENT: f32 = 0.8;
const MIN_DISTANCE: f32 = 50.0;
const PUSHING_AWAY_SPEED: f32 = 1.1;
const MERGE_TIMER: f32 = 20.0;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Cell {
    pub position: Point,
    mass: f32,
    speed: f32,
    can_move: bool,
    direction_shot: Option<Point>,
    img_url: String,
    to_be_removed: bool
}

impl Cell {
    pub fn new(x: f32, y: f32, mass: f32, speed: f32, can_move: bool, direction_shot: Option<Point>, img_url: String) -> Self {
        Self {
            position: Point{x: x, y: y, radius : mass_to_radius(mass)},
            mass,
            speed,
            can_move,
            direction_shot,
            img_url,
            to_be_removed: false
        }
    }

    pub fn mark_for_removal(&mut self) {
        self.to_be_removed = true;
    }

    fn set_mass(&mut self, new_mass: f32){
        self.mass = new_mass;
        self.recalculate_radius();
    }

    fn add_mass(&mut self, to_add: f32){
        self.set_mass(self.mass + to_add)
    }

    fn recalculate_radius(&mut self){
        self.position.radius = mass_to_radius(self.mass);
    }

    fn move_cell(&mut self, mouse_x: f32, mouse_y: f32, slow_base: f32, init_mass_log: f32) {
        let target_x = self.position.x - self.position.x + mouse_x;
        let target_y = self.position.y - self.position.y + mouse_y;
        let dist = (target_y.powi(2) + target_x.powi(2)).sqrt();
        let deg = target_y.atan2(target_x);
    
        let mut slow_down = 1.0;
        let (mut delta_y, mut delta_x);

        if self.can_move {
            if self.speed <= MIN_SPEED {
                slow_down = math_log(self.mass, Some(slow_base * 3.0)) - init_mass_log + 1.0;
            }
            delta_y = self.speed * deg.sin() / slow_down;
            delta_x = self.speed * deg.cos() / slow_down;
            if dist < (MIN_DISTANCE + self.position.radius) {
                let ratio = dist / (MIN_DISTANCE + self.position.radius);
                delta_y *= ratio;
                delta_x *= ratio;
            }
        } else {
            self.speed = lerp(self.speed, self.speed - SPEED_DECREMENT, 0.9); // Assuming lerp function exists
            if self.speed <= MIN_SPEED {
                self.speed = MIN_SPEED;
                self.can_move = true;
            }
    
            if let Some(direction_shot) = self.direction_shot {
                let not_dis = (direction_shot.y.powi(2) + direction_shot.x.powi(2)).sqrt();
                let not_deg = direction_shot.y.atan2(direction_shot.x);
                let real_deg = lerp_deg(not_deg, deg, 0.08 * SPLIT_CELL_SPEED / self.speed); // Assuming lerp_deg function exists
    
                delta_y = self.speed * real_deg.sin();
                delta_x = self.speed * real_deg.cos();
                if not_dis < MIN_DISTANCE + self.position.radius {
                    let ratio = not_dis / (MIN_DISTANCE + self.position.radius) / slow_down;
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
    }
}

#[derive(Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Player {
    pub id: Uuid,
    pub hue: u16,
    name: Option<String>,
    admin: bool,
    pub screen_width: Option<f32>,
    pub screen_height: Option<f32>,
    pub time_to_merge: Option<std::time::SystemTime>,
    img_url: String,
    pub last_heartbeat: std::time::SystemTime,
    // Properties to be initialized later
    pub cells: Vec<Cell>,
    pub mass_total: f32,
    pub x: f32,
    pub y: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub ratio: f32,
}

impl Player {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            hue: rand::random::<u16>() % 360,
            name: None,
            admin: false,
            screen_width: None,
            screen_height: None,
            time_to_merge: None,
            img_url: String::new(),
            last_heartbeat: std::time::SystemTime::now(),
            // Initial states for properties to be initialized later
            cells: Vec::new(),
            mass_total: 0.0,
            x: 0.0,
            y: 0.0,
            target_x: 0.0,
            target_y: 0.0,
            ratio: 1.0,
        }
    }

    pub fn init(&mut self, position: Point, default_player_mass: f32, img_url: &str) {
        self.cells = vec![
            Cell::new(
                position.x,
                position.y,
                default_player_mass,
                MIN_SPEED,
                true, // Assuming cells can move when initialized
                None, // No direction shot specified, assuming it is not required at initialization
                img_url.to_string()
            )
        ];
        self.mass_total = default_player_mass;
        self.x = position.x;
        self.y = position.y;
        self.target_x = 0.0;
        self.target_y = 0.0;
        self.ratio = 1.0;
    }

    pub fn client_provided_data(&mut self, img_url: String, name: String, screen_width: f32, screen_height: f32) {
        self.img_url = img_url;
        self.name = Some(name);
        self.screen_width = Some(screen_width);
        self.screen_height = Some(screen_height);
        self.set_last_heartbeat();
    }

    pub fn lose_mass_if_needed(&mut self, mass_loss_rate: f32, default_player_mass: f32, min_mass_loss: f32) {
        for i in 0..self.cells.len() {
            if self.cells[i].mass * (1.0 - (mass_loss_rate / 1000.0)) > default_player_mass && self.mass_total > min_mass_loss {
                let mass_loss = self.cells[i].mass * (mass_loss_rate / 1000.0);
                self.change_cell_mass(i as u8, -mass_loss);
            }
        }
    }
    
    pub fn set_last_heartbeat(&mut self) {
        self.last_heartbeat = std::time::SystemTime::now();
    }

    pub fn set_last_split(&mut self) {
        let merge_duration = Duration::from_secs_f64((1000.0 * MERGE_TIMER + self.mass_total / 100.0).into());
        self.time_to_merge = Some(SystemTime::now() + merge_duration);
    }

    pub fn change_cell_mass(&mut self, cell_index: u8, mass_diff: f32) {
        self.cells[cell_index as usize].add_mass(mass_diff);
        self.mass_total += mass_diff;
    }    

    pub fn remove_cell(&mut self, cell_index: u8) -> bool {
        self.mass_total -= self.cells[cell_index as usize].mass;
        self.cells.remove(cell_index as usize);
        self.cells.is_empty()
    }

    pub fn split_cell(&mut self, cell_index: usize, max_requested_pieces: u8, default_player_mass: f32, split_dir: Option<f32>) {
        if cell_index >= self.cells.len() {
            return; // Early return if the cell index is out of bounds
        }

        // Extract all necessary data from the cell
        let (cell_pos_x, cell_pos_y, cell_mass, cell_img_url) = {
            let cell = &self.cells[cell_index];
            (cell.position.x, cell.position.y, cell.mass, cell.img_url.clone())
        };

        let max_allowed_pieces = (cell_mass / default_player_mass).floor() as u8;
        let pieces_to_create = max_requested_pieces.min(max_allowed_pieces);

        if pieces_to_create == 0 {
            return;
        }

        let new_cells_mass = cell_mass / pieces_to_create as f32;
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
            directions.resize(pieces_to_create as usize, target_direction);
        }

        // Update the original cell mass before creating new cells
        self.cells[cell_index].set_mass(new_cells_mass);

        // Create new cells
        for direction in directions {
            self.cells.push(Cell::new(
                cell_pos_x, 
                cell_pos_y, 
                new_cells_mass, 
                30.0,  // Assuming a fixed speed for new cells
                true,  // Can move
                Some(direction), 
                cell_img_url.clone()
            ));
        }

        // Set last split time, assuming such a method exists
        self.set_last_split();
    }

    pub fn split_random(&mut self, cell_index: usize, max_requested_pieces: u8, default_player_mass: f32) {
        if cell_index >= self.cells.len() {
            return; // Early return if the cell index is out of bounds
        }

        let cell_mass = self.cells[cell_index].mass;
        if cell_mass < default_player_mass {
            return; // Cannot split cells smaller than the minimum cell mass.
        }

        let max_allowed_pieces = (cell_mass / default_player_mass).floor() as u8;
        let pieces_to_create: usize = max_requested_pieces.min(max_allowed_pieces).into();
        if pieces_to_create <= 1 {
            return; // Not enough mass to split into more than one piece
        }

        let masses = self.distribute_mass_randomly(cell_mass, pieces_to_create, default_player_mass);
        
        let cell_position = (self.cells[cell_index].position.x, self.cells[cell_index].position.y);
        let cell_img_url = self.cells[cell_index].img_url.clone(); // Assuming img_url needs to be cloned

        // Update the original cell's mass to the last piece's mass before creating new cells
        self.cells[cell_index].set_mass(masses[pieces_to_create as usize - 1]);

        // Create new cells with the distributed masses
        for &mass in &masses[..pieces_to_create as usize - 1] {
            self.cells.push(Cell::new(
                cell_position.0, 
                cell_position.1, 
                mass, 
                SPLIT_CELL_SPEED * 10.0 / mass, 
                true, 
                None, // Assuming no direction needed or using default direction
                cell_img_url.clone()
            ));
        }

        self.set_last_split();
    }

    fn calculate_target_direction(&self) -> Point {
        let dx = self.target_x - self.x;
        let dy = self.target_y - self.y;
        let norm = (dx.powi(2) + dy.powi(2)).sqrt();
        Point { x: dx / norm, y: dy / norm, radius: 0.0 }
    }

    pub fn distribute_mass_randomly(&mut self, total_mass: f32, pieces: usize, min_mass: f32) -> Vec<f32> {
        let mut rng = rand::thread_rng();
        let mut masses = vec![min_mass; pieces];
        let mut remaining_mass = total_mass - min_mass * pieces as f32;

        let mut i = 0;
        while remaining_mass > 0.0 {
            let add_mass = (rng.gen::<f32>() * remaining_mass).floor().min(remaining_mass);
            masses[i] += add_mass;
            remaining_mass -= add_mass;
            i = (i + 1) % pieces;
        }

        // Shuffle the array to randomize which cells get which mass
        for i in (1..masses.len()).rev() {
            let j = rng.gen_range(0..=i);
            masses.swap(i, j);
        }

        masses
    }

    pub fn virus_split(&mut self, cell_indexes: &[usize], max_cells: usize, default_player_mass: f32) {
        for &cell_index in cell_indexes {
            if cell_index < self.cells.len() { // Safety check to ensure the index is valid
                let max_requested_pieces = max_cells.saturating_sub(self.cells.len()) + 1;
                self.split_cell(cell_index, max_requested_pieces as u8, default_player_mass, Some(std::f32::consts::PI)); // Example split angle of PI
            }
        }
    }

    pub fn user_split(&mut self, max_cells: usize, default_player_mass: f32) {
        let cells_to_create = if self.cells.len() > max_cells / 2 {
            max_cells.saturating_sub(self.cells.len()) + 1
        } else {
            self.cells.len()
        };

        // Sort cells by mass in descending order
        self.cells.sort_by(|a, b| b.mass.partial_cmp(&a.mass).unwrap_or(std::cmp::Ordering::Equal));

        for i in 0..cells_to_create.min(self.cells.len()) {
            self.split_cell(i, 2, default_player_mass, None); // No specific split direction provided
        }
    }

    fn sort_by_left(&mut self) {
        self.cells.sort_by(|a, b| {
            (a.position.x - a.position.radius)
                .partial_cmp(&(b.position.x - b.position.radius))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    pub fn enumerate_colliding_cells<F>(&mut self, mut action: F)
    where
        F: FnMut(usize, usize),
    {
        self.sort_by_left();
        let len = self.cells.len();
        for i in 0..len {
            for j in (i + 1)..len {
                if self.cells[j].position.x - self.cells[j].position.radius > self.cells[i].position.x + self.cells[i].position.radius {
                    break;
                }
                // Inlining collision check for example purposes
                if (self.cells[i].position.x - self.cells[j].position.x).powi(2) + 
                (self.cells[i].position.y - self.cells[j].position.y).powi(2) <
                (self.cells[i].position.radius + self.cells[j].position.radius).powi(2) {
                    action(i, j);
                }
            }
        }
        self.cells.retain(|cell| !cell.to_be_removed);
    }



    pub fn merge_colliding_cells(&mut self) {
        let time_to_merge = self.time_to_merge.map_or(false, |tm| tm <= SystemTime::now());
        if !time_to_merge {
            return;
        }
    
        let len = self.cells.len();
        for i in 0..len {
            let (before, after) = self.cells.split_at_mut(i + 1);
            let cell_a = &mut before[i];
            for j in 0..after.len() {
                let cell_b = &mut after[j];
                if check_overlap(&cell_a.position, &cell_b.position) {
                    cell_a.add_mass(cell_b.mass);
                    cell_b.mark_for_removal();
                }
            }
        }
        self.cells.retain(|cell| !cell.to_be_removed);
    }

    pub fn push_away_colliding_cells(&mut self, time_to_merge: bool) {
        if !time_to_merge {
            return;
        }

        let len = self.cells.len();
        for i in 0..len {
            let (before, after) = self.cells.split_at_mut(i + 1);
            let cell_a = &mut before[i];
            for j in 0..after.len() {
                let cell_b = &mut after[j];
                if are_colliding(cell_a, cell_b) {
                    let vector = Point{x: cell_b.position.x - cell_a.position.x, y: cell_b.position.y - cell_a.position.y, radius: 0.0}.normalize().scale(PUSHING_AWAY_SPEED);
                    cell_a.position.x -= vector.x;
                    cell_a.position.y -= vector.y;
                    cell_b.position.x += vector.x;
                    cell_b.position.y += vector.y;
                }
            }
        }
    }

    fn move_cells(&mut self, slow_base: f32, game_width: i32, game_height: i32, init_mass_log: f32) {
        let current_time = std::time::SystemTime::now();

        if self.cells.len() > 1 {
            if let Some(time_to_merge) = self.time_to_merge {
                if current_time > time_to_merge {
                    self.merge_colliding_cells();
                } else {
                    self.push_away_colliding_cells(current_time <= time_to_merge);
                }
            }
        }

        let mut x_sum = 0.0;
        let mut y_sum = 0.0;

        for cell in &mut self.cells {
            // Assume cell has a method `move` taking necessary parameters
            cell.move_cell(self.target_x, self.target_y, slow_base, init_mass_log);
            adjust_for_boundaries(&mut cell.position.x, &mut cell.position.y, cell.position.radius / 3.0, 0.0, game_width as f32, game_height as f32);

            x_sum += cell.position.x;
            y_sum += cell.position.y;
        }

        if !self.cells.is_empty() {
            self.x = x_sum / self.cells.len() as f32;
            self.y = y_sum / self.cells.len() as f32;
        }
    }

    fn check_for_collisions(player_a: &Player, player_b: &Player, player_a_index: usize, player_b_index: usize, callback: &dyn Fn((usize, usize), (usize, usize))) {
        for (cell_a_index, cell_a) in player_a.cells.iter().enumerate() {
            for (cell_b_index, cell_b) in player_b.cells.iter().enumerate() {
                let who_ate_who = check_who_ate_who(cell_a, cell_b);
                match who_ate_who {
                    0 => (),
                    1 => callback((player_b_index, cell_b_index), (player_a_index, cell_a_index)),
                    2 => callback((player_a_index, cell_a_index), (player_b_index, cell_b_index)),
                    _ => (),
                }
            }
        }
    }

}

pub struct PlayerManager{
    pub players: Vec<Player>,
}

impl PlayerManager {
    pub fn new() -> Self {
        PlayerManager {
            players: Vec::new(),
        }
    }

    fn push_new(&mut self, player: Player) {
        self.players.push(player);
    }

    fn find_index_by_id(&self, id: Uuid) -> Option<usize> {
        self.players.iter().position(|p| p.id == id)
    }

    pub fn remove_player_by_id(&mut self, id: Uuid) {
        if let Some(index) = self.find_index_by_id(id) {
            self.players.remove(index);
        }
    }

    fn shrink_cells(&mut self, mass_loss_rate: f32, default_player_mass: f32, min_mass_loss: f32) {
        for player in &mut self.players {
            player.lose_mass_if_needed(mass_loss_rate, default_player_mass, min_mass_loss);
        }
    }

    fn handle_collisions(&self, callback: &dyn Fn((usize, usize), (usize, usize))) {
        for (player_a_index, player_a) in self.players.iter().enumerate() {
            for (player_b_index, player_b) in self.players.iter().enumerate().skip(player_a_index + 1) {
                Player::check_for_collisions(player_a, player_b, player_a_index, player_b_index, callback);
            }
        }
    }

    fn get_top_players(&self) -> Vec<(Uuid, String)> {
        // First, clone the players to a mutable local variable to sort
        let mut sorted_players = self.players.clone();
        sorted_players.sort_by(|a, b| 
            b.cells.iter().map(|c| c.mass).sum::<f32>()
                .partial_cmp(&a.cells.iter().map(|c| c.mass).sum::<f32>())
                .unwrap_or(std::cmp::Ordering::Equal)
        );
    
        // Now collect the top 10 players, safely handling Option<String>
        sorted_players.iter()
            .filter_map(|p| {
                Some((
                    p.id.clone(), // If id is None, the player will be skipped
                    p.name.clone()? // If name is None, the player will also be skipped
                ))
            })
            .take(10)
            .collect()
    }

    pub fn get_total_mass(&self) -> f32 {
        self.players.iter().map(|p| p.mass_total).sum()
    }
}
