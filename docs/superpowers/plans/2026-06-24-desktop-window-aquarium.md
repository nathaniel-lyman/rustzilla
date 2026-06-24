# Desktop Window Aquarium Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Run the existing aquarium simulation in a normal resizable desktop window, keeping the ASCII look and keyboard controls, by adding a second thin frontend over the unchanged `lib`.

**Architecture:** A new `minifb` window driver (`src/bin/aquarium.rs`) ticks the *same* `Tank` as the terminal app and renders the *same* styled-cell `Frame` — but through a hand-rolled glyph blitter (`src/raster.rs`) over an embedded 8×8 bitmap font (`src/font8x8.rs`) into a pixel buffer, instead of crossterm cells. `lib` stays backend-agnostic: the font/blitter/sizing are pure, unit-tested logic that produce a `Vec<u32>` of pixels and know nothing about minifb.

**Tech Stack:** Rust 2021, `crossterm` (existing terminal frontend), `minifb` (new, optional, behind a `gui` feature), embedded public-domain `font8x8` data.

**Reference docs:** the spec at `docs/superpowers/specs/2026-06-24-desktop-window-aquarium-design.md`.

**Conventions (from CLAUDE.md):**
- TDD, logic-only: movement/bounds/render-buffer logic is unit-tested; terminal/window I/O and the loops are not (verified by running).
- Keep `cargo clippy --all-targets` and `cargo fmt --check` clean (zero warnings).
- **Never run the window binary in a headless/non-interactive context** — like `cargo run`, it enters a loop that won't exit. Verify it with `cargo build --features gui --bin aquarium` and (for a human) a manual launch. Logic is verified via `cargo test`.

---

## Chunk 1: Window frontend

### Task 1: Extract `Tank::draw` (shared, tested refactor)

The entity→frame draw loop is currently inlined in `main.rs`. Both frontends need it; extract it to a tested method and have `main.rs` call it. Pure refactor, no new dependencies, no behavior change.

**Files:**
- Modify: `src/tank.rs` (add `use` + `Tank::draw` + a test)
- Modify: `src/main.rs:54-59` (call `tank.draw` instead of the inline loop)

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src/tank.rs` (it already has `use super::*;`):

```rust
#[test]
fn draw_places_entities_at_rounded_positions() {
    use crate::render::Frame;
    let mut t = Tank::new(20, 10);
    t.drop_food_at(5.0); // a Food pellet '•' starts at the top: (5, 0)
    let mut frame = Frame::new(20, 10);
    t.draw(&mut frame);
    assert_eq!(frame.cell(5, 0), '•'); // pellet drawn at its rounded cell
    assert_eq!(frame.cell(0, 9), ' '); // elsewhere stays blank
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib draw_places_entities_at_rounded_positions`
Expected: FAIL — `no method named draw found for struct Tank`.

- [ ] **Step 3: Add the import and the method**

At the top of `src/tank.rs`, add `Frame` to the imports (new line after the existing `use crate::geom...`):

```rust
use crate::render::Frame;
```

Add this method inside `impl Tank` (e.g. right after `entities`):

```rust
/// Draw every entity into `frame` at its rounded cell position. Shared by
/// the terminal and window frontends so both render identically.
pub fn draw(&self, frame: &mut Frame) {
    for e in &self.entities {
        let p = e.pos();
        frame.draw_sprite(p.x.round() as i32, p.y.round() as i32, &e.sprite());
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --lib draw_places_entities_at_rounded_positions`
Expected: PASS.

- [ ] **Step 5: Switch `main.rs` to the shared method**

In `src/main.rs`, replace the inline render loop:

```rust
        // --- render ---
        let mut frame = Frame::new(cols, rows);
        for e in tank.entities() {
            let p = e.pos();
            frame.draw_sprite(p.x.round() as i32, p.y.round() as i32, &e.sprite());
        }
```

with:

```rust
        // --- render ---
        let mut frame = Frame::new(cols, rows);
        tank.draw(&mut frame);
```

- [ ] **Step 6: Verify the whole suite + lint still pass**

Run: `cargo test && cargo clippy --all-targets && cargo fmt --check`
Expected: all green, zero warnings. (`cargo build` proves `main.rs` still compiles with the new call.)

- [ ] **Step 7: Commit**

```bash
git add src/tank.rs src/main.rs
git commit -m "refactor: extract Tank::draw shared by both frontends"
```

---

### Task 2: Embed the public-domain 8×8 font

A crate-private module holding the `font8x8` "basic" table (printable ASCII), as data baked into the binary — no dependency.

**Files:**
- Create: `src/font8x8.rs`
- Modify: `src/lib.rs` (declare the module)

- [ ] **Step 1: Declare the module**

In `src/lib.rs`, add (keep the list alphabetical-ish; it can sit after `pub mod fish;`):

```rust
mod font8x8;
```

It is crate-private (no `pub`) — only `raster` reads it.

- [ ] **Step 2: Fetch the canonical data**

Fetch the public-domain table (CC0 / public domain; originally an IBM-PC ROM font, released by Marcel Sondaar / Daniel Hepper):

`https://raw.githubusercontent.com/dhepper/font8x8/master/font8x8_basic.h`

It declares `char font8x8_basic[128][8]` — 128 glyphs, 8 bytes each, one byte per row (row 0 = top). In each byte, **bit `n` (value `1 << n`) is column `n`, with column 0 = leftmost** (this is the convention the blitter in Task 5 matches).

If WebFetch is unavailable, the same table ships in many public-domain sources; any faithful transcription works (the tests below pin only blank-vs-nonblank, not exact glyph shapes).

- [ ] **Step 3: Write the module**

Create `src/font8x8.rs`. Transcribe all 128 entries from the header into this shape (the C `{0x00, 0x00, ...}` rows become Rust `[0x00, 0x00, ...]`):

```rust
//! Public-domain 8×8 bitmap font (the classic `font8x8_basic` set: printable
//! ASCII 0x00..0x80). Public domain / CC0 — derived from an IBM-PC ROM font,
//! via https://github.com/dhepper/font8x8. Each glyph is 8 rows top→bottom;
//! within a row, bit `n` (1 << n) is column `n`, column 0 leftmost.
//! Non-printable rows (control codes < 0x20) are all-zero, like a space.

pub const FONT8X8_BASIC: [[u8; 8]; 128] = [
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // 0x00
    // ... transcribe entries 0x01..=0x7F here ...
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // 0x7F
];

#[cfg(test)]
mod tests {
    use super::FONT8X8_BASIC;

    #[test]
    fn has_128_glyphs() {
        assert_eq!(FONT8X8_BASIC.len(), 128);
    }

    #[test]
    fn space_is_blank() {
        assert_eq!(FONT8X8_BASIC[0x20], [0; 8]); // ' ' draws nothing
    }

    #[test]
    fn letter_a_is_not_blank() {
        assert!(FONT8X8_BASIC[0x41].iter().any(|&r| r != 0)); // 'A'
    }
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib font8x8`
Expected: PASS (3 tests). If `space_is_blank` fails, the transcription is offset by a row — re-check that index `0x20` is the space entry.

- [ ] **Step 5: Lint + format**

Run: `cargo clippy --all-targets && cargo fmt --check`
Expected: zero warnings. (`cargo fmt` will pack the array rows; run `cargo fmt` first if `--check` complains.)

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/font8x8.rs
git commit -m "feat: embed public-domain font8x8 basic table"
```

---

### Task 3: `raster.rs` — colors + glyph accessor

Start the pure renderer module: the `Color → u32` maps, the tank background, and a `glyph()` accessor that maps the sprites' few non-ASCII art glyphs to ASCII look-alikes and blanks anything out of range. No `minifb`, no windowing — pure data.

**Files:**
- Create: `src/raster.rs`
- Modify: `src/lib.rs` (declare `pub mod raster;`)

- [ ] **Step 1: Declare the module**

In `src/lib.rs` add:

```rust
pub mod raster;
```

- [ ] **Step 2: Write failing tests**

Create `src/raster.rs` with just the tests first so they fail to compile (drives the API):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bold_uses_distinct_color() {
        // Bold must be visibly brighter so the bold shark reads as heavier.
        assert_ne!(rgb(Color::Red), rgb_bold(Color::Red));
    }

    #[test]
    fn space_and_unknown_glyphs_are_blank() {
        assert_eq!(glyph(' '), [0; 8]);
        assert_eq!(glyph('\u{2603}'), [0; 8]); // snowman: out of range → blank
    }

    #[test]
    fn art_glyphs_map_to_ascii_and_are_not_blank() {
        // The sprites use these three non-ASCII glyphs; each must render as a
        // visible ASCII look-alike rather than a blank hole.
        for c in ['#', '°', '•', '⊙'] {
            assert!(glyph(c).iter().any(|&r| r != 0), "{c:?} should not be blank");
        }
    }
}
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test --lib raster`
Expected: FAIL — `cannot find function rgb` / `glyph` / `Color` unresolved.

- [ ] **Step 4: Implement the colors + glyph accessor**

Add above the `tests` module in `src/raster.rs`:

```rust
use crate::font8x8::FONT8X8_BASIC;
use crate::sprite::{Color, Style};

/// Tank background (deep blue-black), packed 0x00RRGGBB.
pub const BG: u32 = 0x000A_1428;

/// Map a logical color to a packed 0x00RRGGBB pixel. Symmetric with
/// `render::to_ct` (which maps the same enum to crossterm).
pub fn rgb(color: Color) -> u32 {
    match color {
        Color::Red => 0x00E0_4040,
        Color::Yellow => 0x00E0_C040,
        Color::Green => 0x0040_C040,
        Color::Cyan => 0x0040_C0C0,
        Color::Blue => 0x0040_60E0,
        Color::White => 0x00D0_D0D0,
    }
}

/// Brighter shade for bold sprites (e.g. the shark).
pub fn rgb_bold(color: Color) -> u32 {
    match color {
        Color::Red => 0x00FF_6060,
        Color::Yellow => 0x00FF_F060,
        Color::Green => 0x0060_FF60,
        Color::Cyan => 0x0070_FFFF,
        Color::Blue => 0x0070_90FF,
        Color::White => 0x00FF_FFFF,
    }
}

/// Resolve a cell's pixel color from its style. Uncolored cells get a soft
/// off-white so plain ASCII fish stay visible against `BG`.
pub fn pixel_color(style: Style) -> u32 {
    match (style.color, style.bold) {
        (Some(c), true) => rgb_bold(c),
        (Some(c), false) => rgb(c),
        (None, true) => 0x00FF_FFFF,
        (None, false) => 0x00C0_C8D0,
    }
}

/// The 8×8 bitmap rows for `c`. The sprites use a few non-ASCII art glyphs;
/// map those to ASCII look-alikes. Anything outside printable ASCII is blank.
pub fn glyph(c: char) -> [u8; 8] {
    let c = ascii_fallback(c);
    let code = c as u32;
    if (0x20..0x80).contains(&code) {
        FONT8X8_BASIC[code as usize]
    } else {
        [0; 8]
    }
}

fn ascii_fallback(c: char) -> char {
    match c {
        '°' => 'o', // fish eye
        '⊙' => 'O', // googly eye
        '•' => '*', // food pellet
        other => other,
    }
}
```

- [ ] **Step 5: Run to verify it passes**

Run: `cargo test --lib raster`
Expected: PASS (3 tests).

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/raster.rs
git commit -m "feat: raster color maps + ASCII-fallback glyph accessor"
```

---

### Task 4: `raster.rs` — `grid_dims`

Pure pixel→grid sizing used by the driver to translate window size into tank `(cols, rows)`.

**Files:**
- Modify: `src/raster.rs`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src/raster.rs`:

```rust
#[test]
fn grid_dims_floors_and_clamps() {
    assert_eq!(grid_dims(240, 120, 24, 24), (10, 5)); // exact fit
    assert_eq!(grid_dims(250, 130, 24, 24), (10, 5)); // remainder floored
    assert_eq!(grid_dims(10, 10, 24, 24), (1, 1));    // sub-cell window → 1×1
    assert_eq!(grid_dims(0, 0, 24, 24), (1, 1));      // degenerate 0-size → 1×1
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --lib grid_dims_floors_and_clamps`
Expected: FAIL — `cannot find function grid_dims`.

- [ ] **Step 3: Implement**

Add to `src/raster.rs` (above the tests):

```rust
/// Window pixels → tank grid `(cols, rows)`. Floors to whole cells; clamps each
/// dimension to at least 1 so a transient 0-size window never yields a 0 grid
/// (mirrors `main.rs`'s `.max(1)` guard on terminal size).
pub fn grid_dims(px_w: usize, px_h: usize, cell_w: usize, cell_h: usize) -> (u16, u16) {
    let cols = (px_w / cell_w.max(1)).max(1) as u16;
    let rows = (px_h / cell_h.max(1)).max(1) as u16;
    (cols, rows)
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --lib grid_dims_floors_and_clamps`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/raster.rs
git commit -m "feat: raster::grid_dims pixel-to-grid sizing"
```

---

### Task 5: `raster.rs` — `blit`

The heart of the hand-rolled renderer: turn a `Frame` into a `px_w × px_h` pixel buffer, scaling each 8×8 glyph into `scale × scale` blocks in its resolved color, clipping at the edges.

**Files:**
- Modify: `src/raster.rs`

- [ ] **Step 1: Write failing tests**

Add to the `tests` module in `src/raster.rs` (extend its `use` line to bring in `Frame`, `Sprite`):

```rust
// at the top of `mod tests`, alongside `use super::*;`
use crate::render::Frame;
use crate::sprite::Sprite;

#[test]
fn blit_buffer_size_matches() {
    let f = Frame::new(3, 2);
    assert_eq!(blit(&f, 2, 48, 32).len(), 48 * 32);
}

#[test]
fn blit_blank_frame_is_all_background() {
    let f = Frame::new(3, 2); // all spaces
    assert!(blit(&f, 3, 72, 48).iter().all(|&p| p == BG));
}

#[test]
fn blit_draws_glyph_in_its_own_cell_and_color() {
    // One green glyph at cell (1,0); scale 1 → 8×8 cells.
    let mut f = Frame::new(3, 1);
    f.draw_sprite(1, 0, &Sprite::new(vec!["#".into()]).colored(Color::Green));
    let buf = blit(&f, 1, 24, 8);
    // Some pixel inside cell (1,0) (x in 8..16) is the glyph color...
    let lit = (0..8)
        .flat_map(|y| (8..16).map(move |x| (x, y)))
        .filter(|&(x, y)| buf[y * 24 + x] == rgb(Color::Green))
        .count();
    assert!(lit > 0, "glyph should light pixels in its own cell");
    // ...and the neighbouring cell (0,0) stays all background.
    for y in 0..8 {
        for x in 0..8 {
            assert_eq!(buf[y * 24 + x], BG);
        }
    }
}

#[test]
fn blit_clips_glyph_past_the_edge() {
    // A glyph whose cell runs past the buffer must clip, not panic.
    let mut f = Frame::new(2, 2);
    f.draw_sprite(1, 1, &Sprite::new(vec!["#".into()]));
    let buf = blit(&f, 2, 24, 24); // 16px cells; cell (1,1) spills past 24
    assert_eq!(buf.len(), 24 * 24); // reached here = no panic
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --lib raster`
Expected: FAIL — `cannot find function blit`.

- [ ] **Step 3: Implement**

Add to `src/raster.rs` (above the tests):

```rust
use crate::render::Frame;

/// Render `frame` into a `px_w × px_h` buffer of 0x00RRGGBB pixels at integer
/// `scale` (each cell is `8 * scale` square). Spaces are transparent (`BG`
/// shows through). Glyph pixels past the buffer edge are clipped — never a
/// panic — paralleling `Frame::draw_sprite`'s clipping.
pub fn blit(frame: &Frame, scale: u32, px_w: usize, px_h: usize) -> Vec<u32> {
    let mut buf = vec![BG; px_w * px_h];
    let s = scale.max(1) as usize;
    let cell = 8 * s;
    for cy in 0..frame.height {
        for cx in 0..frame.width {
            let styled = frame.styled(cx, cy);
            if styled.ch == ' ' {
                continue; // transparent: let BG show through
            }
            let rows = glyph(styled.ch);
            let color = pixel_color(styled.style);
            let ox = cx as usize * cell;
            let oy = cy as usize * cell;
            for (by, row) in rows.iter().enumerate() {
                for bx in 0..8 {
                    if row & (1 << bx) == 0 {
                        continue; // dark bit
                    }
                    // Scale this lit bit into an s×s block, clipped to the buffer.
                    for sy in 0..s {
                        let py = oy + by * s + sy;
                        if py >= px_h {
                            break;
                        }
                        for sx in 0..s {
                            let px = ox + bx * s + sx;
                            if px >= px_w {
                                break;
                            }
                            buf[py * px_w + px] = color;
                        }
                    }
                }
            }
        }
    }
    buf
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --lib raster`
Expected: PASS (all raster tests, including the four new blit tests).

- [ ] **Step 5: Lint + format**

Run: `cargo clippy --all-targets && cargo fmt --check`
Expected: zero warnings.

- [ ] **Step 6: Commit**

```bash
git add src/raster.rs
git commit -m "feat: raster::blit renders a Frame into a pixel buffer"
```

---

### Task 6: Cargo `gui` feature + the minifb window driver

Wire the optional dependency and feature, then write the thin driver. The driver is I/O glue (like `flush_diff`/`TerminalGuard`/`main.rs`) and is **not** unit-tested — it is verified by compiling and by a human launch.

**Files:**
- Modify: `Cargo.toml`
- Create: `src/bin/aquarium.rs`

- [ ] **Step 1: Add the feature, optional dep, and bin target**

Edit `Cargo.toml` to:

```toml
[package]
name = "rustzilla"
version = "0.1.0"
edition = "2021"

[features]
gui = ["dep:minifb"]

[dependencies]
crossterm = "0.27"
minifb = { version = "0.27", optional = true }

[[bin]]
name = "aquarium"
path = "src/bin/aquarium.rs"
required-features = ["gui"]
```

The default `rustzilla` binary (`src/main.rs`) is auto-discovered and unchanged. Because the `aquarium` bin sets `required-features = ["gui"]`, a plain `cargo build` / `cargo test` / `cargo clippy --all-targets` skips it and never compiles `minifb`.

- [ ] **Step 2: Write the driver**

Create `src/bin/aquarium.rs`:

```rust
//! Desktop-window frontend: the same Tank as the terminal app, rendered into a
//! resizable window via a hand-rolled glyph blitter over a minifb pixel buffer.
//! This is I/O glue (no unit tests) — verified by compiling and by launching.
//! Launch: `cargo run --features gui --bin aquarium`.
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use rustzilla::input::{action_for_key, Action};
use rustzilla::raster::{self, blit};
use rustzilla::render::Frame;
use rustzilla::tank::Tank;
use std::time::{Duration, Instant};

const SCALE: u32 = 3; // 8×8 font → 24×24 px cells
const CELL: usize = 8 * SCALE as usize;
const FRAME_BUDGET: Duration = Duration::from_millis(60); // ~16 FPS
const MAX_DT: f32 = 0.1; // clamp so a paused/occluded window doesn't teleport fish

/// The four keys the tank reacts to, mapped to the chars `action_for_key` knows.
fn key_char(key: Key) -> Option<char> {
    match key {
        Key::F => Some('f'),
        Key::A => Some('a'),
        Key::S => Some('s'),
        Key::Q => Some('q'),
        _ => None,
    }
}

fn main() {
    let (mut px_w, mut px_h) = (960usize, 600usize);
    let mut window = Window::new(
        "rustzilla",
        px_w,
        px_h,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )
    .expect("failed to open window");

    let (mut cols, mut rows) = raster::grid_dims(px_w, px_h, CELL, CELL);
    let mut tank = Tank::new(cols, rows);
    for _ in 0..6 {
        tank.add_fish_at(); // seed a few fish, like the terminal app
    }

    let mut last = Instant::now();
    while window.is_open() {
        let tick_start = Instant::now();

        // --- input: focused keys → char → shared action_for_key ---
        for key in window.get_keys_pressed(KeyRepeat::No) {
            if let Some(c) = key_char(key) {
                match action_for_key(c) {
                    Some(Action::Quit) => return,
                    Some(Action::Feed) => tank.feed(),
                    Some(Action::AddFish) => tank.add_fish_at(),
                    Some(Action::Shark) => tank.summon_shark(),
                    None => {}
                }
            }
        }

        // --- resize: window pixels → grid ---
        let (w, h) = window.get_size();
        if (w, h) != (px_w, px_h) {
            px_w = w;
            px_h = h;
            let (c, r) = raster::grid_dims(px_w, px_h, CELL, CELL);
            if (c, r) != (cols, rows) {
                cols = c;
                rows = r;
                tank.resize(cols, rows);
            }
        }

        // --- update ---
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(MAX_DT);
        last = now;
        tank.update(dt);

        // --- render ---
        let mut frame = Frame::new(cols, rows);
        tank.draw(&mut frame);
        let buf = blit(&frame, SCALE, px_w, px_h);
        window
            .update_with_buffer(&buf, px_w, px_h)
            .expect("failed to present frame");

        // --- frame budget (~16 FPS, mirrors main.rs) ---
        let elapsed = tick_start.elapsed();
        if elapsed < FRAME_BUDGET {
            std::thread::sleep(FRAME_BUDGET - elapsed);
        }
    }
}
```

- [ ] **Step 3: Verify it compiles (do NOT run it)**

Run: `cargo build --features gui --bin aquarium`
Expected: compiles (this is the first build of `minifb`, so it downloads/compiles its deps — may take a minute). **Do not `cargo run` it** — it opens a window loop that won't exit in a headless context.

- [ ] **Step 4: Lint both modes + format**

Run: `cargo clippy --all-targets && cargo clippy --all-targets --features gui && cargo fmt --check`
Expected: zero warnings in both clippy passes (the second one lints the gui bin).

- [ ] **Step 5: Verify the default build is still lean**

Run: `cargo build` (no features)
Expected: compiles without pulling `minifb` (the `aquarium` target is skipped — its `required-features` aren't met).

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/bin/aquarium.rs
git commit -m "feat: minifb desktop-window frontend behind the gui feature"
```

---

### Task 7: Docs — CLAUDE.md + README

Document the new launch command and, critically, the headless-hang warning for the window binary.

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`

- [ ] **Step 1: Update CLAUDE.md commands**

In `CLAUDE.md`, under the `## Commands` fenced block, add after the `cargo run --example preview` line:

```bash
cargo run --features gui --bin aquarium   # launch the desktop window aquarium (interactive window)
```

- [ ] **Step 2: Extend the headless warning in CLAUDE.md**

Find the bolded note that begins **"Do not run `cargo run` to verify changes in a headless/non-interactive context…"** and extend it so it also covers the window binary, e.g. append a sentence:

> The same applies to `cargo run --features gui --bin aquarium` — it opens a desktop window event loop that only exits on `q`/window-close and will hang a headless run. Verify the window frontend with `cargo build --features gui --bin aquarium` (compiles, doesn't launch) and `cargo test` for its logic; eyeball it via a manual launch.

- [ ] **Step 3: Add an architecture note in CLAUDE.md**

In the `## Architecture` list, after the `render.rs` bullet, add a `raster.rs` bullet and note the second frontend, e.g.:

```markdown
- `raster.rs` — the **window** render path: an embedded public-domain 8×8 bitmap font (`font8x8.rs`), `Color`→`u32` maps, and `blit` (a `Frame` → `Vec<u32>` pixel buffer). Pure and unit-tested; knows nothing about minifb. The terminal path (`render.rs`→crossterm) and window path (`raster.rs`→minifb) are symmetric frontends over the same `Tank`; `lib` stays backend-agnostic.
```

And note the two binaries near the top of the Architecture section (where it says "`main.rs` is a thin binary"):

> There are two thin frontends: `main.rs` (terminal, default `cargo run`) and `src/bin/aquarium.rs` (desktop window, `--features gui`). Both tick the same `Tank` via `Tank::draw`.

- [ ] **Step 4: Update README**

In `README.md`, add the window launch to the usage/commands section and a one-line description of the desktop-window mode (match the README's existing tone/format). Keep it short: mention `cargo run --features gui --bin aquarium`, that controls are the same (`f`/`a`/`s`/`q`), and that the window is resizable.

- [ ] **Step 5: Verify docs don't break anything**

Run: `cargo fmt --check && cargo test`
Expected: green (docs-only change; this just confirms nothing was disturbed).

- [ ] **Step 6: Commit**

```bash
git add CLAUDE.md README.md
git commit -m "docs: document the desktop-window aquarium frontend"
```

---

### Task 8: Final verification

- [ ] **Step 1: Full logic suite**

Run: `cargo test`
Expected: all tests pass, including the new `raster`, `font8x8`, and `Tank::draw` tests, and every pre-existing test.

- [ ] **Step 2: Lint, both feature modes**

Run: `cargo clippy --all-targets && cargo clippy --all-targets --features gui`
Expected: zero warnings in both.

- [ ] **Step 3: Format**

Run: `cargo fmt --check`
Expected: clean.

- [ ] **Step 4: Both binaries build**

Run: `cargo build && cargo build --features gui --bin aquarium`
Expected: both compile. (Still do not *run* the window binary headlessly.)

- [ ] **Step 5: Human smoke test (manual, optional)**

A human runs `cargo run --features gui --bin aquarium`, confirms a window opens with swimming fish, `f`/`a`/`s` react when focused, resizing reflows the tank, and `q`/close exits cleanly.

---

## Out of scope (per spec)

Always-on-top / wallpaper / click-through window behavior; mouse interaction; HUD/score; tank polish (gradients/bubbles); `.app` packaging; unfocused-FPS throttling. These are explicitly deferred.
