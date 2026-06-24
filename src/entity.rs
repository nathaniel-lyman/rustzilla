use crate::geom::{Rect, Vec2};
use crate::sprite::{Color, Facing, Sprite};

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
    /// Called by the tank when this entity eats a fish (only Shark acts on it;
    /// default is a no-op).
    fn on_kill(&mut self) {}
}

const SINK_SPEED: f32 = 3.0; // units/sec
const DISSOLVE_AFTER: f32 = 4.0; // seconds resting on the bottom

const HUNT_SPEED: f32 = 10.0; // units/sec the shark steers toward prey
const FULL_AFTER: usize = 3; // kills before the shark loses interest and leaves

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
    eaten: usize,
    facing_right: bool,
}

impl Shark {
    pub fn new(pos: Vec2, vx: f32) -> Shark {
        Shark {
            pos,
            vx,
            gone: false,
            eaten: 0,
            facing_right: vx >= 0.0,
        }
    }
}

impl Entity for Shark {
    fn update(&mut self, ctx: &TankCtx) {
        // Hunting: while not yet full, steer toward the nearest fish in reach.
        let target = if self.eaten < FULL_AFTER {
            crate::fish::nearest(self.pos, &ctx.fish, crate::fish::hunt_radius(ctx.bounds))
        } else {
            None
        };

        let dx = if let Some(t) = target {
            let h = self.sprite().height() as f32;
            let stepped = crate::fish::step_toward(self.pos, t, HUNT_SPEED * ctx.dt);
            let next = crate::fish::clamp_y(stepped, h, ctx.bounds);
            let dx = next.x - self.pos.x;
            self.pos = next;
            dx
        } else {
            // Cruising: full, or no prey in range — move straight as before.
            self.pos.x += self.vx * ctx.dt;
            self.vx * ctx.dt
        };

        // Face the way it actually moved (negligible motion keeps the heading).
        if dx.abs() > 1e-3 {
            self.facing_right = dx > 0.0;
        }

        let w = self.sprite().width() as f32;
        // Despawn once fully past the far edge (in its cruise direction).
        let off_right = self.vx > 0.0 && self.pos.x > ctx.bounds.x + ctx.bounds.w;
        let off_left = self.vx < 0.0 && self.pos.x + w < ctx.bounds.x;
        if off_right || off_left {
            self.gone = true;
        }
    }

    fn sprite(&self) -> Sprite {
        // Base art faces right: dorsal fin, chunky body, eye/nose on the right.
        // The body widens one segment per kill so fullness is legible at a
        // glance; bold + red gives the simple ASCII real weight on screen.
        let body = format!("<{}°>", "#".repeat(7 + self.eaten));
        let mut s = Sprite::new(vec!["     /\\".into(), body])
            .bold()
            .colored(Color::Red);
        s.facing = if self.facing_right {
            Facing::Right
        } else {
            Facing::Left
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

    fn on_kill(&mut self) {
        self.eaten += 1;
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
    fn shark_fattens_as_it_eats() {
        let mut s = Shark::new(Vec2 { x: 0.0, y: 5.0 }, 6.0);
        let w0 = s.sprite().width();
        s.on_kill();
        assert!(s.sprite().width() > w0, "shark body should widen per kill");
    }

    #[test]
    fn shark_steers_toward_nearest_fish() {
        let fish = Vec2 { x: 10.0, y: 15.0 }; // below and ahead, within hunt_radius
        let c = TankCtx {
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                w: 40.0,
                h: 20.0,
            },
            dt: 1.0,
            food: vec![],
            fish: vec![fish],
            shark: None,
        };
        let mut s = Shark::new(Vec2 { x: 5.0, y: 5.0 }, 6.0);
        let before = s.pos().distance(fish);
        s.update(&c);
        assert!(s.pos().distance(fish) < before, "shark should close on prey");
        assert!(
            s.pos().y > 5.0,
            "a pure cruise would keep y fixed; hunting moves it"
        );
    }

    #[test]
    fn full_shark_stops_hunting_and_cruises() {
        let fish = Vec2 { x: 5.0, y: 15.0 }; // off-axis, within hunt_radius
        let c = TankCtx {
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                w: 40.0,
                h: 20.0,
            },
            dt: 1.0,
            food: vec![],
            fish: vec![fish],
            shark: None,
        };
        let mut s = Shark::new(Vec2 { x: 5.0, y: 5.0 }, 6.0);
        for _ in 0..FULL_AFTER {
            s.on_kill();
        }
        s.update(&c);
        assert!(
            (s.pos().y - 5.0).abs() < 1e-4,
            "full shark ignores the off-axis fish"
        );
        assert!(s.pos().x > 5.0, "full shark cruises by +vx");
    }

    #[test]
    fn hungry_shark_with_no_prey_cruises_off() {
        // No fish present: a still-hungry shark must keep cruising straight and
        // despawn, so it can never sit forever waiting — the tank is never stuck.
        let c = TankCtx {
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
        };
        let mut s = Shark::new(Vec2 { x: 0.0, y: 5.0 }, 6.0);
        assert!(s.eaten < FULL_AFTER, "precondition: shark starts hungry");
        for _ in 0..100 {
            s.update(&c);
        }
        assert!(
            s.dead(),
            "hungry shark with no prey should cruise off and despawn"
        );
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
