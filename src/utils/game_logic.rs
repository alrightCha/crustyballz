use crate::map::point::Point;

//make sure that the player is always within the boundaries of the map and limit it there 
pub fn adjust_for_boundaries(x: &mut f32, y: &mut f32, radius: f32, border_offset: f32, game_width: f32, game_height: f32) {
    let border_calc = radius + border_offset;

    // Clamp x within the horizontal boundaries
    *x = x.clamp(border_calc, game_width - border_calc);

    // Clamp y within the vertical boundaries
    *y = y.clamp(border_calc, game_height - border_calc);
}
