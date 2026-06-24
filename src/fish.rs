use crate::entity::{Entity, Kind, TankCtx};
use crate::geom::{Rect, Vec2};
use crate::sprite::Sprite;

const SENSE_RADIUS: f32 = 12.0; // how far a fish notices food
const FEAR_RADIUS: f32 = 10.0; // how close a shark must be to scare
const FLEE_SPEED: f32 = 8.0;
const SEEK_SPEED: f32 = 4.0;

pub struct Googly {
    pos: Vec2,
    vx: f32, // horizontal drift, units/sec (sign = facing)
}

impl Googly {
    pub fn new(pos: Vec2, vx: f32) -> Googly {
        Googly { pos, vx }
    }
}

/// Nearest position in `food` within `SENSE_RADIUS`, if any.
fn nearest_food(p: Vec2, food: &[Vec2]) -> Option<Vec2> {
    food.iter()
        .copied()
        .filter(|f| p.distance(*f) <= SENSE_RADIUS)
        .min_by(|a, b| {
            p.distance(*a)
                .partial_cmp(&p.distance(*b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// One drift/seek/flee step for a standard fish. `fears_shark` lets Ducky
/// opt out of fleeing. Returns the new position.
pub fn swim_step(
    pos: Vec2,
    vx: f32,
    sprite_w: f32,
    sprite_h: f32,
    fears_shark: bool,
    ctx: &crate::entity::TankCtx,
) -> Vec2 {
    if fears_shark {
        if let Some(s) = ctx.shark {
            if pos.distance(s) <= FEAR_RADIUS {
                let p = step_away(pos, s, FLEE_SPEED * ctx.dt);
                return wrap_x(clamp_y(p, sprite_h, ctx.bounds), sprite_w, ctx.bounds);
            }
        }
    }
    let p = if let Some(target) = nearest_food(pos, &ctx.food) {
        step_toward(pos, target, SEEK_SPEED * ctx.dt)
    } else {
        pos.add(Vec2 {
            x: vx * ctx.dt,
            y: 0.0,
        })
    };
    wrap_x(clamp_y(p, sprite_h, ctx.bounds), sprite_w, ctx.bounds)
}

impl Entity for Googly {
    fn update(&mut self, ctx: &TankCtx) {
        let (w, h) = (self.sprite().width() as f32, self.sprite().height() as f32);
        self.pos = swim_step(self.pos, self.vx, w, h, true, ctx);
    }

    fn sprite(&self) -> Sprite {
        let mut s = Sprite::new(vec!["<°)))><".into()]);
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
        Kind::Fish
    }

    fn dead(&self) -> bool {
        false
    }
}

/// Wrap an entity of pixel-width `w` around the horizontal edges: once it
/// fully leaves one side it re-enters on the other.
pub fn wrap_x(p: Vec2, w: f32, bounds: Rect) -> Vec2 {
    let right = bounds.x + bounds.w;
    if p.x + w < bounds.x {
        Vec2 { x: right, y: p.y }
    } else if p.x > right {
        Vec2 {
            x: bounds.x - w,
            y: p.y,
        }
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
    let dir = Vec2 {
        x: target.x - p.x,
        y: target.y - p.y,
    }
    .normalized();
    p.add(dir.scaled(speed))
}

/// Move `speed` units from `p` directly away from `threat`.
pub fn step_away(p: Vec2, threat: Vec2, speed: f32) -> Vec2 {
    let dir = Vec2 {
        x: p.x - threat.x,
        y: p.y - threat.y,
    }
    .normalized();
    p.add(dir.scaled(speed))
}

pub struct Tophat {
    pos: Vec2,
    vx: f32,
}
impl Tophat {
    pub fn new(pos: Vec2, vx: f32) -> Tophat {
        Tophat { pos, vx }
    }
}
impl Entity for Tophat {
    fn update(&mut self, ctx: &TankCtx) {
        let (w, h) = (self.sprite().width() as f32, self.sprite().height() as f32);
        // Dignified: drifts at half speed.
        self.pos = swim_step(self.pos, self.vx * 0.5, w, h, true, ctx);
    }
    fn sprite(&self) -> Sprite {
        let mut s = Sprite::new(vec![" _o_ ".into(), "<°)))><".into()]);
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
        Kind::Fish
    }
    fn dead(&self) -> bool {
        false
    }
}

pub struct Upsidedown {
    pos: Vec2,
    vx: f32,
    t: f32,
}
impl Upsidedown {
    pub fn new(pos: Vec2, vx: f32) -> Upsidedown {
        Upsidedown { pos, vx, t: 0.0 }
    }
    fn flipped(&self) -> bool {
        // Flip state toggles every ~5 seconds of accumulated time.
        ((self.t / 5.0) as i32) % 2 == 1
    }
}
impl Entity for Upsidedown {
    fn update(&mut self, ctx: &TankCtx) {
        self.t += ctx.dt;
        let (w, h) = (self.sprite().width() as f32, self.sprite().height() as f32);
        self.pos = swim_step(self.pos, self.vx, w, h, true, ctx);
    }
    fn sprite(&self) -> Sprite {
        let mut s = Sprite::new(vec!["<°)))><".into()]);
        s.facing = if self.vx < 0.0 {
            crate::sprite::Facing::Left
        } else {
            crate::sprite::Facing::Right
        };
        s.flip_v = self.flipped();
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
        Kind::Fish
    }
    fn dead(&self) -> bool {
        false
    }
}

pub struct Ducky {
    pos: Vec2,
    vx: f32,
}
impl Ducky {
    pub fn new(pos: Vec2, vx: f32) -> Ducky {
        Ducky { pos, vx }
    }
}
impl Entity for Ducky {
    fn update(&mut self, ctx: &TankCtx) {
        // Bobs along the surface; fears nothing, ignores food entirely.
        let w = self.sprite().width() as f32;
        let p = self.pos.add(Vec2 {
            x: self.vx * ctx.dt,
            y: 0.0,
        });
        self.pos = wrap_x(p, w, ctx.bounds);
        self.pos.y = ctx.bounds.y; // pinned to the top row
    }
    fn sprite(&self) -> Sprite {
        let mut s = Sprite::new(vec!["_(°)<".into()]);
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
        Kind::Fish
    }
    fn dead(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{Entity, Kind, TankCtx};

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn ctx(food: Vec<Vec2>, shark: Option<Vec2>) -> TankCtx {
        TankCtx {
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                w: 40.0,
                h: 20.0,
            },
            dt: 1.0,
            food,
            shark,
        }
    }

    #[test]
    fn googly_drifts_horizontally() {
        let mut f = Googly::new(Vec2 { x: 10.0, y: 5.0 }, 3.0); // +3 units/sec
        let start = f.pos();
        f.update(&ctx(vec![], None));
        assert!((f.pos().x - start.x).abs() > 0.0);
        assert_eq!(f.kind(), Kind::Fish);
    }

    #[test]
    fn googly_seeks_nearby_food() {
        let mut f = Googly::new(Vec2 { x: 10.0, y: 5.0 }, 3.0);
        let pellet = Vec2 { x: 10.0, y: 12.0 }; // directly below, in range
        let before = f.pos().distance(pellet);
        f.update(&ctx(vec![pellet], None));
        assert!(f.pos().distance(pellet) < before);
    }

    #[test]
    fn googly_flees_nearby_shark() {
        let mut f = Googly::new(Vec2 { x: 20.0, y: 5.0 }, 3.0);
        let shark = Vec2 { x: 18.0, y: 5.0 }; // close on the left
        f.update(&ctx(vec![], Some(shark)));
        // Fled to the right, away from the shark.
        assert!(f.pos().x > 20.0);
    }

    #[test]
    fn ducky_ignores_the_shark() {
        let mut d = Ducky::new(Vec2 { x: 20.0, y: 0.0 }, 2.0);
        let shark = Vec2 { x: 19.0, y: 0.0 };
        let before = d.pos();
        d.update(&ctx(vec![], Some(shark)));
        // Ducky drifts normally; it does not bolt away from the shark.
        assert!((d.pos().y - before.y).abs() < 0.001); // stayed at the surface
    }

    #[test]
    fn tophat_and_upsidedown_are_fish() {
        let t = Tophat::new(Vec2 { x: 3.0, y: 3.0 }, 2.0);
        let u = Upsidedown::new(Vec2 { x: 3.0, y: 8.0 }, 2.0);
        assert_eq!(t.kind(), Kind::Fish);
        assert_eq!(u.kind(), Kind::Fish);
    }

    #[test]
    fn upsidedown_sprite_flips_periodically() {
        let mut u = Upsidedown::new(Vec2 { x: 3.0, y: 8.0 }, 2.0);
        let first = u.sprite().flip_v; // false at t=0
                                       // ctx() uses dt=1.0; 6 ticks → t=6, which crosses the 5.0s flip
                                       // interval exactly once (flipped(6) == true != flipped(0)).
        for _ in 0..6 {
            u.update(&ctx(vec![], None));
        }
        assert_ne!(u.sprite().flip_v, first);
    }

    #[test]
    fn wrap_horizontally_moves_off_left_to_right() {
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            w: 20.0,
            h: 10.0,
        };
        // A 3-wide sprite fully past the left edge reappears at the right.
        let p = wrap_x(Vec2 { x: -4.0, y: 5.0 }, 3.0, bounds);
        assert!(approx(p.x, 20.0));
        // Fully past the right edge reappears off the left.
        let q = wrap_x(Vec2 { x: 21.0, y: 5.0 }, 3.0, bounds);
        assert!(approx(q.x, -3.0));
    }

    #[test]
    fn clamp_y_keeps_sprite_inside_vertically() {
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            w: 20.0,
            h: 10.0,
        };
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
