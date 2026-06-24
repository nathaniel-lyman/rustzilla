use crate::entity::{Entity, Food, Kind, Shark, TankCtx};
use crate::fish::{Googly, Tophat, Upsidedown, Ducky};
use crate::geom::{Rect, Vec2};

pub struct Tank {
    pub bounds: Rect,
    entities: Vec<Box<dyn Entity>>,
    spawn_counter: usize,
}

impl Tank {
    pub const MAX_FISH: usize = 30;

    pub fn new(width: u16, height: u16) -> Tank {
        Tank {
            bounds: Rect { x: 0.0, y: 0.0, w: width as f32, h: height as f32 },
            entities: Vec::new(),
            spawn_counter: 0,
        }
    }

    pub fn add_entity(&mut self, e: Box<dyn Entity>) {
        self.entities.push(e);
    }

    pub fn entities(&self) -> &[Box<dyn Entity>] {
        &self.entities
    }

    pub fn fish_count(&self) -> usize {
        self.count_kind(Kind::Fish)
    }

    pub fn count_kind(&self, kind: Kind) -> usize {
        self.entities.iter().filter(|e| e.kind() == kind).count()
    }

    /// Spawn a fish if under the cap; a gentle no-op otherwise.
    pub fn add_fish_at(&mut self, pos: Vec2) {
        if self.fish_count() >= Self::MAX_FISH {
            return;
        }
        let pos = pos; // top-left anchor
        let fish: Box<dyn Entity> = match self.spawn_counter % 4 {
            0 => Box::new(Googly::new(pos, 3.0)),
            1 => Box::new(Tophat::new(pos, 2.0)),
            2 => Box::new(Upsidedown::new(pos, 3.0)),
            _ => Box::new(Ducky::new(pos, 2.0)),
        };
        self.spawn_counter += 1;
        self.entities.push(fish);
    }

    pub fn update(&mut self, dt: f32) {
        let ctx = self.build_ctx(dt);
        for e in &mut self.entities {
            e.update(&ctx);
        }
        self.resolve_food();
        self.entities.retain(|e| !e.dead());
    }

    fn build_ctx(&self, dt: f32) -> TankCtx {
        let food = self
            .entities
            .iter()
            .filter(|e| e.kind() == Kind::Food)
            .map(|e| e.pos())
            .collect();
        let shark = self
            .entities
            .iter()
            .find(|e| e.kind() == Kind::Shark)
            .map(|e| e.pos());
        TankCtx { bounds: self.bounds, dt, food, shark }
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

    /// Drop a pellet from the top at horizontal position `x`.
    pub fn drop_food_at(&mut self, x: f32) {
        self.entities.push(Box::new(Food::new(Vec2 { x, y: self.bounds.y })));
    }

    /// Summon a shark from the left edge, unless one is already cruising.
    pub fn summon_shark(&mut self) {
        if self.count_kind(Kind::Shark) > 0 {
            return;
        }
        let y = self.bounds.h * 0.5;
        self.entities.push(Box::new(Shark::new(
            Vec2 { x: self.bounds.x - 6.0, y },
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
    use crate::fish::Googly;

    fn tank() -> Tank {
        Tank::new(40, 20)
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
            t.add_fish_at(Vec2 { x: 1.0, y: 1.0 });
        }
        assert_eq!(t.fish_count(), Tank::MAX_FISH);
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
}
