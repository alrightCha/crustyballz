use super::point::Point;
use crate::config::get_current_config;
use crate::recv_messages::Target;
use crate::send_messages::PlayerData;
use crate::utils::game_logic::adjust_for_boundaries;
use crate::utils::quad_tree::Rectangle;
use crate::utils::util::{
    are_colliding, check_overlap, check_who_ate_who, get_current_timestamp, lerp, lerp_deg,
    mass_to_radius, math_log,
};
use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use socketioxide::socket::Sid;
use std::time::{Duration, SystemTime};
use uuid::Uuid;
const MIN_SPEED: f32 = 4.25;
const SPLIT_CELL_SPEED: f32 = 50.0;
const SPEED_DECREMENT: f32 = 0.8;
const MIN_DISTANCE: f32 = 50.0;
const PUSHING_AWAY_SPEED: f32 = 1.1;
const MERGE_TIMER: f32 = 20.0;

#[derive(Serialize)]
pub struct CellData {
    pub canMove: bool,
    pub imgUrl: Option<String>,
    pub mass: f32,
    pub speed: f32,
    pub radius: f32,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Cell {
    pub position: Point,
    pub mass: f32,
    speed: f32,
    can_move: bool,
    direction_shot: Option<Point>,
    img_url: Option<String>,
    to_be_removed: bool,
}

impl Serialize for Cell {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        CellData {
            canMove: self.can_move,
            imgUrl: self.img_url.clone(),
            mass: self.mass,
            radius: self.position.radius,
            speed: self.speed,
            x: self.position.x,
            y: self.position.y,
        }
        .serialize(serializer)
    }
}

impl Cell {
    pub fn new(
        x: f32,
        y: f32,
        mass: f32,
        speed: f32,
        can_move: bool,
        direction_shot: Option<Point>,
        img_url: Option<String>,
    ) -> Self {
        Self {
            position: Point {
                x: x,
                y: y,
                radius: mass_to_radius(mass),
            },
            mass,
            speed,
            can_move,
            direction_shot,
            img_url,
            to_be_removed: false,
        }
    }

    pub fn mark_for_removal(&mut self) {
        self.to_be_removed = true;
    }

    fn set_mass(&mut self, new_mass: f32) {
        self.mass = new_mass.max(0.0);
        self.recalculate_radius();
    }

    pub fn remove_mass(&mut self, to_remove: f32) {
        self.set_mass(self.mass - to_remove)
    }

    pub fn add_mass(&mut self, to_add: f32) {
        self.set_mass(self.mass + to_add)
    }

    fn recalculate_radius(&mut self) {
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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Player {
    pub id: Uuid,
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
    pub total_mass: f32,
    pub x: f32,
    pub y: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub ratio: f32,
}

impl Player {
    pub fn new(socket_id: Sid) -> Self {
        Self {
            id: Uuid::new_v4(),
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
            total_mass: get_current_config().default_player_mass,
            x: 0.0,
            y: 0.0,
            target_x: 0.0,
            target_y: 0.0,
            ratio: 1.03,
        }
    }

    pub fn init(
        &mut self,
        position: Point,
        default_player_mass: f32,
        name: Option<String>,
        screen_width: f32,
        screen_height: f32,
        img_url: Option<String>,
    ) {
        self.cells = vec![Cell::new(
            position.x,
            position.y,
            default_player_mass,
            MIN_SPEED,
            true, // Assuming cells can move when initialized
            None, // No direction shot specified, assuming it is not required at initialization
            img_url.clone(),
        )];
        self.name = name;
        self.img_url = img_url.clone();
        self.total_mass = default_player_mass;
        self.x = position.x;
        self.y = position.y;
        self.screen_width = screen_width;
        self.screen_height = screen_height;
        self.target_x = 0.0;
        self.target_y = 0.0;
        self.ratio = 1.0;
    }

    pub fn get_position_point(&self) -> Point {
        Point {
            x: self.x,
            y: self.y,
            radius: mass_to_radius(self.total_mass),
        }
    }

    pub fn get_target_point(&self) -> Point {
        Point {
            x: self.target_x,
            y: self.target_y,
            radius: mass_to_radius(self.total_mass),
        }
    }

    pub fn recalculate_ratio(&mut self) {
        let new_val = lerp(
            self.ratio,
            0.8 - 0.2 * (self.total_mass / 500.0).ln() - 0.3 * (self.cells.len() as f32) / 18.0,
            0.1,
        );
        if new_val > 0.3 {
            self.ratio = new_val;
        } else {
            self.ratio = 0.3;
        }
    }

    pub fn recalculate_total_mass(&mut self) {
        self.total_mass = self.cells.iter().map(|c| c.mass).sum();
    }

    pub fn generate_player_data(&self) -> PlayerData {
        PlayerData {
            admin: false,
            lastHeartbeat: self.last_heartbeat,
            name: self.name.clone().unwrap_or_default(),
            screenHeight: 1920.0,
            screenWidth: 1080.0,
            target: Target {
                x: self.target_x,
                y: self.target_y,
            },
            timeToMerge: self.time_to_merge,
            cells: self.cells.clone(),
            ratio: self.ratio,
            hue: self.hue,
            id: self.id,
            imgUrl: self.img_url.clone(),
            massTotal: self.total_mass,
            x: self.x,
            y: self.y,
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
        default_player_mass: f32,
        min_mass_loss: f32,
    ) {
        for i in 0..self.cells.len() {
            if self.cells[i].mass * (1.0 - (mass_loss_rate / 1000.0)) > default_player_mass
                && self.total_mass > min_mass_loss
            {
                let mass_loss = self.cells[i].mass * (mass_loss_rate / 1000.0);
                self.change_cell_mass(i as u8, -mass_loss);
            }
        }
    }

    pub fn set_last_heartbeat(&mut self) {
        self.last_heartbeat = get_current_timestamp();
    }

    pub fn set_last_split(&mut self) {
        let merge_duration = 1000.0 * MERGE_TIMER + self.total_mass / 100.0;
        self.time_to_merge = Some(get_current_timestamp() + merge_duration as i64);
    }

    pub fn change_cell_mass(&mut self, cell_index: u8, mass_diff: f32) {
        self.cells[cell_index as usize].add_mass(mass_diff);
        self.total_mass += mass_diff;
    }

    pub fn remove_cell(&mut self, cell_index: u8) -> bool {
        self.total_mass -= self.cells[cell_index as usize].mass;
        self.cells.remove(cell_index as usize);
        self.cells.is_empty()
    }

    pub fn split_cell(
        &mut self,
        cell_index: usize,
        max_requested_pieces: u8,
        default_player_mass: f32,
        split_dir: Option<f32>,
    ) {
        if cell_index >= self.cells.len() {
            return; // Early return if the cell index is out of bounds
        }

        // Extract all necessary data from the cell
        let (cell_pos_x, cell_pos_y, cell_mass, cell_img_url) = {
            let cell = &self.cells[cell_index];
            (
                cell.position.x,
                cell.position.y,
                cell.mass,
                cell.img_url.clone(),
            )
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
                30.0, // Assuming a fixed speed for new cells
                true, // Can move
                Some(direction),
                cell_img_url.clone(),
            ));
        }

        // Set last split time, assuming such a method exists
        self.set_last_split();
    }

    pub fn split_random(
        &mut self,
        cell_index: usize,
        max_requested_pieces: u8,
        default_player_mass: f32,
    ) {
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

        let masses =
            self.distribute_mass_randomly(cell_mass, pieces_to_create, default_player_mass);

        let cell_position = (
            self.cells[cell_index].position.x,
            self.cells[cell_index].position.y,
        );
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
                cell_img_url.clone(),
            ));
        }

        self.set_last_split();
    }

    //returns the direction based on the current position of the player and the target used with the mouse
    fn calculate_target_direction(&self) -> Point {
        let dx = self.target_x - self.x;
        let dy = self.target_y - self.y;
        let norm = (dx.powi(2) + dy.powi(2)).sqrt();
        Point {
            x: dx / norm,
            y: dy / norm,
            radius: 0.0,
        }
    }

    pub fn distribute_mass_randomly(
        &mut self,
        total_mass: f32,
        pieces: usize,
        min_mass: f32,
    ) -> Vec<f32> {
        let mut rng = rand::thread_rng();
        let mut masses = vec![min_mass; pieces];
        let mut remaining_mass = total_mass - min_mass * pieces as f32;

        let mut i = 0;
        while remaining_mass > 0.0 {
            let add_mass = (rng.gen::<f32>() * remaining_mass)
                .floor()
                .min(remaining_mass);
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

    pub fn virus_split(
        &mut self,
        cell_indexes: &[usize],
        max_cells: usize,
        default_player_mass: f32,
    ) {
        for &cell_index in cell_indexes {
            if cell_index < self.cells.len() {
                // Safety check to ensure the index is valid
                let max_requested_pieces = max_cells.saturating_sub(self.cells.len()) + 1;
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
    pub fn user_split(&mut self, max_cells: usize, default_player_mass: f32) {
        let cells_to_create = if self.cells.len() > max_cells / 2 {
            max_cells.saturating_sub(self.cells.len()) + 1
        } else {
            self.cells.len()
        };

        // Sort cells by mass in descending order
        self.cells.sort_by(|a, b| {
            b.mass
                .partial_cmp(&a.mass)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

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

    //loops through the players with a sort and sweep algorithm and checks for collision between them
    pub fn enumerate_colliding_cells<F>(&mut self, mut action: F)
    where
        F: FnMut(usize, usize),
    {
        self.sort_by_left();
        let len = self.cells.len();
        for i in 0..len {
            for j in (i + 1)..len {
                if self.cells[j].position.x - self.cells[j].position.radius
                    > self.cells[i].position.x + self.cells[i].position.radius
                {
                    break;
                }
                // Inlining collision check for example purposes
                if (self.cells[i].position.x - self.cells[j].position.x).powi(2)
                    + (self.cells[i].position.y - self.cells[j].position.y).powi(2)
                    < (self.cells[i].position.radius + self.cells[j].position.radius).powi(2)
                {
                    action(i, j);
                }
            }
        }
        self.cells.retain(|cell| !cell.to_be_removed);
    }

    pub fn merge_colliding_cells(&mut self) {
        let time_to_merge = match self.time_to_merge {
            Some(tm) => tm <= get_current_timestamp(),
            None => false,
        };

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

    //pushes cells when they are in contact in case the user is still split
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
                if are_colliding(&cell_a.position, &cell_b.position) {
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
                }
            }
        }
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
                    self.push_away_colliding_cells(current_time <= time_to_merge);
                }
            }
        }

        let mut x_sum = 0.0;
        let mut y_sum = 0.0;

        for cell in &mut self.cells {
            // Assume cell has a method `move` taking necessary parameters
            cell.move_cell(self.target_x, self.target_y, slow_base, init_mass_log);
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
