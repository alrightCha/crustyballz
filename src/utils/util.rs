 

use super::quad_tree::Rectangle;
use crate::map::player::{Cell, Player};
use crate::map::point::Point;
use rand::Rng;
use std::f32::consts::PI;
use uuid::Uuid;

//checks if the nickname of the player is valid and can be used within the gam e
pub fn valid_nick(nickname: &str) -> bool {
    let regex = regex::Regex::new(r"^\w*").unwrap();
    regex.is_match(nickname)
}

//used to determine the radius of the cell / food / virus / massFood, using its mass 
pub fn mass_to_radius(mass: f32) -> f32 {
    4.0 + (mass.sqrt() * 6.0)
}

//used to not see an immediate change, sort of a smoothing function 
pub fn lerp(start: f32, end: f32, factor: f32) -> f32 {
    let difference = end - start;
    start + difference * factor
}

//same as above but for degrees

pub fn lerp_deg(start: f32, end: f32, factor: f32) -> f32 {
    let mut difference = end - start;
    if difference < -PI { difference += 2.0 * PI };
    if difference > PI { difference -= 2.0 * PI };
    start + difference * factor
}

//returns the player ratio to know how far the screen should be zoomed out / zoomed in. Mostly between 0.1 and 1.5
pub fn get_ratio(player: &Player, ratio: &mut f32){
    let new_val = lerp(player.ratio, 0.8 - 0.2 * (player.mass_total / 500.0).ln() - 0.3 * (player.cells.len() as f32) / 18.0, 0.1);
    if new_val > 0.3 {
        *ratio = new_val;
    } else {
        *ratio = 0.3;
    }
}

//returns a rectangle representing the user's screen on the map 
pub fn get_visible_area(player: &Player, ratio: &mut f32) -> Rectangle {
    get_ratio(player, ratio);

    let half_width = (player.screen_width.unwrap_or_default() as f32 / player.ratio) / 2.0;
    let half_height = (player.screen_height.unwrap_or_default() as f32 / player.ratio) / 2.0;

    Rectangle::new(player.x - half_width, player.y - half_height, half_width * 2.0, half_height * 2.0)
}

pub fn math_log(n: f32, base: Option<f32>) -> f32 {
    let base_log = base.map_or(1.0, |b| b.ln());
    n.ln() / base_log
}

//returns distance between two points 
pub fn get_distance(p1: &Point, p2: &Point) -> f32 {
    ((p2.x - p1.x).powi(2) + (p2.y - p1.y).powi(2)).sqrt() - p1.radius - p2.radius
}

pub fn random_in_range(from: f32, to: f32) -> f32 {
    let mut rng = rand::thread_rng();
    rng.gen_range(from..to)
}

pub fn get_position(is_uniform: bool, radius: f32, uniform_positions: Option<&[Point]>) -> Point {
    if is_uniform {
        // Check if we have some positions to consider for uniform positioning
        if let Some(positions) = uniform_positions {
            uniform_position(positions, radius)
        } else {
            // If no positions are provided, fall back to random positioning
            random_position(radius)
        }
    } else {
        random_position(radius)
    }
}

//generates a random point to use its x and y values and know a position on the map 
fn random_position(radius: f32) -> Point {
    Point {
        x: random_in_range(radius, 10000.0 - radius) as f32,
        y: random_in_range(radius, 10000.0 - radius) as f32,
        radius: radius as f32,
    }
}

//makes sure that the posiiton is not below a player, used to determine the spawning point of a player in accordance to the rest of the players
fn uniform_position(points: &[Point], radius: f32) -> Point {
    let mut max_distance = 0.0;
    let mut best_candidate = random_position(radius);
    let number_of_candidates = 10;

    if points.is_empty() {
        return best_candidate;
    }

    for _ in 0..number_of_candidates {
        let mut min_distance = f32::INFINITY;
        let candidate = random_position(radius);

        for point in points {
            let distance = get_distance(&candidate, point);
            if distance < min_distance {
                min_distance = distance;
            }
        }

        if min_distance > max_distance {
            best_candidate = candidate;
            max_distance = min_distance;
        } else {
            return random_position(radius);
        }
    }

    best_candidate
}

pub fn find_index(arr: &[Player], id: Uuid) -> Option<usize> {
    arr.iter().enumerate().rev().find_map(|(i, player)| {
        if player.id == id {
            Some(i)
        } else {
            None
        }
    })
}

//cheks which cell ate the other one by knowing which one is bigger, if there is an overlap between the cells 
pub fn check_who_ate_who(cell_a: &Cell, cell_b: &Cell) -> u8 {
    if check_overlap(&cell_a.position, &cell_b.position) {
        let min_cell_rad = f32::min(cell_a.position.radius, cell_b.position.radius);
        if min_cell_rad == cell_a.position.radius {
           return 2;
        } else {
            return 1;
        }
    }
    return 0;
}


//cheks if the entity (point) is visible on the user's screen 
pub fn is_visible_entity(position_a: Point, player: Player) -> bool {
    if let Some(width) = player.screen_width{
        if let Some(height) = player.screen_height{
            return test_rectangle_rectangle(position_a.x, position_a.y, position_a.radius, player.x, player.y, width, height)
        }
    }
    return false
}

//takes the screen and an object and checks if they enterfere to know if it should be displayed or not 
fn test_rectangle_rectangle(center_x_a: f32, center_y_a: f32, radius: f32, 
    center_x_b: f32, center_y_b: f32, width_b: f32, height_b: f32) -> bool {
    let half_width_a = radius / 2.0;
    let half_height_a = radius / 2.0;
    let half_width_b = width_b / 2.0;
    let half_height_b = height_b / 2.0;

    center_x_a + half_width_a > center_x_b - half_width_b &&
    center_x_a - half_width_a < center_x_b + half_width_b &&
    center_y_a + half_height_a > center_y_b - half_height_b &&
    center_y_a - half_height_a < center_y_b + half_height_b
}


//checks if two cells are colliding or not (touching borders at least)
pub fn are_colliding(cell1: &Cell, cell2: &Cell) -> bool {
    // Simple collision detection logic (circle-circle collision)
    let dx = cell1.position.x - cell2.position.x;
    let dy = cell1.position.y - cell2.position.y;
    let distance = (dx * dx + dy * dy).sqrt();
    distance < (cell1.position.radius + cell2.position.radius)
}

//returns true if a cell is covering more than 60% of another cell 
pub fn check_overlap(circle_a: &Point, circle_b: &Point) -> bool {
    let dx = circle_a.x - circle_b.x;
    let dy = circle_a.y - circle_b.y;
    let distance = f32::sqrt(dx * dx + dy * dy);

    let r1 = circle_a.radius;
    let r2 = circle_b.radius;
    let r_min = r1.min(r2);
    let r_max = r1.max(r2);

    // Check for complete containment
    if distance + r_min <= r_max {
        return true; // One circle is completely inside the other
    }

    // Calculate intersection area if circles are partially overlapping
    if distance < r1 + r2 && distance > f32::abs(r1 - r2) {
        let angle1 = f32::acos((distance * distance + r1 * r1 - r2 * r2) / (2.0 * distance * r1));
        let angle2 = f32::acos((distance * distance + r2 * r2 - r1 * r1) / (2.0 * distance * r2));
        let part1 = r1 * r1 * angle1;
        let part2 = r2 * r2 * angle2;
        let part3 = 0.5 * f32::sqrt((-distance + r1 + r2) * (distance + r1 - r2) * (distance - r1 + r2) * (distance + r1 + r2));
        let intersection_area = part1 + part2 - part3;

        // Check if the intersection area is at least 60% of the area of the smaller circle
        if intersection_area >= 0.6 * PI * r_min * r_min {
            return true;
        }
    }

    false
}

//generates a random color for the food and the players 
pub fn random_color() -> (String, String) {
    let mut rng = rand::thread_rng();
    let random_number = rng.gen_range(0..(1 << 24));
    let color = format!("#{:06x}", random_number);

    let r = (((random_number >> 16) & 0xFF) as u8).saturating_sub(32);
    let g = (((random_number >> 8) & 0xFF) as u8).saturating_sub(32);
    let b = ((random_number & 0xFF) as u8).saturating_sub(32);
    let border_color = format!("#{:06x}", (r as u32) << 16 | (g as u32) << 8 | (b as u32));

    (color, border_color)
}