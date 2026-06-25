# Pixel-Art Fish Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the char-based sprite/render pipeline with flat pixel art — half-block (`▀`) truecolor in the terminal, square pixel blocks in the window — keeping every behavior unchanged.

**Architecture:** Expand-contract. First add the new pixel primitives (`PixelSprite`, a pixel `Frame`, a pixel blit) **alongside** the existing char ones so the crate stays green and the pure logic is TDD'd in isolation. Then one coordinated task rewires the cast + both frontends onto the new types, deletes the old char path, and renames the temporary types to their final spec names. Finally, update the docs.

**Tech Stack:** Rust, crossterm (terminal, truecolor `Color::Rgb`), minifb (window, `u32` pixel buffer). No new dependencies.

**Spec:** `docs/superpowers/specs/2026-06-24-pixel-art-fish-design.md`

---

## Strategy & invariants (read first)

- **The world stays in cells.** `Vec2` positions, movement, collisions, the fish cap, and spawning are untouched. Only the art representation and the two renderers change.
- **Coordinate mapping:** 1 cell = 1 pixel wide, 2 pixels tall. A sprite drawn at cell `(ox, oy)` places pixel `(row, col)` at pixel `(ox + col, 2·oy + row)`. Even pixel rows are a cell's **top** half, odd rows its **bottom** half. A sprite `H` px tall occupies `ceil(H/2)` cells.
- **Transparent = water** (`#0A1428`), not the terminal default.
- **Green checkpoints:**
  - Chunk 1 tasks are additive → verify with `cargo test` (full) + `cargo clippy --all-targets` after each.
  - The Chunk 2 rewire is a single coordinated commit; verify `cargo build --all-targets --features gui`, `cargo test`, `cargo clippy --all-targets --features gui`, `cargo fmt --check` all clean before committing.
- **Temporary names** introduced in Chunk 1 (`PixelFrame`, `PixelCell`, `flush_pixels`, `blit_pixels`, `grid_dims_px`) are renamed to their final spec names (`Frame`, `Cell`, `flush_diff`, `blit`, `grid_dims`) in Task 7, replacing the deleted char-era versions.
- **Why no dead-code warnings between Task 6 and Task 7:** after the cast/frontends move to the pixel path, the old `Sprite`/`Style`/`Frame`/`flush_diff`/`blit`/`grid_dims` are briefly unused, but they are all `pub` library items, which Rust does not flag as `dead_code`. The build stays clippy-clean until Task 7 deletes them. (This is why Tasks 4–7 can be one working session with a single green commit at the end.)
- **Do not run `cargo run`** (it hangs). Verify art via `cargo run --example preview` and `cargo build --features gui`.

## File structure

| File | Responsibility after this plan |
|------|-------------------------------|
| `src/sprite.rs` | `Color` palette (10 flat colors) + `PixelSprite` (grid of `Option<Color>`, facing/flip, `from_art`, `rendered_rows`, `cell_w`/`cell_h`). No more `Sprite`/`Style`/`mirror_char`. |
| `src/render.rs` | `Cell { top, bottom }`, pixel `Frame` (`set_pixel`/`pixel`/`draw_sprite`/`diff`/`full_changes`), `flush_diff` emitting `▀` truecolor, `TerminalGuard`. |
| `src/raster.rs` | `BG`, `rgb(Color)->u32`, `grid_dims(px_w,px_h,scale)`, `blit` (square pixel blocks). No font path. |
| `src/font8x8.rs` | **Deleted.** |
| `src/entity.rs`, `src/fish.rs` | `Entity::sprite() -> PixelSprite`; pixel art for each actor; shark fattens by inserting body columns. |
| `src/tank.rs` | `draw(&mut Frame)` (pixel frame); coupled tests rewritten. |
| `src/main.rs` | Full-frame first paint + on resize, then diff. |
| `src/bin/aquarium.rs` | Pixel-block window sizing (`SCALE` = block edge). |
| `examples/preview.rs` | Dump each `PixelSprite` to stdout as ANSI half-blocks. |
| `src/lib.rs` | Drop `pub mod font8x8;`. |
| `README.md`, `CLAUDE.md` | Describe the pixel-art model; note the truecolor requirement. |

---

## Chunk 1: New pixel primitives (additive, green)

### Task 1: Palette + `PixelSprite` in `sprite.rs`

**Files:**
- Modify: `src/sprite.rs` (add palette variants + `PixelSprite`; keep `Sprite`/`Style` for now)
- Modify: `src/render.rs` (extend `to_ct` match for new variants — keep it compiling)
- Modify: `src/raster.rs` (extend `rgb`/`rgb_bold` matches for new variants)

- [ ] **Step 1: Add the four new palette variants and tune RGBs.** In `src/sprite.rs`, extend the existing `Color` enum (keep all six current variants — the target palette is a superset) by adding `Orange`, `Black`, `Grey`, `Belly`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Red,
    Yellow,
    Green,
    Cyan,
    Blue,
    White,
    Orange,
    Black,
    Grey,
    Belly,
}
```

- [ ] **Step 2: Keep the existing matches exhaustive.** Adding variants breaks the four exhaustive `match`es on `Color` in the crate: `render::to_ct`, `raster::{rgb, rgb_bold}`, and `examples/preview.rs::ansi_prefix`. (`cargo test` compiles examples, so missing the last one makes Chunk 1's checkpoints red — `preview.rs` is fully rewritten in Task 6, but it must keep compiling until then.) Add arms so the whole crate still compiles (final truecolor values; these are what the new path will use):
  - `src/render.rs` `to_ct`: this function returns `crossterm::style::Color`. Add arms mapping every variant to `CtColor::Rgb { r, g, b }` using the spec palette (Cyan `#49D0E0`, Blue `#5B8CFF`, Green `#4FCF6F`, Yellow `#F2C641`, Orange `#E8902F`, White `#F2F2F2`, Black `#141414`, Grey `#8A93A0`, Belly `#C9D0D8`, Red `#D84A4A`). Converting the existing six to `Rgb` now is fine — the old `flush_diff` still works with `Rgb` colors.
  - `src/raster.rs` `rgb`: add/replace arms so all ten map to `0x00RRGGBB` per the same palette. `rgb_bold` and `pixel_color` still exist; just add the four new variants to `rgb`/`rgb_bold` (give `rgb_bold` the same values as `rgb` for the four new ones — bold is going away in Task 7).
  - `examples/preview.rs` `ansi_prefix`: this match maps `Color` to an ANSI 16-color code; the four new variants are never actually produced by the old `Sprite` path during Chunk 1, so a catch-all is sufficient. Add `Color::Orange | Color::Black | Color::Grey | Color::Belly => 37,` (white-ish) as a final arm. This file is replaced wholesale in Task 6.

- [ ] **Step 3: Write failing tests for `PixelSprite`.** Append to `src/sprite.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn from_art_maps_palette_and_treats_unknown_as_transparent() {
    let s = PixelSprite::from_art(&[".b.", "bkb"], &[('b', Color::Cyan), ('k', Color::Black)]);
    assert_eq!(s.pixels[0], vec![None, Some(Color::Cyan), None]);
    assert_eq!(s.pixels[1], vec![Some(Color::Cyan), Some(Color::Black), Some(Color::Cyan)]);
}

#[test]
fn from_art_pads_ragged_rows_to_max_width() {
    let s = PixelSprite::from_art(&["bb", "b"], &[('b', Color::Cyan)]);
    assert_eq!(s.width(), 2);
    assert_eq!(s.pixels[1], vec![Some(Color::Cyan), None]); // padded with None
}

#[test]
fn rendered_rows_mirror_columns_when_facing_left() {
    let mut s = PixelSprite::from_art(&["bk."], &[('b', Color::Cyan), ('k', Color::Black)]);
    s.facing = Facing::Left;
    // Column-reversed: ".kb"
    assert_eq!(
        s.rendered_rows()[0],
        vec![None, Some(Color::Black), Some(Color::Cyan)]
    );
}

#[test]
fn rendered_rows_flip_vertically() {
    let mut s = PixelSprite::from_art(&["b", "k"], &[('b', Color::Cyan), ('k', Color::Black)]);
    s.flip_v = true;
    assert_eq!(s.rendered_rows()[0], vec![Some(Color::Black)]);
    assert_eq!(s.rendered_rows()[1], vec![Some(Color::Cyan)]);
}

#[test]
fn cell_dimensions_round_height_up() {
    let s = PixelSprite::from_art(&["b", "b", "b"], &[('b', Color::Cyan)]); // 1x3 px
    assert_eq!(s.cell_w(), 1);
    assert_eq!(s.cell_h(), 2); // ceil(3/2)
}
```

- [ ] **Step 4: Run the tests, expect failure.** Run: `cargo test --lib sprite` — Expected: FAIL (`PixelSprite` not found).

- [ ] **Step 5: Implement `PixelSprite`.** In `src/sprite.rs`, keep `Facing`/`Sprite`/`Style` as they are and add:

```rust
/// A grid of optional palette colors (None = transparent). Authored in pixels,
/// facing right and upright; `facing`/`flip_v` are applied at render time.
#[derive(Clone, Debug)]
pub struct PixelSprite {
    pub pixels: Vec<Vec<Option<Color>>>,
    pub facing: Facing,
    pub flip_v: bool,
}

impl PixelSprite {
    /// Build from palette-indexed string rows. Any char not in `map` (including
    /// '.' and ' ') is transparent. Ragged rows are right-padded with None.
    /// A mistyped palette char becomes transparent rather than panicking; the
    /// `every_entity_sprite_is_well_formed` test is the safety net for blanks.
    pub fn from_art(rows: &[&str], map: &[(char, Color)]) -> PixelSprite {
        let width = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
        let lookup = |c: char| map.iter().find(|(k, _)| *k == c).map(|(_, v)| *v);
        let pixels = rows
            .iter()
            .map(|row| {
                let mut r: Vec<Option<Color>> = row.chars().map(lookup).collect();
                r.resize(width, None);
                r
            })
            .collect();
        PixelSprite { pixels, facing: Facing::Right, flip_v: false }
    }

    pub fn width(&self) -> usize {
        self.pixels.first().map(|r| r.len()).unwrap_or(0)
    }

    pub fn height(&self) -> usize {
        self.pixels.len()
    }

    /// Width in terminal cells (1 cell = 1 pixel wide).
    pub fn cell_w(&self) -> usize {
        self.width()
    }

    /// Height in terminal cells (1 cell = 2 pixels tall, rounded up).
    pub fn cell_h(&self) -> usize {
        self.height().div_ceil(2)
    }

    /// Pixel rows with facing (column mirror) then flip_v (row reversal) applied.
    pub fn rendered_rows(&self) -> Vec<Vec<Option<Color>>> {
        let mut rows: Vec<Vec<Option<Color>>> = match self.facing {
            Facing::Right => self.pixels.clone(),
            Facing::Left => self
                .pixels
                .iter()
                .map(|r| r.iter().rev().copied().collect())
                .collect(),
        };
        if self.flip_v {
            rows.reverse();
        }
        rows
    }
}
```

- [ ] **Step 6: Run the tests, expect pass.** Run: `cargo test --lib sprite` — Expected: PASS. Then `cargo clippy --all-targets` — Expected: zero warnings.

- [ ] **Step 7: Commit.**

```bash
git add src/sprite.rs src/render.rs src/raster.rs examples/preview.rs
git commit -m "feat: add PixelSprite + palette colors alongside the char sprite"
```

---

### Task 2: Pixel `Frame` in `render.rs`

**Files:**
- Modify: `src/render.rs` (add `PixelCell`, `PixelFrame`, `flush_pixels` alongside the existing `Frame`/`flush_diff`)

- [ ] **Step 1: Write failing tests.** Append to `src/render.rs` tests:

```rust
#[test]
fn pixel_frame_starts_transparent() {
    let f = PixelFrame::new(2, 2);
    assert_eq!(f.pixel(0, 0), None);
    assert_eq!(f.pixel(1, 3), None); // bottom half of the bottom-right cell
}

#[test]
fn set_pixel_targets_top_and_bottom_halves() {
    let mut f = PixelFrame::new(1, 1);
    f.set_pixel(0, 0, Color::Cyan); // even py -> top
    f.set_pixel(0, 1, Color::Red); // odd py -> bottom
    assert_eq!(f.pixel(0, 0), Some(Color::Cyan));
    assert_eq!(f.pixel(0, 1), Some(Color::Red));
}

#[test]
fn set_pixel_clips_out_of_range() {
    let mut f = PixelFrame::new(1, 1);
    f.set_pixel(5, 5, Color::Cyan); // out of bounds: no panic, no write
    f.set_pixel(-1, -1, Color::Cyan);
    assert_eq!(f.pixel(0, 0), None);
}

#[test]
fn draw_sprite_places_pixels_and_skips_transparent() {
    let mut f = PixelFrame::new(3, 2); // canvas 3x4 px
    let s = crate::sprite::PixelSprite::from_art(&[".b", "k."], &[('b', Color::Cyan), ('k', Color::Black)]);
    f.draw_sprite(0, 0, &s); // top-left at pixel (0,0)
    assert_eq!(f.pixel(0, 0), None); // '.' transparent
    assert_eq!(f.pixel(1, 0), Some(Color::Cyan)); // 'b' at row0,col1
    assert_eq!(f.pixel(0, 1), Some(Color::Black)); // 'k' at row1,col0 -> py=1 (bottom half)
}

#[test]
fn draw_sprite_transparent_pixel_does_not_erase() {
    let mut f = PixelFrame::new(2, 1);
    f.set_pixel(0, 0, Color::Cyan);
    let s = crate::sprite::PixelSprite::from_art(&["."], &[]); // single transparent pixel
    f.draw_sprite(0, 0, &s);
    assert_eq!(f.pixel(0, 0), Some(Color::Cyan)); // untouched
}

#[test]
fn diff_reports_a_cell_when_either_half_changes() {
    let prev = PixelFrame::new(1, 1);
    let mut next = PixelFrame::new(1, 1);
    next.set_pixel(0, 1, Color::Red); // only the bottom half changes
    let changes = next.diff(&prev);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].2.bottom, Some(Color::Red));
}

#[test]
fn full_changes_returns_every_cell() {
    let f = PixelFrame::new(3, 2);
    assert_eq!(f.full_changes().len(), 6);
}
```

- [ ] **Step 2: Run, expect failure.** Run: `cargo test --lib render` — Expected: FAIL (`PixelFrame` not found).

- [ ] **Step 3: Implement.** Add to `src/render.rs` (above the existing `tests` module, after the `use crossterm::...` block so the truecolor helpers are in scope):

```rust
use crate::sprite::{Color, PixelSprite};

/// One terminal cell as two vertically-stacked pixels (None = water).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelCell {
    pub top: Option<Color>,
    pub bottom: Option<Color>,
}

impl PixelCell {
    fn blank() -> PixelCell {
        PixelCell { top: None, bottom: None }
    }
}

/// An in-memory grid of pixel cells. Addressed in cells; the pixel canvas is
/// `width × height*2` (each cell holds a top and bottom pixel).
pub struct PixelFrame {
    pub width: u16,
    pub height: u16,
    cells: Vec<PixelCell>,
}

impl PixelFrame {
    pub fn new(width: u16, height: u16) -> PixelFrame {
        PixelFrame {
            width,
            height,
            cells: vec![PixelCell::blank(); width as usize * height as usize],
        }
    }

    fn idx(&self, x: u16, y: u16) -> usize {
        y as usize * self.width as usize + x as usize
    }

    /// Read the pixel at pixel coords (py: 0..height*2). Top half = even py.
    pub fn pixel(&self, px: u16, py: u16) -> Option<Color> {
        if px >= self.width || py >= self.height * 2 {
            return None;
        }
        let cell = self.cells[self.idx(px, py / 2)];
        if py % 2 == 0 { cell.top } else { cell.bottom }
    }

    /// Set the pixel at pixel coords; out-of-range is clipped (no panic).
    pub fn set_pixel(&mut self, px: i32, py: i32, color: Color) {
        if px < 0 || py < 0 || px >= self.width as i32 || py >= self.height as i32 * 2 {
            return;
        }
        let i = self.idx(px as u16, (py as u16) / 2);
        if py % 2 == 0 {
            self.cells[i].top = Some(color);
        } else {
            self.cells[i].bottom = Some(color);
        }
    }

    /// Draw a sprite with its top-left pixel at cell (ox, oy) -> pixel (ox, 2*oy).
    /// Transparent pixels are skipped; everything clips to the frame.
    pub fn draw_sprite(&mut self, ox: i32, oy: i32, sprite: &PixelSprite) {
        for (row, pixels) in sprite.rendered_rows().iter().enumerate() {
            for (col, px) in pixels.iter().enumerate() {
                if let Some(color) = px {
                    self.set_pixel(ox + col as i32, 2 * oy + row as i32, *color);
                }
            }
        }
    }

    /// Cells that differ from `prev`, as (x, y, new_cell).
    pub fn diff(&self, prev: &PixelFrame) -> Vec<(u16, u16, PixelCell)> {
        let mut out = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let i = self.idx(x, y);
                if self.cells[i] != prev.cells[i] {
                    out.push((x, y, self.cells[i]));
                }
            }
        }
        out
    }

    /// Every cell as a change — forces a full repaint (startup / post-resize).
    pub fn full_changes(&self) -> Vec<(u16, u16, PixelCell)> {
        let mut out = Vec::with_capacity(self.cells.len());
        for y in 0..self.height {
            for x in 0..self.width {
                out.push((x, y, self.cells[self.idx(x, y)]));
            }
        }
        out
    }
}

/// Water background as a crossterm truecolor (matches `raster::BG`).
fn water() -> CtColor {
    CtColor::Rgb { r: 0x0A, g: 0x14, b: 0x28 }
}

fn half_rgb(c: Option<Color>) -> CtColor {
    c.map(to_ct).unwrap_or_else(water)
}

/// Write changed cells as half-blocks: fg = top pixel, bg = bottom pixel,
/// transparent halves render as water. One reset at the end (every cell sets
/// both fg and bg, so styles never bleed between cells).
pub fn flush_pixels(out: &mut Stdout, changes: &[(u16, u16, PixelCell)]) -> std::io::Result<()> {
    use crossterm::style::SetBackgroundColor;
    for (x, y, cell) in changes {
        queue!(out, cursor::MoveTo(*x, *y))?;
        queue!(out, SetForegroundColor(half_rgb(cell.top)))?;
        queue!(out, SetBackgroundColor(half_rgb(cell.bottom)))?;
        queue!(out, Print('▀'))?;
    }
    queue!(out, ResetColor)?;
    out.flush()
}
```

Note: the existing `use crossterm::style::{...}` import list needs `Color as CtColor` (already imported) and `SetForegroundColor` (already imported); add `SetBackgroundColor` either to that `use` or locally as shown.

- [ ] **Step 4: Run, expect pass.** Run: `cargo test --lib render` — Expected: PASS. Then `cargo clippy --all-targets` — zero warnings (`flush_pixels` is `pub`, so no dead-code warning).

- [ ] **Step 5: Commit.**

```bash
git add src/render.rs
git commit -m "feat: add pixel Frame (top/bottom cells, half-block flush) alongside char Frame"
```

---

### Task 3: Pixel blit in `raster.rs`

**Files:**
- Modify: `src/raster.rs` (add `grid_dims_px` + `blit_pixels` alongside the existing font `blit`)

- [ ] **Step 1: Write failing tests.** Append to `src/raster.rs` tests:

```rust
#[test]
fn grid_dims_px_uses_tall_cells() {
    // Cells are `scale` wide and `2*scale` tall, so vertical divides by 2*scale.
    assert_eq!(grid_dims_px(240, 240, 6), (40, 20));
    assert_eq!(grid_dims_px(10, 10, 6), (1, 1)); // clamps to >= 1
}

#[test]
fn blit_pixels_blank_frame_is_all_water() {
    let f = crate::render::PixelFrame::new(3, 2);
    let buf = blit_pixels(&f, 4, 12, 16);
    assert!(buf.iter().all(|&p| p == BG));
}

#[test]
fn blit_pixels_lights_correct_half_blocks() {
    let mut f = crate::render::PixelFrame::new(2, 1); // canvas 2x2 px
    f.set_pixel(0, 0, Color::Cyan); // top half of cell (0,0)
    f.set_pixel(0, 1, Color::Red); // bottom half of cell (0,0)
    let buf = blit_pixels(&f, 1, 2, 2); // scale 1 -> 1px blocks; buffer 2x2
    assert_eq!(buf[0 * 2 + 0], rgb(Color::Cyan)); // (0,0) top
    assert_eq!(buf[1 * 2 + 0], rgb(Color::Red)); // (0,1) bottom
    assert_eq!(buf[0 * 2 + 1], BG); // neighbour cell stays water
}

#[test]
fn blit_pixels_clips_past_the_edge() {
    let mut f = crate::render::PixelFrame::new(2, 2);
    f.set_pixel(1, 3, Color::Cyan);
    let buf = blit_pixels(&f, 2, 3, 3); // deliberately too-small buffer
    assert_eq!(buf.len(), 3 * 3); // reached here = no panic
}
```

- [ ] **Step 2: Run, expect failure.** Run: `cargo test --lib raster` — Expected: FAIL.

- [ ] **Step 3: Implement.** Add to `src/raster.rs`:

```rust
/// Window pixels -> tank grid (cols, rows). Cells are `scale` wide and
/// `2*scale` tall so on-screen pixels stay square. Clamps each dim to >= 1.
pub fn grid_dims_px(px_w: usize, px_h: usize, scale: usize) -> (u16, u16) {
    let cols = (px_w / scale.max(1)).max(1) as u16;
    let rows = (px_h / (2 * scale.max(1))).max(1) as u16;
    (cols, rows)
}

/// Render a `PixelFrame` into a `px_w × px_h` buffer. Each pixel is a
/// `scale × scale` square block; the top pixel sits at window-y `2*cy*scale`,
/// the bottom at `(2*cy+1)*scale`. Transparent pixels leave water (`BG`).
/// Blocks past the buffer edge clip — never a panic.
pub fn blit_pixels(
    frame: &crate::render::PixelFrame,
    scale: usize,
    px_w: usize,
    px_h: usize,
) -> Vec<u32> {
    let mut buf = vec![BG; px_w * px_h];
    let s = scale.max(1);
    let mut paint = |sub_x: usize, sub_y: usize, color: u32| {
        for dy in 0..s {
            let y = sub_y * s + dy;
            if y >= px_h {
                break;
            }
            for dx in 0..s {
                let x = sub_x * s + dx;
                if x >= px_w {
                    break;
                }
                buf[y * px_w + x] = color;
            }
        }
    };
    for cy in 0..frame.height {
        for cx in 0..frame.width {
            if let Some(c) = frame.pixel(cx, 2 * cy) {
                paint(cx as usize, (2 * cy) as usize, rgb(c));
            }
            if let Some(c) = frame.pixel(cx, 2 * cy + 1) {
                paint(cx as usize, (2 * cy + 1) as usize, rgb(c));
            }
        }
    }
    buf
}
```

- [ ] **Step 4: Run, expect pass.** Run: `cargo test --lib raster` — Expected: PASS. `cargo clippy --all-targets` — zero warnings.

- [ ] **Step 5: Commit.**

```bash
git add src/raster.rs
git commit -m "feat: add square-block pixel blit alongside the font blit"
```

---

## Chunk 2: Rewire the cast + frontends, delete the char path, docs

### Task 4: Author the pixel art and switch `Entity::sprite()` (lib only)

This task changes the `Entity::sprite()` return type to `PixelSprite` and rewires `tank.draw`. It keeps **`cargo test --lib`** green. The binaries/example break here and are fixed in Task 5 & 6 — **commit Tasks 4, 5, 6 together** (or run them back-to-back and commit once at the end of Task 6) so no commit leaves `cargo test` (full) red. The simplest path: do Tasks 4–7 as one working session, verify everything at the end of Task 7, commit once.

**Files:**
- Modify: `src/entity.rs` (Food + Shark sprites → pixel; `sprite()` signature; `bounds()` via `cell_w/cell_h`)
- Modify: `src/fish.rs` (the four fish sprites → pixel; `bounds()`; movement uses `cell_w/cell_h`; rewrite facing tests)
- Modify: `src/tank.rs` (`draw(&mut PixelFrame)`; rewrite the two coupled tests)
- Modify: `src/entity.rs` test (`shark_fattens_as_it_eats` still valid; confirm)

- [ ] **Step 1: Change the trait return type.** In `src/entity.rs`, change `fn sprite(&self) -> Sprite;` to `fn sprite(&self) -> PixelSprite;` and update the `use crate::sprite::...` line to import `PixelSprite` (drop `Sprite`, `Style`; keep `Color`, `Facing`).

- [ ] **Step 2: Food sprite.** Replace `Food::sprite` body:

```rust
fn sprite(&self) -> PixelSprite {
    // A small 2x2 orange pellet.
    PixelSprite::from_art(&["oo", "oo"], &[('o', Color::Orange)])
}
```

And `Food::bounds` uses the cell footprint:

```rust
fn bounds(&self) -> Rect {
    let s = self.sprite();
    Rect { x: self.pos.x, y: self.pos.y, w: s.cell_w() as f32, h: s.cell_h() as f32 }
}
```

- [ ] **Step 3: Shark sprite + fatten.** Replace `Shark::sprite`. Add a free helper in `entity.rs`:

```rust
/// Build the shark's pixel rows; the mid-body widens one column per kill.
fn shark_rows(eaten: usize) -> Vec<String> {
    let m = 7 + eaten; // mid-body width (parallels the old "#".repeat(7 + eaten))
    // (tail-left fixed, mid stretch, head-right fixed). 'g' body, 'e' belly,
    // 'k' eye, 'r' mouth.
    let tail = ["...", "..g", ".gg", "ggg", "ggg", ".ee", "..e", "..."];
    let mid = ['.', 'g', 'g', 'g', 'g', 'e', 'e', '.'];
    let head = [".ggg.", "ggggg", "ggkgg", "ggggg", "ggggr", "eeeee", ".eee.", "....."];
    (0..8)
        .map(|r| format!("{}{}{}", tail[r], mid[r].to_string().repeat(m), head[r]))
        .collect()
}
```

```rust
fn sprite(&self) -> PixelSprite {
    let rows = shark_rows(self.eaten);
    let refs: Vec<&str> = rows.iter().map(|s| s.as_str()).collect();
    let mut s = PixelSprite::from_art(
        &refs,
        &[('g', Color::Grey), ('e', Color::Belly), ('k', Color::Black), ('r', Color::Red)],
    );
    s.facing = if self.facing_right { Facing::Right } else { Facing::Left };
    s
}
```

And `Shark::bounds` uses `cell_w/cell_h` (same pattern as Food). The shark's `update` reads `self.sprite().width()`/`.height()` for `w`/`h`; change those to `cell_w()`/`cell_h()`.

- [ ] **Step 4: Fish sprites.** In `src/fish.rs`, update the `use` to import `PixelSprite` (drop `Sprite`). For each fish, replace `sprite()` and make `bounds()` + the `swim_step` width/height use `cell_w()/cell_h()`. Art (body `b`, white `w`, black `k`, orange `o`):

```rust
// Googly — cyan body, big white eye with black pupil.
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
```

```rust
// Cool — blue body, black shades bar. (No bold; the bar carries the read.)
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
```

```rust
// Upside-down — green body, small eye; flip_v toggled by the 5s timer.
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
```

```rust
// Ducky — yellow duck, orange beak, black eye. Faced by vx, pinned to surface.
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
        &[('d', Color::Yellow), ('o', Color::Orange), ('k', Color::Black)],
    );
    s.facing = if self.vx < 0.0 { Facing::Left } else { Facing::Right };
    s
}
```

For each fish's `update`, the `(w, h)` it passes to `swim_step` becomes `(self.sprite().cell_w() as f32, self.sprite().cell_h() as f32)`; each `bounds()` uses `cell_w()/cell_h()` like Food above.

- [ ] **Step 5: `tank.draw` → `PixelFrame`.** In `src/tank.rs`, change `use crate::render::Frame;` to `use crate::render::PixelFrame;` and `pub fn draw(&self, frame: &mut Frame)` to `&mut PixelFrame`. The body is unchanged (`frame.draw_sprite(p.x.round() as i32, p.y.round() as i32, &e.sprite())`).

- [ ] **Step 6: Rewrite the three coupled tests** (per the spec's Testing section):
  - `tank.rs::draw_places_entities_at_rounded_positions`:

```rust
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
```

  - `tank.rs::shark_eats_overlapping_fish`: replace the `width() == 11` assertion with a comparison to a fresh shark (no magic number):

```rust
    let fresh = crate::entity::Shark::new(crate::geom::Vec2 { x: 0.0, y: 0.0 }, 0.0);
    let shark = t.entities().iter().find(|e| e.kind() == Kind::Shark).expect("shark present");
    assert!(
        shark.sprite().width() > fresh.sprite().width(),
        "shark should fatten after counting the kill"
    );
```

  - `entity.rs::shark_fattens_as_it_eats` already uses a relative comparison (`sprite().width() > w0`) — it stays as-is; just confirm it compiles and passes (width now means pixels, still strictly increases per kill).

- [ ] **Step 7: Rewrite the fish facing tests** in `src/fish.rs`. The old assertions compared mirrored strings; assert mirrored **pixel rows** instead. Replace `fish_faces_its_travel_direction`, `fish_flips_to_face_the_food_it_chases`, and any sibling that reads `sprite().rendered_rows()[0]` as a string. Example:

```rust
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
```

For `fish_flips_to_face_the_food_it_chases`, keep the behavioral setup and assert the heading via the rendered first row vs the authored `pixels[0]` (`pixels` is `pub`). Facing-right renders `pixels[0]` as-is; facing-left renders it column-reversed:

```rust
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
```

- [ ] **Step 8: Verify lib tests.** Run: `cargo test --lib` — Expected: PASS (all movement/collision/cap/spread/facing/frame/raster/sprite tests green). The bins/example do not compile yet — that is expected and fixed next. Do **not** commit yet.

---

### Task 5: Fix the frontends (`main.rs`, `aquarium.rs`)

**Files:**
- Modify: `src/main.rs`
- Modify: `src/bin/aquarium.rs`

- [ ] **Step 1: `main.rs` — pixel frame + forced first paint.** Update imports to `use rustzilla::render::{flush_pixels, PixelFrame};`. Replace `Frame::new` with `PixelFrame::new`. Add `let mut needs_full = true;` before the loop. In the resize branch, after rebuilding `prev`, set `needs_full = true;` (the `Clear(All)` already there stays). In the render section:

```rust
let mut frame = PixelFrame::new(cols, rows);
tank.draw(&mut frame);
let changes = if needs_full { frame.full_changes() } else { frame.diff(&prev) };
flush_pixels(guard.stdout(), &changes)?;
needs_full = false;
prev = frame;
```

- [ ] **Step 2: `aquarium.rs` — pixel-block sizing.** Update imports to `use rustzilla::raster::{self, blit_pixels};` and `use rustzilla::render::PixelFrame;`. Replace the cell constants:

```rust
const SCALE: usize = 6; // each pixel is a 6x6 block; cells are 6 wide x 12 tall
```

Drop `CELL`. Replace `raster::grid_dims(px_w, px_h, CELL, CELL)` (both call sites) with `raster::grid_dims_px(px_w, px_h, SCALE)`. Replace the frame/blit with:

```rust
let mut frame = PixelFrame::new(cols, rows);
tank.draw(&mut frame);
let buf = blit_pixels(&frame, SCALE, px_w, px_h);
window.update_with_buffer(&buf, px_w, px_h).expect("failed to present frame");
```

(The initial `960×600` window stays; with `SCALE=6` that's 160×50 cells.)

- [ ] **Step 2b: Build the gui bin.** Run: `cargo build --features gui --bin aquarium` — Expected: compiles (does not launch).

---

### Task 6: Rewrite `examples/preview.rs`

**Files:**
- Modify: `examples/preview.rs`

- [ ] **Step 1: Read the current preview** to match its entity-iteration shape, then rewrite it to print each entity's `PixelSprite` to stdout as ANSI half-blocks (two pixel rows per text line):

```rust
use rustzilla::sprite::{Color, PixelSprite};

fn rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Cyan => (0x49, 0xD0, 0xE0),
        Color::Blue => (0x5B, 0x8C, 0xFF),
        Color::Green => (0x4F, 0xCF, 0x6F),
        Color::Yellow => (0xF2, 0xC6, 0x41),
        Color::Orange => (0xE8, 0x90, 0x2F),
        Color::White => (0xF2, 0xF2, 0xF2),
        Color::Black => (0x14, 0x14, 0x14),
        Color::Grey => (0x8A, 0x93, 0xA0),
        Color::Belly => (0xC9, 0xD0, 0xD8),
        Color::Red => (0xD8, 0x4A, 0x4A),
    }
}
const WATER: (u8, u8, u8) = (0x0A, 0x14, 0x28);

fn print_sprite(name: &str, s: &PixelSprite) {
    println!("\n{name}:");
    let rows = s.rendered_rows();
    let mut y = 0;
    while y < rows.len() {
        for x in 0..s.width() {
            let top = rows[y].get(x).copied().flatten().map(rgb).unwrap_or(WATER);
            let bottom = rows.get(y + 1).and_then(|r| r.get(x)).copied().flatten().map(rgb).unwrap_or(WATER);
            print!(
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                top.0, top.1, top.2, bottom.0, bottom.1, bottom.2
            );
        }
        println!("\x1b[0m");
        y += 2;
    }
}
```

Build the cast (`Googly`, `Cool`, `Upsidedown`, `Ducky`, `Food`, `Shark`) via their constructors (as the current preview does) and call `print_sprite(name, &e.sprite())` for each. Keep it `fn main()`, no TTY setup.

- [ ] **Step 2: Run preview.** Run: `cargo run --example preview` — Expected: prints six labeled pixel sprites and exits 0. Eyeball that each reads (the executor should visually confirm fish look like fish).

---

### Task 7: Delete the char path, rename to final names, verify, commit

**Files:** `src/sprite.rs`, `src/render.rs`, `src/raster.rs`, `src/lib.rs`, delete `src/font8x8.rs`

- [ ] **Step 1: Delete old types.**
  - `src/sprite.rs`: delete `struct Sprite`, `struct Style`, `enum Facing`'s nothing (keep `Facing`), `fn mirror_row`, `fn mirror_char`, and the old `Sprite`-based tests.
  - `src/render.rs`: delete the old `struct Cell { ch, style }`, the old `struct Frame`, its `impl` (`set`, `cell`, `styled`, char `draw_sprite`, char `diff`), and the old `flush_diff`. **Keep `to_ct`** — after the rename it is the color map used by the pixel `flush_diff` (via `half_rgb`), so it is still live, not orphaned. Delete the old char-based tests.
  - `src/raster.rs`: delete `rgb_bold`, `pixel_color`, `glyph`, `ascii_fallback`, the old `grid_dims`, the old `blit`, the `use crate::font8x8::...`, and all font/glyph tests (incl. `every_entity_glyph_renders_non_blank`).
  - Delete `src/font8x8.rs`; remove `pub mod font8x8;` from `src/lib.rs`.

- [ ] **Step 2: Rename temporary names to final spec names** (project-wide):
  - `PixelFrame` → `Frame`, `PixelCell` → `Cell`, `flush_pixels` → `flush_diff`, `blit_pixels` → `blit`, `grid_dims_px` → `grid_dims`.
  - Run: `git grep -l 'PixelFrame\|PixelCell\|flush_pixels\|blit_pixels\|grid_dims_px'` and replace in each (src + tests + main.rs + aquarium.rs). After renaming, `blit`/`grid_dims` occupy the names the deleted font versions had — confirm no duplicate definitions remain.

- [ ] **Step 3: Add the well-formed-cast test** (replaces the deleted glyph guard) in `src/raster.rs` tests (or `entity.rs`):

```rust
#[test]
fn every_entity_sprite_is_well_formed() {
    use crate::entity::{Entity, Food, Shark};
    use crate::fish::{Cool, Ducky, Googly, Upsidedown};
    use crate::geom::Vec2;
    let p = Vec2 { x: 0.0, y: 0.0 };
    let cast: Vec<Box<dyn Entity>> = vec![
        Box::new(Googly::new(p, 1.0)),
        Box::new(Cool::new(p, 1.0)),
        Box::new(Upsidedown::new(p, 1.0)),
        Box::new(Ducky::new(p, 1.0)),
        Box::new(Food::new(p)),
        Box::new(Shark::new(p, 1.0)),
    ];
    for e in &cast {
        let rows = e.sprite().rendered_rows();
        assert!(!rows.is_empty(), "sprite has no rows");
        let w = rows[0].len();
        assert!(rows.iter().all(|r| r.len() == w), "sprite rows are ragged");
        assert!(
            rows.iter().flatten().any(|p| p.is_some()),
            "sprite renders fully blank"
        );
    }
}
```

- [ ] **Step 4: Full verification.** Run, expecting all clean:
  - `cargo test` — Expected: PASS (full suite, incl. bins).
  - `cargo build --all-targets --features gui` — Expected: compiles.
  - `cargo clippy --all-targets --features gui` — Expected: zero warnings.
  - `cargo fmt --check` — Expected: clean (run `cargo fmt` if not).
  - `cargo run --example preview` — Expected: renders the cast, exits 0.

- [ ] **Step 5: Commit the whole rewire.**

```bash
git add -A
git commit -m "feat: render the aquarium as flat pixel art (half-block terminal, square-block window)"
```

---

### Task 8: Docs

**Files:** `README.md`, `CLAUDE.md`

- [ ] **Step 1: Update `README.md`.** Replace ASCII-art descriptions of the cast (`><(((°>`, `_(°)>`), the `Style { bold, color }` mention, and the "embedded 8×8 bitmap font" window description with the pixel-art model: a curated flat `Color` palette, `PixelSprite` grids, half-block (`▀`) truecolor terminal rendering, square-block window rendering, and the **24-bit truecolor terminal** requirement.

- [ ] **Step 2: Update `CLAUDE.md`.** In the architecture notes, rewrite the `sprite.rs`/`render.rs`/`raster.rs` bullets: char grid → pixel grid; `Style`/bold layer → per-pixel palette color; `flip_v` glyph-swapping via `mirror_char` → column-reversal mirror; the `font8x8`/`ascii_fallback`/`every_entity_glyph_renders_non_blank` path → `blit` square blocks + `every_entity_sprite_is_well_formed`. Add the truecolor requirement. Update the "Rendering & weight" and "facing & movement" notes (no bold; facing mirrors pixel columns).

- [ ] **Step 3: Commit.**

```bash
git add README.md CLAUDE.md
git commit -m "docs: describe the pixel-art render model and truecolor requirement"
```

---

## Done criteria

- `cargo test`, `cargo build --all-targets --features gui`, `cargo clippy --all-targets --features gui`, `cargo fmt --check` all clean.
- `cargo run --example preview` shows the six pixel sprites.
- Manual launch (`cargo run`, then `q`) shows a water-filled tank of pixel fish; `cargo run --features gui --bin aquarium` shows the same in a window. (User-run; not part of automated verification.)
- README + CLAUDE.md describe the pixel model.
