use crate::map::food::Food;

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
    northwest: Option<Box<QuadTree>>,
    northeast: Option<Box<QuadTree>>,
    southwest: Option<Box<QuadTree>>,
    southeast: Option<Box<QuadTree>>,
}

impl QuadTree {
    pub fn new(boundary: Rectangle, capacity: usize) -> Self {
        QuadTree {
            boundary,
            capacity,
            points: Vec::new(),
            divided: false,
            northwest: None,
            northeast: None,
            southwest: None,
            southeast: None,
        }
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

            if self.northwest.as_mut().unwrap().insert(point) || 
               self.northeast.as_mut().unwrap().insert(point) ||
               self.southwest.as_mut().unwrap().insert(point) ||
               self.southeast.as_mut().unwrap().insert(point) {
                return true;
            }
        }

        false
    }

    fn subdivide(&mut self) {
        let x = self.boundary.x;
        let y = self.boundary.y;
        let w = self.boundary.w / 2.0;
        let h = self.boundary.h / 2.0;

        self.northwest = Some(Box::new(QuadTree::new(Rectangle { x, y, w, h }, self.capacity)));
        self.northeast = Some(Box::new(QuadTree::new(Rectangle { x: x + w, y, w, h }, self.capacity)));
        self.southwest = Some(Box::new(QuadTree::new(Rectangle { x, y: y + h, w, h }, self.capacity)));
        self.southeast = Some(Box::new(QuadTree::new(Rectangle { x: x + w, y: y + h, w, h }, self.capacity)));

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
            self.northwest.as_ref().unwrap().retrieve(range, found);
            self.northeast.as_ref().unwrap().retrieve(range, found);
            self.southwest.as_ref().unwrap().retrieve(range, found);
            self.southeast.as_ref().unwrap().retrieve(range, found);
        }
    }

    pub fn remove(&mut self, point: &Food) -> bool {
        if !self.boundary.contains(point) {
            return false;
        }

        // Try to remove the point from the current node
        if let Some(index) = self.points.iter().position(|p| p.x == point.x && p.y == point.y) {
            self.points.remove(index);
            return true;
        }

        // If the point is not in the current node and the tree is divided, try to remove it from the children
        if self.divided {
            return self.northwest.as_mut().unwrap().remove(point)
                || self.northeast.as_mut().unwrap().remove(point)
                || self.southwest.as_mut().unwrap().remove(point)
                || self.southeast.as_mut().unwrap().remove(point);
        }

        false
    }
}
