# Rustzilla CLI Aquarium Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `rustzilla`, an elegant and silly terminal aquarium where ASCII fish with visual gags drift around and the user can drop food, add fish, or summon a shark.

**Architecture:** A classic ~15–20 FPS game loop over a `crossterm` terminal canvas. All actors (fish, food, shark) live in a `Vec<Box<dyn Entity>>` inside a `Tank`. Each tick, the tank builds a read-only `TankCtx` snapshot, calls `update()` on every entity, resolves fish-vs-food collisions, and removes dead entities. A double-buffered renderer draws only changed cells. A `TerminalGuard` restores the terminal via `Drop` on any exit path.

**Tech Stack:** Rust (2021 edition), `crossterm` for terminal control, `cargo test` for the logic test suite. No other runtime dependencies.

**Spec:** `docs/superpowers/specs/2026-06-23-rustzilla-cli-aquarium-design.md`

---

## File Structure

| File | Responsibility |
|------|----------------|
| `Cargo.toml` | Crate metadata + `crossterm` dependency |
| `src/main.rs` | Terminal setup/teardown, the game loop |
| `src/geom.rs` | `Vec2`, `Rect` math (pure, fully tested) |
| `src/sprite.rs` | `Sprite` (char grid + facing/flip) and mirroring |
| `src/entity.rs` | `Entity` trait, `Kind`, `TankCtx`; `Food` and `Shark` |
| `src/fish.rs` | The fish cast (Googly, Tophat, Upsidedown, Ducky) + shared movement helpers |
| `src/tank.rs` | `Tank`: holds entities, ticks them, spawns, collisions, fish cap |
| `src/render.rs` | Double-buffered frame → terminal cells |
| `src/input.rs` | Non-blocking key polling → `Action` enum |

`lib.rs` exposes the modules so integration/unit tests can reach them; `main.rs` is the thin binary.

**Conventions used throughout:**
- Positions and velocities are `f32` in cell units (sub-cell movement, rounded at render time).
- A sprite is anchored at its **top-left**; `bounds()` is the `Rect` from `pos()` with the sprite's width/height.
- Time is real wall-clock `dt` (seconds since last tick), passed into `update()`.

---

## Chunk 1: Project skeleton + geometry

### Task 1: Create the cargo project

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/main.rs`

- [ ] **Step 1: Initialize the crate**

Run: `cargo init --name rustzilla` in the project root. This creates `Cargo.toml` and `src/main.rs`. (If `cargo init` complains the directory is not empty or already has files, that's fine — it only adds what's missing.)

- [ ] **Step 2: Pin the dependency and edition**

Replace `Cargo.toml` with:

```toml
[package]
name = "rustzilla"
version = "0.1.0"
edition = "2021"

[dependencies]
crossterm = "0.27"
```

- [ ] **Step 3: Create the library root**

Create `src/lib.rs`:

```rust
pub mod geom;
pub mod sprite;
pub mod entity;
pub mod fish;
pub mod tank;
pub mod render;
pub mod input;
```

- [ ] **Step 4: Stub the modules so the crate compiles**

Create each of these empty files so `cargo build` succeeds before we fill them in: `src/geom.rs`, `src/sprite.rs`, `src/entity.rs`, `src/fish.rs`, `src/tank.rs`, `src/render.rs`, `src/input.rs`. Leave them empty for now.

- [ ] **Step 5: Minimal main**

Replace `src/main.rs`:

```rust
fn main() {
    println!("rustzilla: nothing to see yet");
}
```

- [ ] **Step 6: Verify it builds**

Run: `cargo build`
Expected: compiles with no errors (warnings about empty modules are fine).

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock src/
git commit -m "chore: scaffold rustzilla crate with crossterm"
```

---

### Task 2: `Vec2` vector math

**Files:**
- Modify: `src/geom.rs`

- [ ] **Step 1: Write the failing tests**

Put this in `src/geom.rs`:

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib geom`
Expected: FAIL — `add`, `scaled`, `distance`, `normalized` are not defined.

- [ ] **Step 3: Implement the methods**

Add to `src/geom.rs` (above the test module):

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib geom`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add src/geom.rs
git commit -m "feat: add Vec2 math"
```

---

### Task 3: `Rect` bounds + overlap

**Files:**
- Modify: `src/geom.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/geom.rs`:

```rust
    #[test]
    fn rect_overlap_detects_intersection() {
        let a = Rect { x: 0.0, y: 0.0, w: 4.0, h: 2.0 };
        let b = Rect { x: 3.0, y: 1.0, w: 2.0, h: 2.0 };
        assert!(a.overlaps(b));
    }

    #[test]
    fn rect_overlap_rejects_disjoint() {
        let a = Rect { x: 0.0, y: 0.0, w: 2.0, h: 2.0 };
        let b = Rect { x: 5.0, y: 5.0, w: 1.0, h: 1.0 };
        assert!(!a.overlaps(b));
    }

    #[test]
    fn rect_contains_point() {
        let a = Rect { x: 1.0, y: 1.0, w: 3.0, h: 3.0 };
        assert!(a.contains(Vec2 { x: 2.0, y: 2.0 }));
        assert!(!a.contains(Vec2 { x: 10.0, y: 2.0 }));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib geom`
Expected: FAIL — `Rect` is not defined.

- [ ] **Step 3: Implement `Rect`**

Add to `src/geom.rs` (above the test module):

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    /// Half-open AABB overlap test.
    pub fn overlaps(self, other: Rect) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }

    pub fn contains(self, p: Vec2) -> bool {
        p.x >= self.x
            && p.x < self.x + self.w
            && p.y >= self.y
            && p.y < self.y + self.h
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib geom`
Expected: PASS (6 tests total).

- [ ] **Step 5: Commit**

```bash
git add src/geom.rs
git commit -m "feat: add Rect overlap and contains"
```

---

## Chunk 2: Sprite + rendering primitives

### Task 4: `Sprite` with facing/flip and dimensions

**Files:**
- Modify: `src/sprite.rs`

- [ ] **Step 1: Write the failing tests**

Put this in `src/sprite.rs`:

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Facing {
    Left,
    Right,
}

/// A small fixed grid of characters, drawn from `rows` (which are authored
/// facing right, upright). `facing`/`flip_v` are applied at render time.
#[derive(Clone, Debug)]
pub struct Sprite {
    pub rows: Vec<String>,
    pub facing: Facing,
    pub flip_v: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_come_from_rows() {
        let s = Sprite::new(vec!["><(((°>".into(), " ~~".into()]);
        assert_eq!(s.width(), 7);
        assert_eq!(s.height(), 2);
    }

    #[test]
    fn rendered_rows_mirror_when_facing_left() {
        let mut s = Sprite::new(vec!["<°)))><".into()]);
        s.facing = Facing::Left;
        // Mirrored: reverse the row, then swap paired glyphs 1:1.
        // Length is always preserved (mirroring never adds/removes chars).
        assert_eq!(s.rendered_rows()[0], "><(((°>");
    }

    #[test]
    fn rendered_rows_flip_vertically() {
        let mut s = Sprite::new(vec!["top".into(), "bot".into()]);
        s.flip_v = true;
        assert_eq!(s.rendered_rows(), vec!["bot".to_string(), "top".to_string()]);
    }
}
```

Note: the expected mirror string above assumes the glyph-swap table in Step 3 (`<`↔`>`, `(`↔`)`, etc.). If you change that table, update the expected value to match `mirror_row`'s actual output. Keep the swap table small and documented.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib sprite`
Expected: FAIL — `Sprite::new`, `width`, `height`, `rendered_rows` undefined.

- [ ] **Step 3: Implement `Sprite`**

Add to `src/sprite.rs` (above the test module):

```rust
impl Sprite {
    pub fn new(rows: Vec<String>) -> Sprite {
        Sprite { rows, facing: Facing::Right, flip_v: false }
    }

    pub fn width(&self) -> usize {
        self.rows.iter().map(|r| r.chars().count()).max().unwrap_or(0)
    }

    pub fn height(&self) -> usize {
        self.rows.len()
    }

    /// Rows with facing/flip applied, ready to draw.
    pub fn rendered_rows(&self) -> Vec<String> {
        let mut rows: Vec<String> = match self.facing {
            Facing::Right => self.rows.clone(),
            Facing::Left => self.rows.iter().map(|r| mirror_row(r)).collect(),
        };
        if self.flip_v {
            rows.reverse();
        }
        rows
    }
}

/// Reverse a row and swap direction-sensitive glyphs so a left-facing
/// fish still looks like a fish.
fn mirror_row(row: &str) -> String {
    row.chars().rev().map(mirror_char).collect()
}

fn mirror_char(c: char) -> char {
    match c {
        '<' => '>',
        '>' => '<',
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '/' => '\\',
        '\\' => '/',
        '{' => '}',
        '}' => '{',
        other => other,
    }
}
```

- [ ] **Step 4: Run, then lock the mirror expectation**

Run: `cargo test --lib sprite`
Expected: PASS (3 tests). If you changed the glyph-swap table, reconcile the `facing_left` expected string with `mirror_row`'s actual output.

- [ ] **Step 5: Commit**

```bash
git add src/sprite.rs
git commit -m "feat: add Sprite with facing and vertical flip"
```

---

### Task 5: `Frame` buffer (render target, terminal-free)

**Files:**
- Modify: `src/render.rs`

This task builds only the in-memory frame buffer and its diff logic — both unit-testable without a terminal. Wiring it to `crossterm` happens in Task 11.

- [ ] **Step 1: Write the failing tests**

Put this in `src/render.rs`:

```rust
use crate::sprite::Sprite;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite::Sprite;

    #[test]
    fn blank_frame_is_all_spaces() {
        let f = Frame::new(3, 2);
        assert_eq!(f.cell(0, 0), ' ');
        assert_eq!(f.cell(2, 1), ' ');
    }

    #[test]
    fn draw_sprite_places_chars_and_skips_spaces() {
        let mut f = Frame::new(10, 3);
        let s = Sprite::new(vec!["ab".into()]);
        f.draw_sprite(2, 1, &s);
        assert_eq!(f.cell(2, 1), 'a');
        assert_eq!(f.cell(3, 1), 'b');
        // A leading space in a sprite row must not erase background.
        let bg = Sprite::new(vec![" Z".into()]);
        f.draw_sprite(2, 1, &bg);
        assert_eq!(f.cell(2, 1), 'a'); // space did not overwrite
        assert_eq!(f.cell(3, 1), 'Z');
    }

    #[test]
    fn draw_clips_out_of_bounds() {
        let mut f = Frame::new(4, 2);
        let s = Sprite::new(vec!["XXXX".into()]);
        f.draw_sprite(2, 0, &s); // half off the right edge
        assert_eq!(f.cell(2, 0), 'X');
        assert_eq!(f.cell(3, 0), 'X');
        // No panic = clipping worked.
    }

    #[test]
    fn diff_reports_only_changed_cells() {
        let prev = Frame::new(3, 1);
        let mut next = Frame::new(3, 1);
        next.set(1, 0, 'o');
        let changes = next.diff(&prev);
        assert_eq!(changes, vec![(1, 0, 'o')]);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib render`
Expected: FAIL — `Frame` undefined.

- [ ] **Step 3: Implement `Frame`**

Add to `src/render.rs` (above the test module):

```rust
/// An in-memory grid of characters. Row-major, `width * height` cells.
pub struct Frame {
    pub width: u16,
    pub height: u16,
    cells: Vec<char>,
}

impl Frame {
    pub fn new(width: u16, height: u16) -> Frame {
        Frame {
            width,
            height,
            cells: vec![' '; width as usize * height as usize],
        }
    }

    fn idx(&self, x: u16, y: u16) -> usize {
        y as usize * self.width as usize + x as usize
    }

    pub fn cell(&self, x: u16, y: u16) -> char {
        self.cells[self.idx(x, y)]
    }

    pub fn set(&mut self, x: u16, y: u16, c: char) {
        let i = self.idx(x, y);
        self.cells[i] = c;
    }

    /// Draw a sprite at integer cell (ox, oy). Spaces in the sprite are
    /// transparent. Cells outside the frame are clipped.
    pub fn draw_sprite(&mut self, ox: i32, oy: i32, sprite: &Sprite) {
        for (dy, row) in sprite.rendered_rows().iter().enumerate() {
            let y = oy + dy as i32;
            if y < 0 || y >= self.height as i32 {
                continue;
            }
            for (dx, c) in row.chars().enumerate() {
                if c == ' ' {
                    continue;
                }
                let x = ox + dx as i32;
                if x < 0 || x >= self.width as i32 {
                    continue;
                }
                self.set(x as u16, y as u16, c);
            }
        }
    }

    /// Cells that differ from `prev`, as (x, y, new_char).
    /// Assumes both frames share dimensions.
    pub fn diff(&self, prev: &Frame) -> Vec<(u16, u16, char)> {
        let mut out = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let c = self.cell(x, y);
                if c != prev.cell(x, y) {
                    out.push((x, y, c));
                }
            }
        }
        out
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib render`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src/render.rs
git commit -m "feat: add in-memory Frame buffer with sprite draw and diff"
```

---

## Chunk 3: Entity trait, Tank, and the first fish

### Task 6: `Entity` trait, `Kind`, and `TankCtx`

**Files:**
- Modify: `src/entity.rs`

- [ ] **Step 1: Define the trait and context (no test yet — pure declarations)**

Put this in `src/entity.rs`:

```rust
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
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: compiles (unused-import warnings are acceptable until consumers exist).

- [ ] **Step 3: Commit**

```bash
git add src/entity.rs
git commit -m "feat: add Entity trait, Kind, and TankCtx"
```

---

### Task 7: Shared fish movement helpers

**Files:**
- Modify: `src/fish.rs`

These pure functions are the movement vocabulary every fish reuses. Testing them here keeps each fish's `update()` trivial.

- [ ] **Step 1: Write the failing tests**

Put this in `src/fish.rs`:

```rust
use crate::geom::{Rect, Vec2};

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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib fish`
Expected: FAIL — helpers undefined.

- [ ] **Step 3: Implement the helpers**

Add to `src/fish.rs` (above the test module):

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib fish`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src/fish.rs
git commit -m "feat: add shared fish movement helpers"
```

---

### Task 8: The Googly fish (first `Entity` implementation)

**Files:**
- Modify: `src/fish.rs`

Googly is the baseline drifter: cruises horizontally, wraps at edges, seeks nearby food, flees a nearby shark. The other fish are variations on it.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/fish.rs`:

```rust
    use crate::entity::{Entity, Kind, TankCtx};

    fn ctx(food: Vec<Vec2>, shark: Option<Vec2>) -> TankCtx {
        TankCtx {
            bounds: Rect { x: 0.0, y: 0.0, w: 40.0, h: 20.0 },
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib fish`
Expected: FAIL — `Googly` undefined.

- [ ] **Step 3: Implement `Googly`**

Add near the top of `src/fish.rs` (and add `use crate::entity::{Entity, Kind, TankCtx};` and `use crate::sprite::Sprite;` to the file's imports):

```rust
const SENSE_RADIUS: f32 = 12.0;   // how far a fish notices food
const FEAR_RADIUS: f32 = 10.0;    // how close a shark must be to scare
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

    fn sprite_w(&self) -> f32 {
        self.sprite().width() as f32
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

impl Entity for Googly {
    fn update(&mut self, ctx: &TankCtx) {
        let w = self.sprite_w();
        // 1. Flee a close shark (highest priority).
        if let Some(s) = ctx.shark {
            if self.pos.distance(s) <= FEAR_RADIUS {
                self.pos = step_away(self.pos, s, FLEE_SPEED * ctx.dt);
                self.pos = clamp_y(self.pos, self.sprite().height() as f32, ctx.bounds);
                self.pos = wrap_x(self.pos, w, ctx.bounds);
                return;
            }
        }
        // 2. Otherwise seek nearby food.
        if let Some(target) = nearest_food(self.pos, &ctx.food) {
            self.pos = step_toward(self.pos, target, SEEK_SPEED * ctx.dt);
        } else {
            // 3. Otherwise drift.
            self.pos = self.pos.add(Vec2 { x: self.vx * ctx.dt, y: 0.0 });
        }
        self.pos = clamp_y(self.pos, self.sprite().height() as f32, ctx.bounds);
        self.pos = wrap_x(self.pos, w, ctx.bounds);
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib fish`
Expected: PASS (7 tests).

- [ ] **Step 5: Commit**

```bash
git add src/fish.rs
git commit -m "feat: add Googly fish with drift, seek, and flee"
```

---

### Task 9: The `Tank` — ticking, cap, food collision, removal

**Files:**
- Modify: `src/tank.rs`

- [ ] **Step 1: Write the failing tests**

Put this in `src/tank.rs`:

```rust
use crate::entity::{Entity, Kind, TankCtx};
use crate::geom::{Rect, Vec2};

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
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib tank`
Expected: FAIL — `Tank` and its methods undefined (and `Food`, `add_fish_at` come from later tasks; for now this task depends on Food existing). **Implementation note:** Task 10 (Food) and this task are interdependent. Implement `Food` (Task 10 Step 3) first if the compiler demands it, or stub `drop_food_at`/`set_food_y_for_test` to construct a `Food`. The plan orders Food next; if you prefer, swap Task 9 and Task 10. Either order ends green.

- [ ] **Step 3: Implement `Tank`**

Add to `src/tank.rs` (above the test module). Note `add_fish_at` picks a fish type; until Task 12 adds the full cast it returns a `Googly`, and the random selection is upgraded there.

```rust
use crate::fish::Googly;
use crate::entity::Entity as _;

pub struct Tank {
    pub bounds: Rect,
    entities: Vec<Box<dyn Entity>>,
}

impl Tank {
    pub const MAX_FISH: usize = 30;

    pub fn new(width: u16, height: u16) -> Tank {
        Tank {
            bounds: Rect { x: 0.0, y: 0.0, w: width as f32, h: height as f32 },
            entities: Vec::new(),
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
        // Task 12 upgrades this to a random pick from the full cast.
        self.entities.push(Box::new(Googly::new(pos, 3.0)));
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

    // ---- test-only helpers (kept tiny, behind cfg(test)) ----
    #[cfg(test)]
    pub fn entity_positions(&self) -> Vec<Vec2> {
        self.entities.iter().map(|e| e.pos()).collect()
    }
}
```

Also add, in `src/tank.rs`, the food spawn method (it uses `Food` from Task 10):

```rust
use crate::entity::Food;

impl Tank {
    /// Drop a pellet from the top at horizontal position `x`.
    pub fn drop_food_at(&mut self, x: f32) {
        self.entities.push(Box::new(Food::new(Vec2 { x, y: self.bounds.y })));
    }
}
```

The `fish_eats_pellet_as_it_sinks_through` test (Step 1) deliberately avoids any test-only mutator: it ticks the real sink/collision path until the pellet reaches the fish, so there's no need to poke a pellet's `y` directly.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib tank`
Expected: PASS (4 tests). (Requires Task 10's `Food`.)

- [ ] **Step 5: Commit**

```bash
git add src/tank.rs src/entity.rs
git commit -m "feat: add Tank with ticking, fish cap, food collision, removal"
```

---

## Chunk 4: Food, Shark, and the rest of the cast

### Task 10: `Food` pellet

**Files:**
- Modify: `src/entity.rs`

- [ ] **Step 1: Write the failing tests**

Add a test module to `src/entity.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib entity`
Expected: FAIL — `Food` undefined.

- [ ] **Step 3: Implement `Food`**

Add to `src/entity.rs` (above the test module):

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib entity`
Expected: PASS (3 tests). Now also re-run Task 9: `cargo test --lib tank` → PASS.

- [ ] **Step 5: Commit**

```bash
git add src/entity.rs
git commit -m "feat: add Food pellet with sinking and dissolve"
```

---

### Task 11: `Shark`

**Files:**
- Modify: `src/entity.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/entity.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib entity`
Expected: FAIL — `Shark` undefined.

- [ ] **Step 3: Implement `Shark`**

Add to `src/entity.rs` (above the test module):

```rust
pub struct Shark {
    pos: Vec2,
    vx: f32,
    gone: bool,
}

impl Shark {
    pub fn new(pos: Vec2, vx: f32) -> Shark {
        Shark { pos, vx, gone: false }
    }
}

impl Entity for Shark {
    fn update(&mut self, ctx: &TankCtx) {
        self.pos.x += self.vx * ctx.dt;
        let w = self.sprite().width() as f32;
        // Despawn once fully past the far edge (in its travel direction).
        if self.vx > 0.0 && self.pos.x > ctx.bounds.x + ctx.bounds.w {
            self.gone = true;
        } else if self.vx < 0.0 && self.pos.x + w < ctx.bounds.x {
            self.gone = true;
        }
    }

    fn sprite(&self) -> Sprite {
        let mut s = Sprite::new(vec!["/\\".into(), "<°```>".into()]);
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
```

- [ ] **Step 4: Add `summon_shark` to the tank with a test**

In `src/tank.rs`, add to the `tests` module:

```rust
    #[test]
    fn summon_shark_adds_one_and_only_one() {
        let mut t = tank();
        t.summon_shark();
        assert_eq!(t.count_kind(Kind::Shark), 1);
        t.summon_shark(); // already present → no-op
        assert_eq!(t.count_kind(Kind::Shark), 1);
    }
```

And implement in `src/tank.rs` (add `use crate::entity::Shark;`):

```rust
impl Tank {
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
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS (entity + tank tests green).

- [ ] **Step 6: Commit**

```bash
git add src/entity.rs src/tank.rs
git commit -m "feat: add Shark and tank summon (single shark at a time)"
```

---

### Task 12: The rest of the cast + randomized spawning

**Files:**
- Modify: `src/fish.rs`
- Modify: `src/tank.rs`

Tophat, Upsidedown, and Ducky reuse Googly's movement vocabulary; they differ in sprite and one behavior twist each. To keep `update()` bodies DRY, factor Googly's drift/seek/flee body into a shared free function.

- [ ] **Step 1: Extract the shared swim step (refactor, tests stay green)**

In `src/fish.rs`, add a shared helper that returns the next position for a "normal" fish given its sprite size, current velocity, and whether it fears sharks:

```rust
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
        pos.add(Vec2 { x: vx * ctx.dt, y: 0.0 })
    };
    wrap_x(clamp_y(p, sprite_h, ctx.bounds), sprite_w, ctx.bounds)
}
```

Then rewrite `Googly::update` to delegate:

```rust
    fn update(&mut self, ctx: &TankCtx) {
        let (w, h) = (self.sprite().width() as f32, self.sprite().height() as f32);
        self.pos = swim_step(self.pos, self.vx, w, h, true, ctx);
    }
```

Make `nearest_food`, `FEAR_RADIUS`, etc. visible to `swim_step` (they're already in the same module). Run `cargo test --lib fish` — all 7 prior tests still PASS. Commit this refactor on its own:

```bash
git add src/fish.rs
git commit -m "refactor: extract shared swim_step from Googly"
```

- [ ] **Step 2: Write failing tests for the new fish**

Add to the `tests` module in `src/fish.rs`:

```rust
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
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --lib fish`
Expected: FAIL — `Tophat`, `Upsidedown`, `Ducky` undefined.

- [ ] **Step 4: Implement the three fish**

Add to `src/fish.rs`. Each is a thin struct delegating to `swim_step`:

```rust
pub struct Tophat { pos: Vec2, vx: f32 }
impl Tophat {
    pub fn new(pos: Vec2, vx: f32) -> Tophat { Tophat { pos, vx } }
}
impl Entity for Tophat {
    fn update(&mut self, ctx: &TankCtx) {
        let (w, h) = (self.sprite().width() as f32, self.sprite().height() as f32);
        // Dignified: drifts at half speed.
        self.pos = swim_step(self.pos, self.vx * 0.5, w, h, true, ctx);
    }
    fn sprite(&self) -> Sprite {
        let mut s = Sprite::new(vec![" _o_ ".into(), "<°)))><".into()]);
        s.facing = if self.vx < 0.0 { crate::sprite::Facing::Left } else { crate::sprite::Facing::Right };
        s
    }
    fn pos(&self) -> Vec2 { self.pos }
    fn bounds(&self) -> Rect {
        Rect { x: self.pos.x, y: self.pos.y, w: self.sprite().width() as f32, h: self.sprite().height() as f32 }
    }
    fn kind(&self) -> Kind { Kind::Fish }
    fn dead(&self) -> bool { false }
}

pub struct Upsidedown { pos: Vec2, vx: f32, t: f32 }
impl Upsidedown {
    pub fn new(pos: Vec2, vx: f32) -> Upsidedown { Upsidedown { pos, vx, t: 0.0 } }
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
        s.facing = if self.vx < 0.0 { crate::sprite::Facing::Left } else { crate::sprite::Facing::Right };
        s.flip_v = self.flipped();
        s
    }
    fn pos(&self) -> Vec2 { self.pos }
    fn bounds(&self) -> Rect {
        Rect { x: self.pos.x, y: self.pos.y, w: self.sprite().width() as f32, h: self.sprite().height() as f32 }
    }
    fn kind(&self) -> Kind { Kind::Fish }
    fn dead(&self) -> bool { false }
}

pub struct Ducky { pos: Vec2, vx: f32 }
impl Ducky {
    pub fn new(pos: Vec2, vx: f32) -> Ducky { Ducky { pos, vx } }
}
impl Entity for Ducky {
    fn update(&mut self, ctx: &TankCtx) {
        // Bobs along the surface; fears nothing, ignores food entirely.
        let w = self.sprite().width() as f32;
        let p = self.pos.add(Vec2 { x: self.vx * ctx.dt, y: 0.0 });
        self.pos = wrap_x(p, w, ctx.bounds);
        self.pos.y = ctx.bounds.y; // pinned to the top row
    }
    fn sprite(&self) -> Sprite {
        let mut s = Sprite::new(vec!["_(°)<".into()]);
        s.facing = if self.vx < 0.0 { crate::sprite::Facing::Left } else { crate::sprite::Facing::Right };
        s
    }
    fn pos(&self) -> Vec2 { self.pos }
    fn bounds(&self) -> Rect {
        Rect { x: self.pos.x, y: self.pos.y, w: self.sprite().width() as f32, h: self.sprite().height() as f32 }
    }
    fn kind(&self) -> Kind { Kind::Fish }
    fn dead(&self) -> bool { false }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib fish`
Expected: PASS. (`upsidedown_sprite_flips_periodically` relies on dt=1.0 × 6 ticks crossing the 5.0s flip interval exactly once; if you change the interval constant, keep the loop count on the correct side of it.)

- [ ] **Step 6: Randomize `add_fish_at` across the cast**

In `src/tank.rs`, replace the body of `add_fish_at`'s spawn line with a cheap rotating/pseudo-random pick (no `rand` dependency — keep it dependency-light). Add a `spawn_counter: usize` field to `Tank`, increment it each spawn, and pick by `spawn_counter % 4`:

```rust
        let pos = pos; // top-left anchor
        let fish: Box<dyn Entity> = match self.spawn_counter % 4 {
            0 => Box::new(Googly::new(pos, 3.0)),
            1 => Box::new(Tophat::new(pos, 2.0)),
            2 => Box::new(Upsidedown::new(pos, 3.0)),
            _ => Box::new(Ducky::new(pos, 2.0)),
        };
        self.spawn_counter += 1;
        self.entities.push(fish);
```

Add `use crate::fish::{Googly, Tophat, Upsidedown, Ducky};` and initialize `spawn_counter: 0` in `Tank::new`. Vary the direction by seeding `vx` sign from `spawn_counter` if desired. Run `cargo test --lib` → all PASS.

- [ ] **Step 7: Commit**

```bash
git add src/fish.rs src/tank.rs
git commit -m "feat: add Tophat, Upsidedown, Ducky and randomized spawning"
```

---

## Chunk 5: Terminal, input, render wiring, and the loop

### Task 13: `Action` input mapping

**Files:**
- Modify: `src/input.rs`

- [ ] **Step 1: Write the failing tests**

Put this in `src/input.rs`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Feed,
    AddFish,
    Shark,
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_keys() {
        assert_eq!(action_for_key('f'), Some(Action::Feed));
        assert_eq!(action_for_key('a'), Some(Action::AddFish));
        assert_eq!(action_for_key('s'), Some(Action::Shark));
        assert_eq!(action_for_key('q'), Some(Action::Quit));
    }

    #[test]
    fn unknown_keys_map_to_none() {
        assert_eq!(action_for_key('z'), None);
    }

    #[test]
    fn is_case_insensitive() {
        assert_eq!(action_for_key('Q'), Some(Action::Quit));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib input`
Expected: FAIL — `action_for_key` undefined.

- [ ] **Step 3: Implement the mapping**

Add to `src/input.rs` (above the test module):

```rust
/// Pure key→action mapping (unit-testable without a terminal).
pub fn action_for_key(c: char) -> Option<Action> {
    match c.to_ascii_lowercase() {
        'f' => Some(Action::Feed),
        'a' => Some(Action::AddFish),
        's' => Some(Action::Shark),
        'q' => Some(Action::Quit),
        _ => None,
    }
}
```

- [ ] **Step 4: Add the non-blocking poll (terminal-coupled, no unit test)**

Add to `src/input.rs`:

```rust
use crossterm::event::{self, Event, KeyCode};
use std::time::Duration;

/// Poll for a key without blocking. Returns the mapped action, if any.
/// `Resize` events are surfaced separately by the caller via `poll_event`.
pub fn poll_action(timeout: Duration) -> std::io::Result<Option<Action>> {
    if event::poll(timeout)? {
        if let Event::Key(k) = event::read()? {
            if let KeyCode::Char(c) = k.code {
                return Ok(action_for_key(c));
            }
        }
    }
    Ok(None)
}
```

- [ ] **Step 5: Run tests + build**

Run: `cargo test --lib input` (PASS, 3 tests) and `cargo build` (compiles).

- [ ] **Step 6: Commit**

```bash
git add src/input.rs
git commit -m "feat: add Action mapping and non-blocking key poll"
```

---

### Task 14: `TerminalGuard` (RAII restore) + frame flush

**Files:**
- Modify: `src/render.rs`
- Modify: `src/main.rs`

These are terminal-coupled and verified by running the app, not unit tests.

- [ ] **Step 1: Add the RAII guard and flush to `src/render.rs`**

```rust
use crossterm::{cursor, execute, queue, style::Print, terminal};
use std::io::{Stdout, Write};

/// Enables raw mode + alternate screen on creation and unconditionally
/// restores the terminal in `Drop` — so cleanup runs on normal exit AND
/// during panic unwinding.
pub struct TerminalGuard {
    stdout: Stdout,
}

impl TerminalGuard {
    pub fn enter() -> std::io::Result<TerminalGuard> {
        let mut stdout = std::io::stdout();
        terminal::enable_raw_mode()?;
        execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(TerminalGuard { stdout })
    }

    pub fn stdout(&mut self) -> &mut Stdout {
        &mut self.stdout
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best-effort restore; ignore errors during teardown.
        let _ = execute!(self.stdout, cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

/// Write only the cells that changed since the previous frame.
pub fn flush_diff(out: &mut Stdout, changes: &[(u16, u16, char)]) -> std::io::Result<()> {
    for (x, y, c) in changes {
        queue!(out, cursor::MoveTo(*x, *y), Print(c))?;
    }
    out.flush()
}
```

- [ ] **Step 2: Verify it builds**

Run: `cargo build`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add src/render.rs
git commit -m "feat: add TerminalGuard RAII restore and diff flush"
```

---

### Task 15: The game loop in `main.rs`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Write the loop**

Replace `src/main.rs`:

```rust
use rustzilla::input::{poll_action, Action};
use rustzilla::render::{flush_diff, Frame, TerminalGuard};
use rustzilla::tank::Tank;
use std::time::{Duration, Instant};

fn main() -> std::io::Result<()> {
    let mut guard = TerminalGuard::enter()?;
    let (cols, rows) = crossterm::terminal::size()?;

    let mut tank = Tank::new(cols, rows);
    // Seed a few fish so the tank isn't empty on launch.
    seed_fish(&mut tank, cols, rows);

    let frame_budget = Duration::from_millis(60); // ~16 FPS
    let mut prev = Frame::new(cols, rows);
    let mut last = Instant::now();

    loop {
        let tick_start = Instant::now();

        // --- input ---
        if let Some(action) = poll_action(Duration::from_millis(1))? {
            match action {
                Action::Quit => break,
                Action::Feed => tank.drop_food_at((cols / 2) as f32),
                Action::AddFish => tank.add_fish_at(top_left_spawn(rows)),
                Action::Shark => tank.summon_shark(),
            }
        }

        // --- update ---
        let now = Instant::now();
        let dt = (now - last).as_secs_f32();
        last = now;
        tank.update(dt);

        // --- render ---
        let mut frame = Frame::new(cols, rows);
        for e in tank.entities() {
            let p = e.pos();
            frame.draw_sprite(p.x.round() as i32, p.y.round() as i32, &e.sprite());
        }
        let changes = frame.diff(&prev);
        flush_diff(guard.stdout(), &changes)?;
        prev = frame;

        // --- frame budget ---
        let elapsed = tick_start.elapsed();
        if elapsed < frame_budget {
            std::thread::sleep(frame_budget - elapsed);
        }
    }

    Ok(()) // TerminalGuard restores the terminal as it drops here.
}

fn top_left_spawn(rows: u16) -> rustzilla::geom::Vec2 {
    rustzilla::geom::Vec2 { x: 2.0, y: (rows / 3) as f32 }
}

fn seed_fish(tank: &mut Tank, cols: u16, rows: u16) {
    let _ = cols;
    for i in 0..6 {
        tank.add_fish_at(rustzilla::geom::Vec2 { x: 2.0, y: (2 + i * 2 % rows.max(4)) as f32 });
    }
}
```

- [ ] **Step 2: Build and run a manual smoke test**

Run: `cargo build`
Expected: compiles.

Run: `cargo run` in a real terminal. Verify:
- Fish drift and wrap around the edges, no flicker.
- `f` drops a pellet that sinks; nearby fish chase it and it vanishes on contact.
- `a` adds a fish (up to the cap).
- `s` sends a shark across; fish scatter; Ducky stays put at the surface.
- `q` quits and the terminal is fully restored (cursor visible, normal screen).

Also verify panic-safety: temporarily add `panic!("test")` after the first render, `cargo run`, confirm the terminal is still restored, then remove the panic.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire up the game loop"
```

---

### Task 16: Resize handling + frame-budget polish

**Files:**
- Modify: `src/input.rs`
- Modify: `src/tank.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add a tank resize method with a test**

In `src/tank.rs` tests:

```rust
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
```

Implement in `src/tank.rs`. Since positions live inside entities and the trait has no setter, the simplest correct approach is to clamp on the next `update()` (which already calls `clamp_y`/`wrap_x`). So `resize` only needs to update `bounds`; entities self-correct on the following tick. To make the test pass deterministically, call one zero-dt update at the end of `resize`:

```rust
impl Tank {
    pub fn resize(&mut self, width: u16, height: u16) {
        self.bounds.w = width as f32;
        self.bounds.h = height as f32;
        self.update(0.0); // entities clamp themselves into the new bounds
    }
}
```

Note: `update(0.0)` advances nothing but re-applies `clamp_y`/`wrap_x`. Confirm `wrap_x` with dt-independent logic leaves an in-bounds fish put (it does — it only acts when fully off-screen). Run `cargo test --lib tank` → PASS.

- [ ] **Step 2: Surface resize events from input**

In `src/input.rs`, add an enum the loop can receive and a combined poll:

```rust
pub enum Input {
    Action(Action),
    Resize(u16, u16),
}

pub fn poll_input(timeout: Duration) -> std::io::Result<Option<Input>> {
    if event::poll(timeout)? {
        match event::read()? {
            Event::Key(k) => {
                if let KeyCode::Char(c) = k.code {
                    return Ok(action_for_key(c).map(Input::Action));
                }
            }
            Event::Resize(w, h) => return Ok(Some(Input::Resize(w, h))),
            _ => {}
        }
    }
    Ok(None)
}
```

- [ ] **Step 3: Handle resize in the loop**

In `src/main.rs`, switch from `poll_action` to `poll_input`, and on `Input::Resize(w, h)`: call `tank.resize(w, h)`, then rebuild `prev = Frame::new(w, h)` and update the local `cols`/`rows` (make them `mut`). Clear the screen on resize so stale cells don't linger: `execute!(guard.stdout(), crossterm::terminal::Clear(crossterm::terminal::ClearType::All))?;`

- [ ] **Step 4: Build, test, manual verify**

Run: `cargo test` (all PASS) and `cargo run`, then resize the terminal window — the aquarium should adapt without corruption and without panicking.

- [ ] **Step 5: Commit**

```bash
git add src/input.rs src/tank.rs src/main.rs
git commit -m "feat: handle terminal resize and polish the frame budget"
```

---

### Task 17: Final pass — README, clippy, full suite

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write a short README**

Create `README.md` with: what it is (one silly paragraph), `cargo run` to play, the control table (`f`/`a`/`s`/`q`), and a one-line description of each fish in the cast.

- [ ] **Step 2: Lint and test**

Run: `cargo clippy --all-targets` → fix any warnings it flags (or justify in a comment).
Run: `cargo test` → entire suite PASS.
Run: `cargo fmt` → format the whole crate.

- [ ] **Step 3: Final manual playthrough**

Run: `cargo run`, exercise every control, confirm clean quit and restored terminal.

- [ ] **Step 4: Commit**

```bash
git add README.md src/
git commit -m "docs: add README; chore: clippy + fmt pass"
```

---

## Done

At this point `rustzilla` is a complete, tested, silly CLI aquarium: a trait-object menagerie of gag fish drifting in a double-buffered terminal canvas, with food, a room-clearing shark, an RAII-restored terminal, and a logic suite that runs without a TTY.
