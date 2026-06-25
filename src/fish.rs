use crate::entity::{Entity, Kind, TankCtx};
use crate::geom::{Rect, Vec2};
use crate::sprite::{Color, PixelSprite};

const FLEE_SPEED: f32 = 8.0;
const SEEK_SPEED: f32 = 4.0;

/// Peak vertical drift speed (cells/sec) while a fish is idly cruising. Kept
/// small so the wander is a gentle bob, not a frantic bounce.
const WANDER_SPEED: f32 = 2.5;

/// Subtle vertical drift velocity (cells/sec) for an idly-cruising fish.
/// Summing two sine waves whose frequencies don't share a common period makes
/// the path wander up and down organically instead of tracing one clean,
/// obviously-repeating curve. `phase` (a per-fish constant) staggers the cast
/// so they don't all bob in lockstep. Only used in the drift branch — a fish
/// seeking food or fleeing a shark already moves with intent.
fn vertical_wander(t: f32, phase: f32) -> f32 {
    WANDER_SPEED * (0.7 * (t * 0.9 + phase).sin() + 0.3 * (t * 2.1 + phase * 1.7).sin())
}

/// A stable per-fish wander phase derived from spawn position, so each fish's
/// vertical bob is out of step with its neighbours. Spawns already spread their
/// positions, so deriving the phase here needs no extra constructor argument.
fn wander_phase(pos: Vec2) -> f32 {
    pos.x * 0.7 + pos.y * 1.3
}

/// How far a fish notices food, scaled to the tank so reactions stay visible
/// in a large terminal (a fixed radius is invisible in a 150-cell-wide tank).
fn sense_radius(bounds: Rect) -> f32 {
    (bounds.w.max(bounds.h) * 0.30).max(12.0)
}

/// How close the shark must be to scare a fish, scaled to the tank.
fn fear_radius(bounds: Rect) -> f32 {
    (bounds.w.max(bounds.h) * 0.35).max(14.0)
}

/// How close a fish must be for the shark to commit to hunting it, scaled to
/// the tank like the other radii. `pub` (unlike its siblings) because the
/// shark in `entity.rs` reads it. Kept >= `fear_radius` so a fish that has
/// just begun to flee is, briefly, still a target.
pub fn hunt_radius(bounds: Rect) -> f32 {
    (bounds.w.max(bounds.h) * 0.40).max(16.0)
}

pub struct Googly {
    pos: Vec2,
    vx: f32, // horizontal drift, units/sec
    t: f32,  // accumulated time, for the vertical wander
    phase: f32,
    facing_right: bool,
    eaten: bool,
}

impl Googly {
    pub fn new(pos: Vec2, vx: f32) -> Googly {
        Googly {
            pos,
            vx,
            t: 0.0,
            phase: wander_phase(pos),
            facing_right: vx >= 0.0,
            eaten: false,
        }
    }
}

/// Nearest position in `points` within `radius`, if any. Shared by the fish
/// food-seek path and the shark's prey hunt.
pub fn nearest(p: Vec2, points: &[Vec2], radius: f32) -> Option<Vec2> {
    points
        .iter()
        .copied()
        .filter(|f| p.distance(*f) <= radius)
        .min_by(|a, b| {
            p.distance(*a)
                .partial_cmp(&p.distance(*b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// One drift/seek/flee step for a standard fish. `fears_shark` lets Ducky
/// opt out of fleeing. `vy` is a gentle vertical drift applied only while
/// idly cruising (see `vertical_wander`), so an idle fish bobs up and down
/// instead of tracking a flat line; seeking and fleeing override it. Returns
/// the new position and the intended horizontal delta (before edge-wrapping)
/// so the caller can face the fish the way it actually moved — e.g. flipping
/// around to chase food behind it.
pub fn swim_step(
    pos: Vec2,
    vx: f32,
    vy: f32,
    sprite_w: f32,
    sprite_h: f32,
    fears_shark: bool,
    ctx: &crate::entity::TankCtx,
) -> (Vec2, f32) {
    let intended = if fears_shark
        && ctx
            .shark
            .is_some_and(|s| pos.distance(s) <= fear_radius(ctx.bounds))
    {
        // Flee directly away from the shark.
        step_away(pos, ctx.shark.unwrap(), FLEE_SPEED * ctx.dt)
    } else if let Some(target) = nearest(pos, &ctx.food, sense_radius(ctx.bounds)) {
        step_toward(pos, target, SEEK_SPEED * ctx.dt)
    } else {
        pos.add(Vec2 {
            x: vx * ctx.dt,
            y: vy * ctx.dt,
        })
    };
    let dx = intended.x - pos.x;
    let next = wrap_x(
        clamp_y(intended, sprite_h, ctx.bounds),
        sprite_w,
        ctx.bounds,
    );
    (next, dx)
}

/// Update a stored facing from a horizontal delta, ignoring negligible motion
/// (so a fish moving purely vertically keeps its current heading).
fn face_from_dx(facing_right: &mut bool, dx: f32) {
    if dx.abs() > 1e-3 {
        *facing_right = dx > 0.0;
    }
}

fn facing_of(facing_right: bool) -> crate::sprite::Facing {
    if facing_right {
        crate::sprite::Facing::Right
    } else {
        crate::sprite::Facing::Left
    }
}

impl Entity for Googly {
    fn update(&mut self, ctx: &TankCtx) {
        self.t += ctx.dt;
        let s = self.sprite();
        let (w, h) = (s.cell_w() as f32, s.cell_h() as f32);
        let vy = vertical_wander(self.t, self.phase);
        let (next, dx) = swim_step(self.pos, self.vx, vy, w, h, true, ctx);
        self.pos = next;
        face_from_dx(&mut self.facing_right, dx);
    }

    fn sprite(&self) -> PixelSprite {
        let mut s = PixelSprite::from_art(
            &[
                ".....bbbbb...",
                "..b.bbbbbbbb.",
                ".bb.bbbbwwwbb",
                "bbb.bbbbwkwbb",
                "bbb.bbbbwwwbb",
                ".bb.bbbbbbbbb",
                "..b.bbbbbbbb.",
                ".....bbbbb...",
            ],
            &[('b', Color::Cyan), ('w', Color::White), ('k', Color::Black)],
        );
        s.facing = facing_of(self.facing_right);
        s
    }

    fn pos(&self) -> Vec2 {
        self.pos
    }

    fn bounds(&self) -> Rect {
        let s = self.sprite();
        Rect {
            x: self.pos.x,
            y: self.pos.y,
            w: s.cell_w() as f32,
            h: s.cell_h() as f32,
        }
    }

    fn kind(&self) -> Kind {
        Kind::Fish
    }

    fn dead(&self) -> bool {
        self.eaten
    }

    fn on_eaten(&mut self) {
        self.eaten = true;
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

pub struct Cool {
    pos: Vec2,
    vx: f32,
    t: f32,
    phase: f32,
    facing_right: bool,
    eaten: bool,
}
impl Cool {
    pub fn new(pos: Vec2, vx: f32) -> Cool {
        Cool {
            pos,
            vx,
            t: 0.0,
            phase: wander_phase(pos),
            facing_right: vx >= 0.0,
            eaten: false,
        }
    }
}
impl Entity for Cool {
    fn update(&mut self, ctx: &TankCtx) {
        self.t += ctx.dt;
        let s = self.sprite();
        let (w, h) = (s.cell_w() as f32, s.cell_h() as f32);
        // Too cool to hurry: cruises at half speed, with a lazy half-amplitude bob.
        let vy = vertical_wander(self.t, self.phase) * 0.5;
        let (next, dx) = swim_step(self.pos, self.vx * 0.5, vy, w, h, true, ctx);
        self.pos = next;
        face_from_dx(&mut self.facing_right, dx);
    }
    fn sprite(&self) -> PixelSprite {
        let mut s = PixelSprite::from_art(
            &[
                ".....bbbbb...",
                "..b.bbbbbbbb.",
                ".bb.bbbbkkkkk",
                "bbb.bbbbkkkkk",
                "bbb.bbbbbbbbb",
                ".bb.bbbbbbbbb",
                "..b.bbbbbbbb.",
                ".....bbbbb...",
            ],
            &[('b', Color::Blue), ('k', Color::Black)],
        );
        s.facing = facing_of(self.facing_right);
        s
    }
    fn pos(&self) -> Vec2 {
        self.pos
    }
    fn bounds(&self) -> Rect {
        let s = self.sprite();
        Rect {
            x: self.pos.x,
            y: self.pos.y,
            w: s.cell_w() as f32,
            h: s.cell_h() as f32,
        }
    }
    fn kind(&self) -> Kind {
        Kind::Fish
    }
    fn dead(&self) -> bool {
        self.eaten
    }
    fn on_eaten(&mut self) {
        self.eaten = true;
    }
}

pub struct Upsidedown {
    pos: Vec2,
    vx: f32,
    t: f32,
    phase: f32,
    facing_right: bool,
    eaten: bool,
}
impl Upsidedown {
    pub fn new(pos: Vec2, vx: f32) -> Upsidedown {
        Upsidedown {
            pos,
            vx,
            t: 0.0,
            phase: wander_phase(pos),
            facing_right: vx >= 0.0,
            eaten: false,
        }
    }
    fn flipped(&self) -> bool {
        // Flip state toggles every ~5 seconds of accumulated time.
        ((self.t / 5.0) as i32) % 2 == 1
    }
}
impl Entity for Upsidedown {
    fn update(&mut self, ctx: &TankCtx) {
        self.t += ctx.dt;
        let s = self.sprite();
        let (w, h) = (s.cell_w() as f32, s.cell_h() as f32);
        let vy = vertical_wander(self.t, self.phase);
        let (next, dx) = swim_step(self.pos, self.vx, vy, w, h, true, ctx);
        self.pos = next;
        face_from_dx(&mut self.facing_right, dx);
    }
    fn sprite(&self) -> PixelSprite {
        let mut s = PixelSprite::from_art(
            &[
                ".....bbbbb...",
                "..b.bbbbbbbb.",
                ".bb.bbbbbbbbb",
                "bbb.bbbbbkbbb",
                "bbb.bbbbbbbbb",
                ".bb.bbbbbbbbb",
                "..b.bbbbbbbb.",
                ".....bbbbb...",
            ],
            &[('b', Color::Green), ('k', Color::Black)],
        );
        s.facing = facing_of(self.facing_right);
        s.flip_v = self.flipped();
        s
    }
    fn pos(&self) -> Vec2 {
        self.pos
    }
    fn bounds(&self) -> Rect {
        let s = self.sprite();
        Rect {
            x: self.pos.x,
            y: self.pos.y,
            w: s.cell_w() as f32,
            h: s.cell_h() as f32,
        }
    }
    fn kind(&self) -> Kind {
        Kind::Fish
    }
    fn dead(&self) -> bool {
        self.eaten
    }
    fn on_eaten(&mut self) {
        self.eaten = true;
    }
}

pub struct Ducky {
    pos: Vec2,
    vx: f32,
    eaten: bool,
}
impl Ducky {
    pub fn new(pos: Vec2, vx: f32) -> Ducky {
        Ducky {
            pos,
            vx,
            eaten: false,
        }
    }
}
impl Entity for Ducky {
    fn update(&mut self, ctx: &TankCtx) {
        // Bobs along the surface; fears nothing, ignores food entirely.
        let s = self.sprite();
        let w = s.cell_w() as f32;
        let p = self.pos.add(Vec2 {
            x: self.vx * ctx.dt,
            y: 0.0,
        });
        self.pos = wrap_x(p, w, ctx.bounds);
        self.pos.y = ctx.bounds.y; // pinned to the top row
    }
    fn sprite(&self) -> PixelSprite {
        let mut s = PixelSprite::from_art(
            &[
                "....ddd......",
                "...ddddd.....",
                "...dddkd.....",
                "...dddddooo..",
                ".ddddddddd...",
                "ddddddddddd..",
                ".ddddddddd...",
                "...ddddd.....",
            ],
            &[
                ('d', Color::Yellow),
                ('o', Color::Orange),
                ('k', Color::Black),
            ],
        );
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
        let s = self.sprite();
        Rect {
            x: self.pos.x,
            y: self.pos.y,
            w: s.cell_w() as f32,
            h: s.cell_h() as f32,
        }
    }
    fn kind(&self) -> Kind {
        Kind::Fish
    }
    fn dead(&self) -> bool {
        self.eaten
    }
    fn on_eaten(&mut self) {
        self.eaten = true;
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
            fish: vec![],
            shark,
        }
    }

    #[test]
    fn nearest_picks_closest_in_radius() {
        let p = Vec2 { x: 0.0, y: 0.0 };
        let pts = vec![Vec2 { x: 10.0, y: 0.0 }, Vec2 { x: 3.0, y: 0.0 }];
        assert_eq!(nearest(p, &pts, 20.0), Some(Vec2 { x: 3.0, y: 0.0 }));
        assert_eq!(nearest(p, &pts, 1.0), None); // nothing within a tiny radius
    }

    #[test]
    fn hunt_radius_is_at_least_fear_radius() {
        // The shark must still target a fish that has just started fleeing,
        // so its hunt reach is never shorter than the fish's fear reach.
        let big = Rect {
            x: 0.0,
            y: 0.0,
            w: 150.0,
            h: 30.0,
        };
        let small = Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 8.0,
        };
        assert!(hunt_radius(big) >= fear_radius(big));
        assert!(hunt_radius(small) >= fear_radius(small));
    }

    #[test]
    fn all_fish_are_mortal() {
        let mut fish: Vec<Box<dyn Entity>> = vec![
            Box::new(Googly::new(Vec2 { x: 1.0, y: 1.0 }, 1.0)),
            Box::new(Cool::new(Vec2 { x: 1.0, y: 1.0 }, 1.0)),
            Box::new(Upsidedown::new(Vec2 { x: 1.0, y: 1.0 }, 1.0)),
            Box::new(Ducky::new(Vec2 { x: 1.0, y: 1.0 }, 1.0)),
        ];
        for f in &mut fish {
            assert!(!f.dead(), "fish should start alive");
            f.on_eaten();
            assert!(f.dead(), "fish should be dead after on_eaten");
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
    fn vertical_wander_pushes_both_up_and_down_but_stays_gentle() {
        let phase = 0.3;
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for i in 0..400 {
            let v = vertical_wander(i as f32 * 0.05, phase);
            min = min.min(v);
            max = max.max(v);
        }
        // It must swing both ways so a fish rises and sinks, not just one.
        assert!(max > 0.5, "wander should push the fish down sometimes");
        assert!(min < -0.5, "wander should push the fish up sometimes");
        // ...but never exceed the gentle peak speed (no frantic bouncing).
        assert!(max <= WANDER_SPEED + 1e-3 && min >= -WANDER_SPEED - 1e-3);
    }

    #[test]
    fn drifting_fish_wanders_up_and_down() {
        // With no food or shark to chase, a fish used to track a perfectly flat
        // line. Now it should bob both above and below where it started.
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            w: 60.0,
            h: 40.0,
        };
        let mk = || TankCtx {
            bounds,
            dt: 0.1,
            food: vec![],
            fish: vec![],
            shark: None,
        };
        let mut f = Googly::new(Vec2 { x: 30.0, y: 20.0 }, 3.0);
        let start_y = f.pos().y;
        let (mut up, mut down) = (false, false);
        for _ in 0..400 {
            f.update(&mk());
            if f.pos().y < start_y - 0.3 {
                up = true;
            }
            if f.pos().y > start_y + 0.3 {
                down = true;
            }
        }
        assert!(
            up && down,
            "a drifting fish should wander both above and below its start"
        );
    }

    #[test]
    fn neighbouring_fish_do_not_bob_in_lockstep() {
        // Two fish spawned at different positions get different wander phases,
        // so their vertical drift is out of sync (no uniform up/down marching).
        let a = Googly::new(Vec2 { x: 10.0, y: 5.0 }, 3.0);
        let b = Googly::new(Vec2 { x: 25.0, y: 12.0 }, 3.0);
        let va = vertical_wander(1.0, a.phase);
        let vb = vertical_wander(1.0, b.phase);
        assert!(
            (va - vb).abs() > 1e-3,
            "fish at different spots should wander out of phase"
        );
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
    fn fish_faces_its_travel_direction() {
        let right = Googly::new(Vec2 { x: 0.0, y: 0.0 }, 3.0);
        let left = Googly::new(Vec2 { x: 0.0, y: 0.0 }, -3.0);
        // Left-facing is the column-reverse of right-facing, row for row.
        let r = right.sprite().rendered_rows();
        let l = left.sprite().rendered_rows();
        for (rr, lr) in r.iter().zip(l.iter()) {
            let mut rev = rr.clone();
            rev.reverse();
            assert_eq!(&rev, lr);
        }
    }

    #[test]
    fn fish_flips_to_face_the_food_it_chases() {
        // Drifting right, but food is to the LEFT and in range: the fish turns to
        // chase it, so its sprite flips from right-facing to left-facing.
        let mut f = Googly::new(Vec2 { x: 20.0, y: 5.0 }, 3.0);
        assert_eq!(f.sprite().rendered_rows()[0], f.sprite().pixels[0]); // facing right
        f.update(&ctx(vec![Vec2 { x: 12.0, y: 5.0 }], None));
        let mut reversed = f.sprite().pixels[0].clone();
        reversed.reverse();
        assert_eq!(f.sprite().rendered_rows()[0], reversed); // flipped to face left
    }

    #[test]
    fn fish_seeks_food_across_a_wide_tank() {
        // In a 150-wide tank, food 32 cells away must still register
        // (a fixed radius of 12 would never notice it).
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            w: 150.0,
            h: 30.0,
        };
        let pellet = Vec2 { x: 48.0, y: 10.0 };
        let c = TankCtx {
            bounds,
            dt: 1.0,
            food: vec![pellet],
            fish: vec![],
            shark: None,
        };
        // Fish at x=80 drifting RIGHT (away from the pellet on its left):
        // only seeking can shrink the distance.
        let mut f = Googly::new(Vec2 { x: 80.0, y: 10.0 }, 3.0);
        let before = f.pos().distance(pellet);
        f.update(&c);
        assert!(f.pos().distance(pellet) < before);
    }

    #[test]
    fn fish_flees_shark_across_a_wide_tank() {
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            w: 150.0,
            h: 30.0,
        };
        let shark = Vec2 { x: 40.0, y: 10.0 };
        let c = TankCtx {
            bounds,
            dt: 1.0,
            food: vec![],
            fish: vec![],
            shark: Some(shark),
        };
        // Fish 20 cells to the shark's right, drifting LEFT (toward it):
        // only fleeing can push it further right.
        let mut f = Googly::new(Vec2 { x: 60.0, y: 10.0 }, -3.0);
        f.update(&c);
        assert!(f.pos().x > 60.0);
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
    fn cool_and_upsidedown_are_fish() {
        let c = Cool::new(Vec2 { x: 3.0, y: 3.0 }, 2.0);
        let u = Upsidedown::new(Vec2 { x: 3.0, y: 8.0 }, 2.0);
        assert_eq!(c.kind(), Kind::Fish);
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
