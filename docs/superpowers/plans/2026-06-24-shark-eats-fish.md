# Shark Eats Fish Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the shark from a harmless cruiser into an active hunter that steers toward the nearest fish, eats fish it overlaps, fattens with each kill, and leaves once full.

**Architecture:** Reuse the existing snapshot-then-mutate update pass. The shark reads a new `TankCtx.fish` position list (read-only) and steers via the same `step_toward`/`clamp_y`/`nearest` helpers fish already use. `Tank` alone marks fish dead and tells the shark it scored (a new `on_kill` trait hook, parallel to the existing `on_eaten`), via a new `resolve_shark` pass that mirrors `resolve_food`. No new entity types, keybindings, or render path.

**Tech Stack:** Rust, `crossterm` (untouched here), `cargo test` (logic-only TDD, no TTY).

**Spec:** `docs/superpowers/specs/2026-06-24-shark-eats-fish-design.md`

**Skills:** Use @superpowers:test-driven-development for every task (RED → GREEN → commit). Verify with @superpowers:verification-before-completion before claiming done.

---

## File Structure

All changes land in three existing modules (plus their in-file `#[cfg(test)]` modules). No new files.

- **`src/entity.rs`** — `TankCtx` gains `fish: Vec<Vec2>`; `Entity` trait gains `on_kill` default hook; `Shark` gains `eaten`/`facing_right` fields, hunting `update`, fattening `sprite`, and `on_kill`. Tank-context test helper updated for the new field.
- **`src/fish.rs`** — all four fish become mortal (`eaten` field + `on_eaten` + `dead`); `nearest_food` is generalized to a `pub fn nearest`; a new `pub fn hunt_radius`. Test helpers updated for the new `TankCtx` field.
- **`src/tank.rs`** — `build_ctx` collects fish positions; new `resolve_shark`; `update` calls it. Test imports extended.

**Key constants (in `src/entity.rs`):** `HUNT_SPEED: f32 = 10.0`, `FULL_AFTER: usize = 3`.

**Cross-module note:** `src/entity.rs` will call `crate::fish::{nearest, hunt_radius, step_toward, clamp_y}`. `src/fish.rs` already calls `crate::entity::*`. Mutual module references within one crate are fine in Rust — this is not a dependency cycle problem.

---

## Task 1: Add fish positions to the world snapshot

Gives the shark a read-only channel to find prey. This must update **every** `TankCtx` construction site or the crate won't compile.

**Files:**
- Modify: `src/entity.rs` — `TankCtx` struct (around line 15) + the `ctx()` test helper (around line 163)
- Modify: `src/tank.rs` — `build_ctx` (around line 91)
- Modify: `src/fish.rs` — the `ctx(...)` test helper (around line 340) and two inline `TankCtx { ... }` literals (around lines 403 and 426)
- Test: `src/tank.rs` test module

- [ ] **Step 1: Write the failing test** (in `src/tank.rs` `#[cfg(test)] mod tests`)

```rust
#[test]
fn build_ctx_snapshots_fish_positions() {
    let mut t = Tank::new(40, 20);
    t.add_entity(Box::new(Googly::new(Vec2 { x: 7.0, y: 3.0 }, 0.0)));
    t.add_entity(Box::new(Googly::new(Vec2 { x: 9.0, y: 4.0 }, 0.0)));
    let ctx = t.build_ctx(0.0);
    assert_eq!(ctx.fish.len(), 2);
    assert!(ctx.fish.contains(&Vec2 { x: 7.0, y: 3.0 }));
}
```

- [ ] **Step 2: Run it — expect a COMPILE failure**

Run: `cargo test --lib build_ctx_snapshots_fish_positions`
Expected: fails to compile — `no field 'fish' on type 'TankCtx'`. (In Rust TDD, a non-compiling test is the RED state.)

- [ ] **Step 3: Add the field and populate it**

In `src/entity.rs`, add to `TankCtx` (keep existing fields):

```rust
pub struct TankCtx {
    pub bounds: Rect,
    pub dt: f32,
    pub food: Vec<Vec2>,     // pellet positions this tick
    pub fish: Vec<Vec2>,     // fish positions this tick (prey for the shark)
    pub shark: Option<Vec2>, // shark position, if one is present
}
```

In `src/tank.rs` `build_ctx`, collect fish alongside food and return the new field:

```rust
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
```

- [ ] **Step 4: Fix the other three construction sites so the crate compiles**

In `src/entity.rs` test helper `ctx()` — add `fish: vec![],` next to `food: vec![],`.
In `src/fish.rs` test helper `ctx(food, shark)` — add `fish: vec![],`.
In `src/fish.rs`, the two inline `TankCtx { ... }` literals (`fish_seeks_food_across_a_wide_tank`, `fish_flees_shark_across_a_wide_tank`) — add `fish: vec![],`.

- [ ] **Step 5: Run the test — expect PASS**

Run: `cargo test --lib build_ctx_snapshots_fish_positions`
Expected: PASS.

- [ ] **Step 6: Run the whole suite — nothing else broke**

Run: `cargo test`
Expected: all green.

- [ ] **Step 7: Commit**

```bash
git add src/entity.rs src/tank.rs src/fish.rs
git commit -m "Add fish positions to TankCtx snapshot"
```

---

## Task 2: Make fish mortal

Each fish gains an `eaten` flag, reusing the same `Entity::on_eaten` hook `Food` already uses. `dead()` returns it instead of hardcoded `false`.

**Files:**
- Modify: `src/fish.rs` — `Googly`, `Cool`, `Upsidedown`, `Ducky` structs, their constructors, and their `Entity` impls
- Test: `src/fish.rs` test module

- [ ] **Step 1: Write the failing test**

```rust
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
```

- [ ] **Step 2: Run it — expect FAIL**

Run: `cargo test --lib all_fish_are_mortal`
Expected: FAIL — assertion `fish should be dead after on_eaten` fails (today `dead()` is always `false`).

- [ ] **Step 3: Implement mortality on all four fish**

For **each** of `Googly`, `Cool`, `Upsidedown`, `Ducky`:

1. Add `eaten: bool,` to the struct.
2. In `new(...)`, initialize `eaten: false,`.
3. In the `Entity` impl, change `fn dead(&self) -> bool { false }` to `fn dead(&self) -> bool { self.eaten }` and add the hook:

```rust
fn on_eaten(&mut self) {
    self.eaten = true;
}
```

Example for `Ducky` (note it has no `facing_right` field — only add `eaten`):

```rust
pub struct Ducky {
    pos: Vec2,
    vx: f32,
    eaten: bool,
}
impl Ducky {
    pub fn new(pos: Vec2, vx: f32) -> Ducky {
        Ducky { pos, vx, eaten: false }
    }
}
```

- [ ] **Step 4: Run the test — expect PASS**

Run: `cargo test --lib all_fish_are_mortal`
Expected: PASS.

- [ ] **Step 5: Full suite stays green**

Run: `cargo test`
Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add src/fish.rs
git commit -m "Make fish mortal via the on_eaten hook"
```

---

## Task 3: Generalize `nearest` and add `hunt_radius`

Expose the closest-target helper for the shark to reuse, and add the tank-scaled hunt radius next to its siblings.

**Files:**
- Modify: `src/fish.rs` — rename `nearest_food` → `pub fn nearest`; update its caller in `swim_step`; add `pub fn hunt_radius`
- Test: `src/fish.rs` test module

- [ ] **Step 1: Write the failing tests**

```rust
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
    let big = Rect { x: 0.0, y: 0.0, w: 150.0, h: 30.0 };
    let small = Rect { x: 0.0, y: 0.0, w: 10.0, h: 8.0 };
    assert!(hunt_radius(big) >= fear_radius(big));
    assert!(hunt_radius(small) >= fear_radius(small));
}
```

- [ ] **Step 2: Run them — expect a COMPILE failure**

Run: `cargo test --lib nearest_picks_closest_in_radius`
Expected: fails to compile — `cannot find function 'nearest'` / `'hunt_radius'`. (RED.)

- [ ] **Step 3: Implement**

Rename the existing private `nearest_food` to a public `nearest` (signature unchanged except the name and the `food` param renamed to `points` for generality):

```rust
/// Nearest position in `points` within `radius`, if any.
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
```

Update the call inside `swim_step` (was `nearest_food(pos, &ctx.food, ...)`):

```rust
} else if let Some(target) = nearest(pos, &ctx.food, sense_radius(ctx.bounds)) {
```

Add `hunt_radius` next to `sense_radius`/`fear_radius`. It is intentionally `pub` (unlike its module-private siblings) because `Shark::update` in `entity.rs` calls it — keep a comment saying so:

```rust
/// How close a fish must be for the shark to commit to hunting it, scaled to
/// the tank like the other radii. `pub` (unlike its siblings) because the
/// shark in `entity.rs` reads it. Kept >= `fear_radius` so a fish that has
/// just begun to flee is, briefly, still a target.
pub fn hunt_radius(bounds: Rect) -> f32 {
    (bounds.w.max(bounds.h) * 0.40).max(16.0)
}
```

- [ ] **Step 4: Run the tests — expect PASS**

Run: `cargo test --lib nearest_picks_closest_in_radius hunt_radius_is_at_least_fear_radius`
Expected: PASS.

- [ ] **Step 5: Existing seek tests still pass (proves the rename rewired cleanly)**

Run: `cargo test`
Expected: all green (incl. `googly_seeks_nearby_food`, `fish_seeks_food_across_a_wide_tank`).

- [ ] **Step 6: Commit**

```bash
git add src/fish.rs
git commit -m "Generalize nearest helper and add hunt_radius"
```

---

## Task 4: Shark gains a kill counter, the `on_kill` hook, and a fattening sprite

State + visual feedback first; the hunting motion comes in Task 5. Keeping `update` as-is here means the shark still cruises, so existing shark tests stay green.

**Files:**
- Modify: `src/entity.rs` — `Entity` trait (`on_kill` default), imports (`Facing`), `HUNT_SPEED`/`FULL_AFTER` consts, `Shark` struct + `new` + `sprite` + `on_kill`
- Test: `src/entity.rs` test module

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn shark_fattens_as_it_eats() {
    let mut s = Shark::new(Vec2 { x: 0.0, y: 5.0 }, 6.0);
    let w0 = s.sprite().width();
    s.on_kill();
    assert!(s.sprite().width() > w0, "shark body should widen per kill");
}
```

- [ ] **Step 2: Run it — expect a COMPILE failure**

Run: `cargo test --lib shark_fattens_as_it_eats`
Expected: fails to compile — `no method named 'on_kill'`. (RED.)

- [ ] **Step 3: Implement**

In `src/entity.rs`, extend the imports:

```rust
use crate::sprite::{Color, Facing, Sprite};
```

Add the trait hook (default no-op) to `Entity`, next to `on_eaten`:

```rust
/// Called by the tank when this entity eats a fish (only Shark acts on it;
/// default is a no-op).
fn on_kill(&mut self) {}
```

Add constants near the top of the file (by `SINK_SPEED` etc.):

```rust
const HUNT_SPEED: f32 = 10.0; // units/sec the shark steers toward prey
const FULL_AFTER: usize = 3; // kills before the shark loses interest and leaves
```

Extend the `Shark` struct and constructor:

```rust
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
```

Replace `Shark::sprite` so the body widens with `eaten` and faces by `facing_right`:

```rust
fn sprite(&self) -> Sprite {
    // Base art faces right. Body widens one segment per kill so fullness is
    // legible at a glance; bold red gives the ASCII real weight.
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
```

Add the `on_kill` override in `impl Entity for Shark`:

```rust
fn on_kill(&mut self) {
    self.eaten += 1;
}
```

(Leave `Shark::update` unchanged in this task. `FULL_AFTER`/`HUNT_SPEED` are unused until Task 5 — if `cargo build` warns about an unused const, that is expected and resolved in Task 5; do not silence it.)

- [ ] **Step 4: Run the test — expect PASS**

Run: `cargo test --lib shark_fattens_as_it_eats`
Expected: PASS.

- [ ] **Step 5: Existing shark tests stay green**

Run: `cargo test`
Expected: all green (incl. `shark_cruises_horizontally`, `shark_dies_after_leaving_the_far_side`). At `eaten == 0` the body is `<#######°>` — identical width to before, so nothing regresses.

- [ ] **Step 6: Commit**

```bash
git add src/entity.rs
git commit -m "Give the shark a kill counter, on_kill hook, and fattening sprite"
```

---

## Task 5: Shark hunts the nearest fish, then cruises off when full

The behavior core. `update` now picks hunting vs. cruising each tick from the snapshot.

**Files:**
- Modify: `src/entity.rs` — `Shark::update`
- Test: `src/entity.rs` test module

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn shark_steers_toward_nearest_fish() {
    let fish = Vec2 { x: 10.0, y: 15.0 }; // below and ahead, within hunt_radius
    let c = TankCtx {
        bounds: Rect { x: 0.0, y: 0.0, w: 40.0, h: 20.0 },
        dt: 1.0,
        food: vec![],
        fish: vec![fish],
        shark: None,
    };
    let mut s = Shark::new(Vec2 { x: 5.0, y: 5.0 }, 6.0);
    let before = s.pos().distance(fish);
    s.update(&c);
    assert!(s.pos().distance(fish) < before, "shark should close on prey");
    assert!(s.pos().y > 5.0, "a pure cruise would keep y fixed; hunting moves it");
}

#[test]
fn full_shark_stops_hunting_and_cruises() {
    let fish = Vec2 { x: 5.0, y: 15.0 }; // off-axis, within hunt_radius
    let c = TankCtx {
        bounds: Rect { x: 0.0, y: 0.0, w: 40.0, h: 20.0 },
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
    assert!((s.pos().y - 5.0).abs() < 1e-4, "full shark ignores the off-axis fish");
    assert!(s.pos().x > 5.0, "full shark cruises by +vx");
}

#[test]
fn hungry_shark_with_no_prey_cruises_off() {
    // No fish present: a still-hungry shark must keep cruising straight and
    // despawn, so it can never sit forever waiting — the tank is never stuck.
    let c = TankCtx {
        bounds: Rect { x: 0.0, y: 0.0, w: 40.0, h: 20.0 },
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
    assert!(s.dead(), "hungry shark with no prey should cruise off and despawn");
}
```

Note: `hungry_shark_with_no_prey_cruises_off` reads `s.eaten` — a private field, but the test lives in `entity.rs`'s own `mod tests`, so it has access. It passes as soon as the cruise branch exists (Task 5), and would also pass against today's pure-cruise `update`; it documents the "leaves even while hungry" guarantee explicitly.

- [ ] **Step 2: Run them — expect FAIL**

Run: `cargo test --lib shark_steers_toward_nearest_fish full_shark_stops_hunting_and_cruises hungry_shark_with_no_prey_cruises_off`
Expected: `shark_steers_toward_nearest_fish` FAILS (today's `update` only moves by `vx`, so `y` never changes). The other two may already pass — that is fine; the suite is green only once all three hold.

- [ ] **Step 3: Implement the mode-based `update`**

```rust
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
```

- [ ] **Step 4: Run the tests — expect PASS**

Run: `cargo test --lib shark_steers_toward_nearest_fish full_shark_stops_hunting_and_cruises hungry_shark_with_no_prey_cruises_off`
Expected: PASS (all three).

- [ ] **Step 5: Existing despawn/cruise tests stay green**

Run: `cargo test`
Expected: all green. `shark_cruises_horizontally` and `shark_dies_after_leaving_the_far_side` use a ctx with no fish → the cruise branch → unchanged behavior. (Their shark stays at `eaten == 0`, so the body width is the original 10 and the off-edge threshold is unchanged.)

- [ ] **Step 6: Commit**

```bash
git add src/entity.rs
git commit -m "Shark hunts nearest fish, cruises off once full"
```

---

## Task 6: Tank resolves shark kills

Wire predation into the world: a `resolve_shark` pass that mirrors `resolve_food`, then call it in `update`.

**Files:**
- Modify: `src/tank.rs` — add `resolve_shark`, call it in `update`, extend test imports
- Test: `src/tank.rs` test module

- [ ] **Step 1: Write the failing tests**

First extend the test imports at the top of `src/tank.rs`'s `mod tests` — change `use crate::fish::Googly;` to:

```rust
use crate::fish::{Ducky, Googly};
```

Then add:

```rust
#[test]
fn shark_eats_overlapping_fish() {
    let mut t = Tank::new(40, 20);
    t.add_entity(Box::new(Googly::new(Vec2 { x: 5.0, y: 5.0 }, 0.0)));
    t.add_entity(Box::new(Shark::new(Vec2 { x: 4.0, y: 5.0 }, 0.0)));
    assert_eq!(t.count_kind(Kind::Fish), 1);
    t.update(0.016); // one frame: the wide shark overlaps the fish
    assert_eq!(t.count_kind(Kind::Fish), 0, "overlapping fish should be eaten");
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
```

- [ ] **Step 2: Run them — expect FAIL**

Run: `cargo test --lib shark_eats_overlapping_fish ducky_is_easy_prey`
Expected: FAIL — fish count stays `1` (nothing eats fish yet).

- [ ] **Step 3: Implement `resolve_shark` and call it**

Add the method to `impl Tank`, modeled on `resolve_food` (two separate passes → no aliasing):

```rust
/// A fish whose bounds overlap the shark's is eaten; the shark counts the kill.
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
```

Call it in `update`, after `resolve_food` and before the `retain` sweep:

```rust
pub fn update(&mut self, dt: f32) {
    let ctx = self.build_ctx(dt);
    for e in &mut self.entities {
        e.update(&ctx);
    }
    self.resolve_food();
    self.resolve_shark();
    self.entities.retain(|e| !e.dead());
}
```

- [ ] **Step 4: Run the tests — expect PASS**

Run: `cargo test --lib shark_eats_overlapping_fish ducky_is_easy_prey`
Expected: PASS.

- [ ] **Step 5: Full suite green**

Run: `cargo test`
Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add src/tank.rs
git commit -m "Resolve shark predation in the tank update pass"
```

---

## Task 7: Final verification

No new code — confirm the whole thing is clean and eyeball the sprite.

**Files:** none (verification only)

- [ ] **Step 1: Full test suite**

Run: `cargo test`
Expected: all tests pass, zero failures.

- [ ] **Step 2: Lint clean**

Run: `cargo clippy --all-targets`
Expected: zero warnings. (In particular, `HUNT_SPEED`/`FULL_AFTER` are now used.)

- [ ] **Step 3: Format clean**

Run: `cargo fmt --check`
Expected: no diff. (If it reports changes, run `cargo fmt` and re-check.)

- [ ] **Step 4: Eyeball the shark sprite (no TTY loop)**

Run: `cargo run --example preview`
Expected: the program prints every entity's sprite and exits. Confirm the shark renders as bold-red `<#######°>` with its fin. (The preview shows the base, `eaten == 0` form; fattening is exercised by the unit tests.)

- [ ] **Step 5: Commit any formatting fixups (if Step 3 changed files)**

```bash
git add -A
git commit -m "cargo fmt"
```

Otherwise nothing to commit — the feature is complete.

---

## Done criteria

- The shark steers toward the nearest fish within `hunt_radius`, eats fish it overlaps, and its body widens per kill.
- After `FULL_AFTER` (3) kills it stops hunting, cruises straight, and despawns off the edge — and a hungry shark with no prey in range also cruises off, so the tank is never fully emptied.
- All four fish are mortal; Ducky is reliably eaten (never flees); fleers can break the hunt radius and survive.
- `cargo test`, `cargo clippy --all-targets`, and `cargo fmt --check` are all clean.
