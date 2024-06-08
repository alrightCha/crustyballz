//used as a way to abstract the data on the map to be able to use the same function on many items because
//they all have a position and a radius

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}

impl Point {
    pub fn normalize(&self) -> Point {
        if self.x == 0.0 && self.y == 0.0 {
            return self.clone();
        }

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
            radius: 0.0,
        }
    }

    pub fn distance_pow(&self, other: &Point) -> f32 {
        (self.x - other.x).powf(2.0) + (self.y - other.y).powf(2.0)
    }

    pub fn distance(&self, other: &Point) -> f32 {
        self.distance_pow(other).sqrt()
    }
}

pub trait AsPoint {
    fn as_point(&self) -> Point;
}

#[cfg(test)]
mod tests {
    use super::Point;

    #[test]
    fn test_normalize() {
        let point = Point {
            x: 10.0,
            y: 10.0,
            radius: 0.0,
        };

        assert_eq!(
            Point {
                x: 0.70710677,
                y: 0.70710677,
                radius: point.radius
            },
            point.normalize()
        );
        
        let point = Point {
            x: -10.0,
            y: -10.0,
            radius: 0.0,
        };

        assert_eq!(
            Point {
                x: -0.70710677,
                y: -0.70710677,
                radius: point.radius
            },
            point.normalize()
        );
    }

    #[test]
    fn test_scale() {
        let point = Point {
            x: 10.0,
            y: 10.0,
            radius: 0.0,
        };

        assert_eq!(
            Point {
                x: 11.1,
                y: 11.1,
                radius: point.radius
            },
            point.scale(1.11)
        );
        assert_eq!(
            Point {
                x: -11.1,
                y: -11.1,
                radius: point.radius
            },
            point.scale(-1.11)
        );
    }
}
