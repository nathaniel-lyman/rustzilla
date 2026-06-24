use crate::geom::{Rect, Vec2};
use crate::sprite::Sprite;

/// What an entity is, so the tank can build context, enforce the fish cap,
/// and resolve fish-vs-food collisions without downcasting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Fish,
    Food,
    Shark,
}

/// Read-only snapshot the tank hands to every entity each tick. Entities
/// never mutate the tank directly; the tank applies spawns/removals itself.
pub struct TankCtx {
    pub bounds: Rect,
    pub dt: f32,
    pub food: Vec<Vec2>,        // pellet positions this tick
    pub shark: Option<Vec2>,    // shark position, if one is present
}

pub trait Entity {
    fn update(&mut self, ctx: &TankCtx);
    fn sprite(&self) -> Sprite;
    fn pos(&self) -> Vec2;
    fn bounds(&self) -> Rect;
    fn kind(&self) -> Kind;
    /// True once the entity should be removed (eaten pellet, exited shark).
    fn dead(&self) -> bool;
    /// Called by the tank when a fish overlaps this entity (only Food acts
    /// on it; default is a no-op).
    fn on_eaten(&mut self) {}
}

const SINK_SPEED: f32 = 3.0;       // units/sec
const DISSOLVE_AFTER: f32 = 4.0;   // seconds resting on the bottom

pub struct Food {
    pos: Vec2,
    eaten: bool,
    rest_time: f32,
}

impl Food {
    pub fn new(pos: Vec2) -> Food {
        Food { pos, eaten: false, rest_time: 0.0 }
    }
}

impl Entity for Food {
    fn update(&mut self, ctx: &TankCtx) {
        let floor = ctx.bounds.y + ctx.bounds.h - 1.0;
        if self.pos.y < floor {
            self.pos.y = (self.pos.y + SINK_SPEED * ctx.dt).min(floor);
        } else {
            self.rest_time += ctx.dt;
        }
    }

    fn sprite(&self) -> Sprite {
        Sprite::new(vec!["•".into()])
    }

    fn pos(&self) -> Vec2 {
        self.pos
    }

    fn bounds(&self) -> Rect {
        Rect { x: self.pos.x, y: self.pos.y, w: 1.0, h: 1.0 }
    }

    fn kind(&self) -> Kind {
        Kind::Food
    }

    fn dead(&self) -> bool {
        self.eaten || self.rest_time >= DISSOLVE_AFTER
    }

    fn on_eaten(&mut self) {
        self.eaten = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Rect;

    fn ctx() -> TankCtx {
        TankCtx {
            bounds: Rect { x: 0.0, y: 0.0, w: 40.0, h: 20.0 },
            dt: 1.0,
            food: vec![],
            shark: None,
        }
    }

    #[test]
    fn food_sinks_over_time() {
        let mut f = Food::new(Vec2 { x: 5.0, y: 0.0 });
        f.update(&ctx());
        assert!(f.pos().y > 0.0);
        assert_eq!(f.kind(), Kind::Food);
    }

    #[test]
    fn food_dies_when_eaten() {
        let mut f = Food::new(Vec2 { x: 5.0, y: 0.0 });
        assert!(!f.dead());
        f.on_eaten();
        assert!(f.dead());
    }

    #[test]
    fn food_dies_after_resting_on_the_bottom() {
        let mut f = Food::new(Vec2 { x: 5.0, y: 0.0 });
        // Sink to the bottom and linger; it should eventually dissolve.
        for _ in 0..100 {
            f.update(&ctx());
        }
        assert!(f.dead());
    }
}
