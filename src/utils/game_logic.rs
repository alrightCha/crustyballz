use crate::map::point::Point;

//make sure that the player is always within the boundaries of the map and limit it there 
pub fn adjust_for_boundaries(x: &mut f32, y: &mut f32, radius: f32, border_offset: f32, game_width: f32, game_height: f32) {
    let border_calc = radius + border_offset;

    // Clamp x within the horizontal boundaries
    *x = x.clamp(border_calc, game_width - border_calc);

    // Clamp y within the vertical boundaries
    *y = y.clamp(border_calc, game_height - border_calc);
}


// Function to check if a position is touching the game border
pub fn is_touching_border(position: &Point, radius: f32, game_width: f32, game_height: f32) -> bool {
    let border_calc = radius + 5.0;
    position.x > game_width - border_calc ||
    position.y > game_height - border_calc ||
    position.x < border_calc ||
    position.y < border_calc
}

// Function to change the target direction based on the border collisions
pub fn reaim(position: &Point, target: &mut Point, radius: f32, game_width: f32, game_height: f32) {
    let border_calc = radius + 5.0;

    // Reflect horizontally if touching right or left border
    if position.x > game_width - border_calc {
        target.x = target.x.abs() * 4.0; // Reflect horizontally
        target.y = -target.y.abs() * 2.0;
    } else if position.x < border_calc {
        target.x = -target.x.abs() * 2.0; // Reflect horizontally
    }

    // Reflect vertically if touching bottom or top border
    if position.y > game_height - border_calc {
        target.y = target.y.abs() * 2.0; // Reflect vertically
    } else if position.y < border_calc {
        target.y = -target.y.abs() * 2.0; // Reflect vertically
    }
}