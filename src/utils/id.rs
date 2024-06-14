pub fn from_position(x: u16, y: u16) -> u32 {
    ((x as u32) << 16) | y as u32
}

pub type PlayerID = u8;
pub type FoodID = u32;
pub type VirusID = u16;
pub type MassFoodID = u16;
