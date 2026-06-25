use crate::entity::{Entity, Food, Kind, Shark, TankCtx};
use crate::fish::{Cool, Ducky, Googly, Upsidedown};
use crate::geom::{Rect, Vec2};
use crate::render::PixelFrame;

pub struct Tank {
    pub bounds: Rect,
    entities: Vec<Box<dyn Entity>>,
    spawn_counter: usize,
    food_counter: usize,
}

/// Fractional part of `x` (the `{x}` in low-discrepancy sequences).
fn frac(x: f32) -> f32 {
    x - x.floor()
}

impl Tank {
    pub const MAX_FISH: usize = 30;

    pub fn new(width: u16, height: u16) -> Tank {
        Tank {
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                w: width as f32,
                h: height as f32,
            },
            entities: Vec::new(),
            spawn_counter: 0,
            food_counter: 0,
        }
    }

    pub fn add_entity(&mut self, e: Box<dyn Entity>) {
        self.entities.push(e);
    }

    pub fn entities(&self) -> &[Box<dyn Entity>] {
        &self.entities
    }

    /// Draw every entity into `frame` at its rounded cell position. Shared by
    /// the terminal and window frontends so both render identically.
    pub fn draw(&self, frame: &mut PixelFrame) {
        for e in &self.entities {
            let p = e.pos();
            frame.draw_sprite(p.x.round() as i32, p.y.round() as i32, &e.sprite());
        }
    }

    pub fn fish_count(&self) -> usize {
        self.count_kind(Kind::Fish)
    }

    pub fn count_kind(&self, kind: Kind) -> usize {
        self.entities.iter().filter(|e| e.kind() == kind).count()
    }

    /// Spawn a random fish if under the cap; a gentle no-op otherwise.
    ///
    /// Each fish gets a spread-out position, depth, speed, and direction
    /// (derived from a low-discrepancy sequence) so fish never spawn in a
    /// single column or swim in lockstep — which is what made them overlap.
    pub fn add_fish_at(&mut self) {
        if self.fish_count() >= Self::MAX_FISH {
            return;
        }
        let n = self.spawn_counter;
        self.spawn_counter += 1;

        let fx = frac(n as f32 * 0.618_034 + 0.123);
        let fy = frac(n as f32 * 0.381_966 + 0.456);
        let fs = frac(n as f32 * 0.754_877 + 0.789);

        let x = self.bounds.x + fx * (self.bounds.w - 8.0).max(1.0);
        let y = self.bounds.y + fy * (self.bounds.h - 2.0).max(1.0);
        let speed = 1.5 + fs * 2.5; // 1.5..4.0 cells/sec
        let dir = if n.is_multiple_of(2) { 1.0 } else { -1.0 };
        let vx = speed * dir;
        let pos = Vec2 { x, y };

        let fish: Box<dyn Entity> = match n % 4 {
            0 => Box::new(Googly::new(pos, vx)),
            1 => Box::new(Cool::new(pos, vx)),
            2 => Box::new(Upsidedown::new(pos, vx)),
            _ => Box::new(Ducky::new(pos, vx)),
        };
        self.entities.push(fish);
    }

    pub fn update(&mut self, dt: f32) {
        let ctx = self.build_ctx(dt);
        for e in &mut self.entities {
            e.update(&ctx);
        }
        self.resolve_food();
        self.resolve_shark();
        self.entities.retain(|e| !e.dead());
    }

    fn build_ctx(&self, dt: f32) -> TankCtx {
        let food = self
            .entities
            .iter()
            .filter(|e| e.kind() == Kind::Food)
            .map(|e| e.pos())
            .collect();
        let fish = self
            .entities
            .iter()
            .filter(|e| e.kind() == Kind::Fish)
            .map(|e| e.pos())
            .collect();
        let shark = self
            .entities
            .iter()
            .find(|e| e.kind() == Kind::Shark)
            .map(|e| e.pos());
        TankCtx {
            bounds: self.bounds,
            dt,
            food,
            fish,
            shark,
        }
    }

    /// A pellet whose cell overlaps any fish's bounds is eaten.
    fn resolve_food(&mut self) {
        let fish_bounds: Vec<Rect> = self
            .entities
            .iter()
            .filter(|e| e.kind() == Kind::Fish)
            .map(|e| e.bounds())
            .collect();
        for e in &mut self.entities {
            if e.kind() == Kind::Food {
                let b = e.bounds();
                if fish_bounds.iter().any(|fb| fb.overlaps(b)) {
                    e.on_eaten();
                }
            }
        }
    }

    /// A fish whose cell overlaps the shark's bounds is eaten; the shark
    /// counts the kill. Two separate passes (read the shark's bounds, then
    /// mark fish, then bump the shark) avoid aliasing `entities`, mirroring
    /// `resolve_food`.
    fn resolve_shark(&mut self) {
        let shark_bounds = self
            .entities
            .iter()
            .find(|e| e.kind() == Kind::Shark)
            .map(|e| e.bounds());
        let Some(sb) = shark_bounds else {
            return;
        };
        let mut kills = 0;
        for e in &mut self.entities {
            if e.kind() == Kind::Fish && e.bounds().overlaps(sb) {
                e.on_eaten();
                kills += 1;
            }
        }
        if kills > 0 {
            if let Some(shark) = self.entities.iter_mut().find(|e| e.kind() == Kind::Shark) {
                for _ in 0..kills {
                    shark.on_kill();
                }
            }
        }
    }

    /// Drop a pellet from the top at horizontal position `x`.
    pub fn drop_food_at(&mut self, x: f32) {
        self.entities.push(Box::new(Food::new(Vec2 {
            x,
            y: self.bounds.y,
        })));
    }

    /// Drop a pellet at a varied column so repeated feeds spread out rather
    /// than stacking in one line.
    pub fn feed(&mut self) {
        let n = self.food_counter;
        self.food_counter += 1;
        let fx = frac(n as f32 * 0.618_034 + 0.5);
        let x = self.bounds.x + fx * (self.bounds.w - 1.0).max(1.0);
        self.drop_food_at(x);
    }

    /// Summon a shark from the left edge, unless one is already cruising.
    pub fn summon_shark(&mut self) {
        if self.count_kind(Kind::Shark) > 0 {
            return;
        }
        let y = self.bounds.h * 0.5;
        self.entities.push(Box::new(Shark::new(
            Vec2 {
                x: self.bounds.x - 6.0,
                y,
            },
            10.0,
        )));
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.bounds.w = width as f32;
        self.bounds.h = height as f32;
        self.update(0.0); // entities clamp themselves into the new bounds
    }

    // ---- test-only helpers (kept tiny, behind cfg(test)) ----
    #[cfg(test)]
    pub fn entity_positions(&self) -> Vec<Vec2> {
        self.entities.iter().map(|e| e.pos()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fish::{Ducky, Googly};

    fn tank() -> Tank {
        Tank::new(40, 20)
    }

    #[test]
    fn draw_places_entities_at_rounded_positions() {
        use crate::render::PixelFrame;
        let mut t = Tank::new(20, 10);
        t.drop_food_at(5.0); // pellet starts at the top: cell (5, 0)
        let mut frame = PixelFrame::new(20, 10);
        t.draw(&mut frame);
        // The pellet lights at least one pixel within its cell (top-left px = (5, 0)).
        assert!(frame.pixel(5, 0).is_some());
        // A far-away region stays transparent (water).
        assert_eq!(frame.pixel(0, 18), None);
    }

    #[test]
    fn update_ticks_entities() {
        let mut t = tank();
        t.add_entity(Box::new(Googly::new(Vec2 { x: 5.0, y: 5.0 }, 3.0)));
        let before = t.entity_positions()[0];
        t.update(1.0);
        assert_ne!(t.entity_positions()[0], before);
    }

    #[test]
    fn fish_cap_blocks_extra_spawns() {
        let mut t = tank();
        for _ in 0..Tank::MAX_FISH + 5 {
            t.add_fish_at();
        }
        assert_eq!(t.fish_count(), Tank::MAX_FISH);
    }

    #[test]
    fn spawned_fish_spread_across_the_tank() {
        let mut t = Tank::new(150, 30);
        for _ in 0..4 {
            t.add_fish_at();
        }
        let xs: Vec<f32> = t.entity_positions().iter().map(|p| p.x).collect();
        // Fish must not all land in the same column (the old lockstep bug).
        let spread = xs.iter().any(|&x| (x - xs[0]).abs() > 1.0);
        assert!(
            spread,
            "fish should spawn at varied x positions, got {xs:?}"
        );
    }

    #[test]
    fn fish_eats_pellet_as_it_sinks_through() {
        let mut t = tank();
        // A stationary fish (vx = 0) sits in the pellet's column; the pellet
        // sinks through it and gets eaten within a bounded number of ticks.
        t.add_entity(Box::new(Googly::new(Vec2 { x: 10.0, y: 10.0 }, 0.0)));
        t.drop_food_at(10.0); // same column, starts at the top
        let mut eaten = false;
        for _ in 0..100 {
            t.update(0.5);
            if t.count_kind(Kind::Food) == 0 {
                eaten = true;
                break;
            }
        }
        assert!(eaten, "pellet should be eaten as it passes the fish");
    }

    #[test]
    fn summon_shark_adds_one_and_only_one() {
        let mut t = tank();
        t.summon_shark();
        assert_eq!(t.count_kind(Kind::Shark), 1);
        t.summon_shark(); // already present → no-op
        assert_eq!(t.count_kind(Kind::Shark), 1);
    }

    #[test]
    fn dead_entities_are_removed() {
        let mut t = tank();
        t.drop_food_at(5.0);
        assert_eq!(t.count_kind(Kind::Food), 1);
        // Sink the pellet far past the bottom; it should die and be removed.
        for _ in 0..200 {
            t.update(1.0);
        }
        assert_eq!(t.count_kind(Kind::Food), 0);
    }

    #[test]
    fn resize_clamps_entities_into_new_bounds() {
        let mut t = Tank::new(40, 20);
        t.add_entity(Box::new(Googly::new(Vec2 { x: 38.0, y: 18.0 }, 0.0)));
        t.resize(10, 6);
        let p = t.entity_positions()[0];
        assert!(p.x <= 10.0 && p.y <= 6.0);
        assert_eq!(t.bounds.w, 10.0);
        assert_eq!(t.bounds.h, 6.0);
    }

    #[test]
    fn shark_eats_overlapping_fish() {
        let mut t = Tank::new(40, 20);
        t.add_entity(Box::new(Googly::new(Vec2 { x: 5.0, y: 5.0 }, 0.0)));
        t.add_entity(Box::new(Shark::new(Vec2 { x: 4.0, y: 5.0 }, 0.0)));
        assert_eq!(t.count_kind(Kind::Fish), 1);
        t.update(0.016); // one frame: the wide shark overlaps the fish
        assert_eq!(
            t.count_kind(Kind::Fish),
            0,
            "overlapping fish should be eaten"
        );
        // The kill must also register on the shark itself: its body widens one
        // segment per kill. This pins the resolve_shark -> on_kill wiring, which
        // the fish-count check alone misses.
        let fresh = crate::entity::Shark::new(crate::geom::Vec2 { x: 0.0, y: 0.0 }, 0.0);
        let shark = t
            .entities()
            .iter()
            .find(|e| e.kind() == Kind::Shark)
            .expect("shark present");
        assert!(
            shark.sprite().width() > fresh.sprite().width(),
            "shark should fatten after counting the kill"
        );
    }

    #[test]
    fn ducky_is_easy_prey() {
        // Ducky never flees and is pinned to the surface, so a surfacing shark
        // eats it readily (the emergent consequence the design calls out).
        let mut t = Tank::new(40, 20);
        t.add_entity(Box::new(Ducky::new(Vec2 { x: 6.0, y: 0.0 }, 0.0)));
        t.add_entity(Box::new(Shark::new(Vec2 { x: 4.0, y: 0.0 }, 0.0)));
        t.update(0.016);
        assert_eq!(t.count_kind(Kind::Fish), 0);
    }

    #[test]
    fn full_shark_eventually_leaves_the_tank() {
        // End-to-end: a shark that eats its fill through real Tank::update ticks
        // must then cruise off and despawn — the "never empties / always leaves"
        // guarantee, exercised through resolve_shark rather than on_kill directly.
        let mut t = Tank::new(40, 20);
        for _ in 0..3 {
            // Three fish sitting on the shark's path, eaten as it passes.
            t.add_entity(Box::new(Googly::new(Vec2 { x: 4.0, y: 10.0 }, 0.0)));
        }
        t.add_entity(Box::new(Shark::new(Vec2 { x: 0.0, y: 10.0 }, 6.0)));
        for _ in 0..200 {
            t.update(0.5);
        }
        assert_eq!(
            t.count_kind(Kind::Shark),
            0,
            "a full shark should cruise off and despawn"
        );
    }

    #[test]
    fn build_ctx_snapshots_fish_positions() {
        let mut t = Tank::new(40, 20);
        t.add_entity(Box::new(Googly::new(Vec2 { x: 7.0, y: 3.0 }, 0.0)));
        t.add_entity(Box::new(Googly::new(Vec2 { x: 9.0, y: 4.0 }, 0.0)));
        let ctx = t.build_ctx(0.0);
        assert_eq!(ctx.fish.len(), 2);
        assert!(ctx.fish.contains(&Vec2 { x: 7.0, y: 3.0 }));
    }
}
