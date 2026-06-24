use crate::geom::{Rect, Vec2};
use crate::sprite::{Color, Sprite};

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
    pub food: Vec<Vec2>,     // pellet positions this tick
    pub fish: Vec<Vec2>,     // fish positions this tick (prey for the shark)
    pub shark: Option<Vec2>, // shark position, if one is present
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

const SINK_SPEED: f32 = 3.0; // units/sec
const DISSOLVE_AFTER: f32 = 4.0; // seconds resting on the bottom

pub struct Food {
    pos: Vec2,
    eaten: bool,
    rest_time: f32,
}

impl Food {
    pub fn new(pos: Vec2) -> Food {
        Food {
            pos,
            eaten: false,
            rest_time: 0.0,
        }
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
        Sprite::new(vec!["•".into()]).bold().colored(Color::Yellow)
    }

    fn pos(&self) -> Vec2 {
        self.pos
    }

    fn bounds(&self) -> Rect {
        Rect {
            x: self.pos.x,
            y: self.pos.y,
            w: 1.0,
            h: 1.0,
        }
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

pub struct Shark {
    pos: Vec2,
    vx: f32,
    gone: bool,
}

impl Shark {
    pub fn new(pos: Vec2, vx: f32) -> Shark {
        Shark {
            pos,
            vx,
            gone: false,
        }
    }
}

impl Entity for Shark {
    fn update(&mut self, ctx: &TankCtx) {
        self.pos.x += self.vx * ctx.dt;
        let w = self.sprite().width() as f32;
        // Despawn once fully past the far edge (in its travel direction).
        let off_right = self.vx > 0.0 && self.pos.x > ctx.bounds.x + ctx.bounds.w;
        let off_left = self.vx < 0.0 && self.pos.x + w < ctx.bounds.x;
        if off_right || off_left {
            self.gone = true;
        }
    }

    fn sprite(&self) -> Sprite {
        // Base art faces right: dorsal fin, chunky body, eye/nose on the right.
        // Bold + red gives the simple ASCII real weight on screen.
        let mut s = Sprite::new(vec!["     /\\".into(), "<#######°>".into()])
            .bold()
            .colored(Color::Red);
        s.facing = if self.vx < 0.0 {
            crate::sprite::Facing::Left
        } else {
            crate::sprite::Facing::Right
        };
        s
    }

    fn pos(&self) -> Vec2 {
        self.pos
    }

    fn bounds(&self) -> Rect {
        Rect {
            x: self.pos.x,
            y: self.pos.y,
            w: self.sprite().width() as f32,
            h: self.sprite().height() as f32,
        }
    }

    fn kind(&self) -> Kind {
        Kind::Shark
    }

    fn dead(&self) -> bool {
        self.gone
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Rect;

    fn ctx() -> TankCtx {
        TankCtx {
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                w: 40.0,
                h: 20.0,
            },
            dt: 1.0,
            food: vec![],
            fish: vec![],
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

    #[test]
    fn shark_cruises_horizontally() {
        let mut s = Shark::new(Vec2 { x: 0.0, y: 5.0 }, 6.0);
        let x0 = s.pos().x;
        s.update(&ctx());
        assert!(s.pos().x > x0);
        assert_eq!(s.kind(), Kind::Shark);
    }

    #[test]
    fn shark_dies_after_leaving_the_far_side() {
        let mut s = Shark::new(Vec2 { x: 0.0, y: 5.0 }, 6.0);
        assert!(!s.dead());
        for _ in 0..100 {
            s.update(&ctx()); // ctx bounds width = 40
        }
        assert!(s.dead());
    }
}
