//used as a way to abstract the data on the map to be able to use the same function on many items because 
//they all have a position and a radius

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}

impl Point {
    pub fn normalize(&self) -> Point {
        let norm = (self.x.powi(2) + self.y.powi(2)).sqrt();
        Point {
            x: self.x / norm,
            y: self.y / norm,
            radius: self.radius, // Or however you decide to handle radius
        }
    }

    pub fn scale(&self, factor: f32) -> Point {
        Point {
            x: self.x * factor,
            y: self.y * factor,
            radius: 0.0
        }
    }
}

pub trait AsPoint {
    fn as_point(&self) -> Point;
}
