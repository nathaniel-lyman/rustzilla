#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn add(self, other: Vec2) -> Vec2 {
        Vec2 { x: self.x + other.x, y: self.y + other.y }
    }

    pub fn scaled(self, k: f32) -> Vec2 {
        Vec2 { x: self.x * k, y: self.y * k }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn distance(self, other: Vec2) -> f32 {
        Vec2 { x: self.x - other.x, y: self.y - other.y }.length()
    }

    /// Unit vector in the same direction; the zero vector maps to itself.
    pub fn normalized(self) -> Vec2 {
        let len = self.length();
        if len < 1e-6 {
            self
        } else {
            self.scaled(1.0 / len)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn add_and_scale() {
        let a = Vec2 { x: 1.0, y: 2.0 };
        let b = Vec2 { x: 3.0, y: -1.0 };
        let sum = a.add(b);
        assert_eq!(sum, Vec2 { x: 4.0, y: 1.0 });
        assert_eq!(a.scaled(2.0), Vec2 { x: 2.0, y: 4.0 });
    }

    #[test]
    fn distance_and_normalize() {
        let a = Vec2 { x: 0.0, y: 0.0 };
        let b = Vec2 { x: 3.0, y: 4.0 };
        assert!(approx(a.distance(b), 5.0));
        let n = b.normalized();
        assert!(approx(n.x, 0.6) && approx(n.y, 0.8));
    }

    #[test]
    fn normalize_zero_is_zero() {
        let z = Vec2 { x: 0.0, y: 0.0 };
        assert_eq!(z.normalized(), z);
    }
}
