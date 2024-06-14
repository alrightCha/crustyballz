use crate::map::food::Food;

use super::id::FoodID;

#[derive(Debug)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rectangle {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rectangle { x, y, w, h }
    }

    fn contains(&self, point: &Food) -> bool {
        let in_x_bounds = point.x >= self.x && point.x <= self.x + self.w;
        let in_y_bounds = point.y >= self.y && point.y <= self.y + self.h;
        in_x_bounds && in_y_bounds
    }

    fn intersects(&self, range: &Rectangle) -> bool {
        let x_overlap = !(range.x > self.x + self.w || range.x + range.w < self.x);
        let y_overlap = !(range.y > self.y + self.h || range.y + range.h < self.y);
        x_overlap && y_overlap
    }
}

//This is an implementation of a quadtree which helps optimizing the food.rs so that the search for the food on the screen is made fast
//reducing the complexity from O^2 to n log(n)
//I used it only on the food because there is a lot of food compared to other items
pub struct QuadTree {
    boundary: Rectangle,
    capacity: usize,
    points: Vec<Food>,
    divided: bool,
    north_west: Option<Box<QuadTree>>,
    north_east: Option<Box<QuadTree>>,
    south_west: Option<Box<QuadTree>>,
    south_east: Option<Box<QuadTree>>,
}

impl QuadTree {
    pub fn new(boundary: Rectangle, capacity: usize) -> Self {
        QuadTree {
            boundary,
            capacity,
            points: Vec::new(),
            divided: false,
            north_west: None,
            north_east: None,
            south_west: None,
            south_east: None,
        }
    }

    pub fn get_all_foods(&self) -> Vec<&Food> {
        let mut foods: Vec<&Food> = self.points.iter().collect();

        if self.divided {
            if let Some(ref node) = self.north_west {
                foods.extend(node.get_all_foods());
            }
            if let Some(ref node) = self.north_east {
                foods.extend(node.get_all_foods());
            }
            if let Some(ref node) = self.south_west {
                foods.extend(node.get_all_foods());
            }
            if let Some(ref node) = self.south_east {
                foods.extend(node.get_all_foods());
            }
        }

        foods
    }

    pub fn insert(&mut self, point: Food) -> bool {
        if !self.boundary.contains(&point) {
            return false;
        }

        if self.points.len() < self.capacity {
            self.points.push(point);
            return true;
        } else {
            if !self.divided {
                self.subdivide();
            }

            if self.divided {
                if let Some(ref mut node) = self.north_west {
                    if node.insert(point) {
                        return true;
                    }
                }
                if let Some(ref mut node) = self.north_east {
                    if node.insert(point) {
                        return true;
                    }
                }
                if let Some(ref mut node) = self.south_west {
                    if node.insert(point) {
                        return true;
                    }
                }
                if let Some(ref mut node) = self.south_east {
                    if node.insert(point) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn subdivide(&mut self) {
        let x = self.boundary.x;
        let y = self.boundary.y;
        let w = self.boundary.w / 2.0;
        let h = self.boundary.h / 2.0;

        self.north_west = Some(Box::new(QuadTree::new(
            Rectangle { x, y, w, h },
            self.capacity,
        )));
        self.north_east = Some(Box::new(QuadTree::new(
            Rectangle { x: x + w, y, w, h },
            self.capacity,
        )));
        self.south_west = Some(Box::new(QuadTree::new(
            Rectangle { x, y: y + h, w, h },
            self.capacity,
        )));
        self.south_east = Some(Box::new(QuadTree::new(
            Rectangle {
                x: x + w,
                y: y + h,
                w,
                h,
            },
            self.capacity,
        )));

        self.divided = true;
    }

    pub fn retrieve(&self, range: &Rectangle, found: &mut Vec<Food>) {
        if !self.boundary.intersects(range) {
            return;
        }

        for p in &self.points {
            if range.contains(p) {
                found.push(*p);
            }
        }

        if self.divided {
            if let Some(ref node) = self.north_west {
                node.retrieve(range, found);
            }
            if let Some(ref node) = self.north_east {
                node.retrieve(range, found);
            }
            if let Some(ref node) = self.south_west {
                node.retrieve(range, found);
            }
            if let Some(ref node) = self.south_east {
                node.retrieve(range, found);
            }
        }
    }

    pub fn contains_food(&self, food_id: FoodID) -> bool {
        if let Some(_) = self.points.iter().position(|p| p.id == food_id) {
            return true;
        }

        if self.divided {
            if let Some(ref node) = self.north_west {
                return node.contains_food(food_id);
            }
            if let Some(ref node) = self.north_east {
                return node.contains_food(food_id);
            }
            if let Some(ref node) = self.south_west {
                return node.contains_food(food_id);
            }
            if let Some(ref node) = self.south_east {
                return node.contains_food(food_id);
            }
        }

        false
    }

    pub fn remove(&mut self, food: &Food) -> bool {
        if !self.boundary.contains(food) {
            return false;
        }

        // Try to remove the point from the current node
        if let Some(index) = self.points.iter().position(|p| p.id == food.id) {
            self.points.remove(index);
            return true;
        }

        // If the point is not in the current node and the tree is divided, try to remove it from the children
        if self.divided {
            if let Some(ref mut node) = self.north_west {
                return node.remove(food);
            }
            if let Some(ref mut node) = self.north_east {
                return node.remove(food);
            }
            if let Some(ref mut node) = self.south_west {
                return node.remove(food);
            }
            if let Some(ref mut node) = self.south_east {
                return node.remove(food);
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use crate::map::{food::Food, point::Point};

    use super::{QuadTree, Rectangle};

    #[test]
    fn test_quad_tree_insert() {
        let boundary = Rectangle::new(0.0, 0.0, 10_000.0, 10_000.0);
        let mut quad_tree = QuadTree::new(boundary, 25);

        for i in 0..25 {
            quad_tree.insert(Food::new(
                i as u32,
                Point {
                    x: 0.0,
                    y: 0.0,
                    radius: 1.0,
                },
            ));
        }

        assert_eq!(quad_tree.points.len(), 25);
        assert_eq!(quad_tree.divided, false);

        quad_tree.insert(Food::new(
            9999 as u32,
            Point {
                x: 0.0,
                y: 0.0,
                radius: 1.0,
            },
        ));

        assert_eq!(quad_tree.points.len(), 25);
        assert_eq!(quad_tree.divided, true);

        match quad_tree.north_west {
            Some(ref tree) => {
                assert_eq!(tree.points.len(), 1);
                assert_eq!(tree.divided, false);
            }
            None => panic!(),
        }

        for i in 0..25 {
            quad_tree.insert(Food::new(
                i as u32,
                Point {
                    x: 0.0,
                    y: 0.0,
                    radius: 1.0,
                },
            ));
        }

        match quad_tree.north_west {
            Some(ref tree) => match tree.north_west {
                Some(ref tree_north) => {
                    assert_eq!(tree_north.points.len(), 1);
                    assert_eq!(tree_north.divided, false);
                }
                None => panic!(),
            },
            None => panic!(),
        }
    }

    #[test]
    fn test_quad_tree_retrieve() {
        let boundary = Rectangle::new(0.0, 0.0, 10_000.0, 10_000.0);
        let mut quad_tree = QuadTree::new(boundary, 25);

        for i in 0..26 {
            quad_tree.insert(Food::new(
                i as u32,
                Point {
                    x: 0.0,
                    y: 0.0,
                    radius: 1.0,
                },
            ));
        }

        let player_view = Rectangle::new(0.0, 0.0, 1920.0, 1080.0);

        let mut foods = vec![];
        quad_tree.retrieve(&player_view, &mut foods);
        assert_eq!(foods.len(), 26);

        let player_view = Rectangle::new(2.0, 2.0, 1920.0, 1080.0);

        let mut foods = vec![];
        quad_tree.retrieve(&player_view, &mut foods);
        assert_eq!(foods.len(), 0);
    }
}
