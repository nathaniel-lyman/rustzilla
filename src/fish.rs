use crate::geom::{Rect, Vec2};

/// Wrap an entity of pixel-width `w` around the horizontal edges: once it
/// fully leaves one side it re-enters on the other.
pub fn wrap_x(p: Vec2, w: f32, bounds: Rect) -> Vec2 {
    let right = bounds.x + bounds.w;
    if p.x + w < bounds.x {
        Vec2 { x: right, y: p.y }
    } else if p.x > right {
        Vec2 { x: bounds.x - w, y: p.y }
    } else {
        p
    }
}

/// Keep an entity of pixel-height `h` fully within the vertical bounds.
pub fn clamp_y(p: Vec2, h: f32, bounds: Rect) -> Vec2 {
    let max_y = bounds.y + bounds.h - h;
    let y = p.y.clamp(bounds.y, max_y.max(bounds.y));
    Vec2 { x: p.x, y }
}

/// Move `speed` units from `p` toward `target`.
pub fn step_toward(p: Vec2, target: Vec2, speed: f32) -> Vec2 {
    let dir = Vec2 { x: target.x - p.x, y: target.y - p.y }.normalized();
    p.add(dir.scaled(speed))
}

/// Move `speed` units from `p` directly away from `threat`.
pub fn step_away(p: Vec2, threat: Vec2, speed: f32) -> Vec2 {
    let dir = Vec2 { x: p.x - threat.x, y: p.y - threat.y }.normalized();
    p.add(dir.scaled(speed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn wrap_horizontally_moves_off_left_to_right() {
        let bounds = Rect { x: 0.0, y: 0.0, w: 20.0, h: 10.0 };
        // A 3-wide sprite fully past the left edge reappears at the right.
        let p = wrap_x(Vec2 { x: -4.0, y: 5.0 }, 3.0, bounds);
        assert!(approx(p.x, 20.0));
        // Fully past the right edge reappears off the left.
        let q = wrap_x(Vec2 { x: 21.0, y: 5.0 }, 3.0, bounds);
        assert!(approx(q.x, -3.0));
    }

    #[test]
    fn clamp_y_keeps_sprite_inside_vertically() {
        let bounds = Rect { x: 0.0, y: 0.0, w: 20.0, h: 10.0 };
        let p = clamp_y(Vec2 { x: 5.0, y: -3.0 }, 2.0, bounds);
        assert!(approx(p.y, 0.0));
        let q = clamp_y(Vec2 { x: 5.0, y: 100.0 }, 2.0, bounds);
        assert!(approx(q.y, 8.0)); // height(10) - sprite height(2)
    }

    #[test]
    fn step_toward_moves_closer() {
        let from = Vec2 { x: 0.0, y: 0.0 };
        let target = Vec2 { x: 10.0, y: 0.0 };
        let next = step_toward(from, target, 2.0); // speed 2 units
        assert!(next.distance(target) < from.distance(target));
        assert!(approx(next.x, 2.0));
    }

    #[test]
    fn step_away_moves_farther() {
        let from = Vec2 { x: 5.0, y: 0.0 };
        let threat = Vec2 { x: 4.0, y: 0.0 };
        let next = step_away(from, threat, 3.0);
        assert!(next.distance(threat) > from.distance(threat));
        assert!(next.x > from.x); // flees to the right, away from threat
    }
}
