use crate::map::cell::Cell;
use crate::map::player::Player;
use crate::map::point::Point;
use chrono::Utc;
use lazy_static::lazy_static;
use rand::distributions::uniform::{SampleRange, SampleUniform};
use rand::Rng;
use regex::Regex;
use std::f32::consts::PI;
use uuid::Uuid;

use super::consts::{Mass, TotalMass};
use super::id::PlayerID;

lazy_static! {
    // Regex matches any string of 0 to 14 word characters.
    static ref REGEX_VALID_NICK: Regex = Regex::new(r"^\w{0,14}$").unwrap();
}

/// Checks if the nickname of the player is valid and can be used within the game
pub fn valid_nick(nickname: &str) -> bool {
    REGEX_VALID_NICK.is_match(nickname) && nickname.len() < 15
}

pub fn get_current_timestamp() -> i64 {
    Utc::now().timestamp()
}

pub fn get_current_timestamp_micros() -> i64 {
    Utc::now().timestamp_micros()
}

pub fn total_mass_to_radius(mass: TotalMass) -> f32 {
    4.0 + ((mass as f32).sqrt() * 6.0)
}

pub fn mass_to_radius(mass: Mass) -> f32 {
    4.0 + ((mass as f32).sqrt() * 6.0)
}

//used to not see an immediate change, sort of a smoothing function
pub fn lerp(start: f32, end: f32, factor: f32) -> f32 {
    let difference = end - start;
    start + difference * factor
}

pub fn lerp_move(start: f32, end: f32, factor: f32) -> f32 {
    let difference = start - end;
    start - difference * factor
}
//same as above but for degrees

pub fn lerp_deg(start: f32, end: f32, factor: f32) -> f32 {
    let mut difference = end - start;
    if difference < -PI {
        difference += 2.0 * PI
    };
    if difference > PI {
        difference -= 2.0 * PI
    };
    start + difference * factor
}

pub fn math_log(n: f32, base: Option<f32>, min_speed: f32) -> f32 {
    let base_log = base.map_or(1.0, |b| b.ln());
    let x = n / min_speed;
    base_log / x.ln()
}

//returns distance between two points
pub fn get_distance(p1: &Point, p2: &Point) -> f32 {
    ((p2.x - p1.x).powi(2) + (p2.y - p1.y).powi(2)).sqrt() - p1.radius - p2.radius
}

pub fn random_in_range<R, T>(range: R) -> T
    where
    T: SampleUniform,
    R: SampleRange<T>
    {
    let mut rng = rand::thread_rng();
    rng.gen_range(range)
}

pub fn create_random_position_in_range(max_x: f32, max_y: f32) -> Point {
    Point {
        x: random_in_range(0.0..max_x),
        y: random_in_range(0.0..max_y),
        radius: 0.0,
    }
}

pub fn create_random_number_u32(from: u16, to: u16) -> u16 {
    rand::thread_rng().gen_range(from..to)
}

pub fn create_random_position(
    is_uniform: bool,
    radius: f32,
    uniform_positions: Option<&[Point]>,
) -> Point {
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
        x: random_in_range(radius..10000.0 - radius),
        y: random_in_range(radius..10000.0 - radius),
        radius,
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

pub fn find_index(arr: &[Player], id: PlayerID) -> Option<usize> {
    arr.iter().enumerate().rev().find_map(
        |(i, player)| {
            if player.id == id {
                Some(i)
            } else {
                None
            }
        },
    )
}

//cheks which cell ate the other one by knowing which one is bigger, if there is an overlap between the cells
pub fn check_who_ate_who(cell_a: &Cell, cell_b: &Cell) -> u8 {
    if check_overlap(&cell_a.position, &cell_b.position) {
        let min_cell_rad = f32::min(cell_a.position.radius, cell_b.position.radius);
        if min_cell_rad == cell_a.position.radius {
            if(cell_a.mass > 1.2 * cell_b.mass){
                return 2;
            }
        } else {
            if(cell_b.mass > 1.2 * cell_a.mass){
                return 1;
            }
        }
    }
    return 0;
}

//Rework
pub fn is_visible_entity(position_a: Point, player: &Player) -> bool {
    return test_rectangle_rectangle(
        position_a.x,
        position_a.y,
        position_a.radius,
        player.x,
        player.y,
        player.screen_width / player.ratio,
        player.screen_height / player.ratio,
    );
}

fn test_rectangle_rectangle(
    center_x_a: f32,
    center_y_a: f32,
    radius: f32,
    center_x_b: f32,
    center_y_b: f32,
    width_b: f32,
    height_b: f32,
) -> bool {
    let half_width_a = radius;
    let half_height_a = radius;
    let half_width_b = width_b / 2.0;
    let half_height_b = height_b / 2.0;

    center_x_a + half_width_a > center_x_b - half_width_b
        && center_x_a - half_width_a < center_x_b + half_width_b
        && center_y_a + half_height_a > center_y_b - half_height_b
        && center_y_a - half_height_a < center_y_b + half_height_b
}

pub fn are_colliding(cell1: &Point, cell2: &Point) -> bool {
    // Simple collision detection logic (circle-circle collision)
    let dx = cell1.x - cell2.x;
    let dy = cell1.y - cell2.y;

    let distance = (dx * dx + dy * dy).sqrt();

    distance < (cell1.radius + cell2.radius)
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
        let part3 = 0.5
            * f32::sqrt(
                (-distance + r1 + r2)
                    * (distance + r1 - r2)
                    * (distance - r1 + r2)
                    * (distance + r1 + r2),
            );
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
