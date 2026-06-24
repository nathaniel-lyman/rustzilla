# Desktop Window Aquarium — Design

**Date:** 2026-06-24
**Status:** Approved (spec review passed)

## Summary

`rustzilla` currently runs only as a terminal TUI: `main.rs` enters an
alternate screen, ticks a `Tank`, and flushes a styled-cell `Frame` to stdout
via crossterm. This feature adds a **second frontend** that runs the *same*
simulation in a normal desktop window — a passive, ambient fish tank you can
park in a corner of your screen.

The terminal app is unchanged. The window app is a thin new driver over the
existing `lib.rs`: it ticks the same `Tank`, reuses the same `Action` input
model, and renders the same ASCII sprites — only the output target changes from
"cells written to a terminal" to "glyphs blitted into a pixel buffer."

Crucially, the rendering is **hand-rolled**: rather than pull in a GPU/font
stack, we take on exactly one small windowing crate (`minifb`) for "a window +
a pixel buffer + key input," embed a public-domain 8×8 bitmap font as data, and
write our own glyph blitter. The blitter is pure logic and unit-tested, which
fits the project's "TDD, logic-only" convention far better than an opaque GUI
toolkit would.

**Goals**
- Run the existing aquarium in a normal, resizable desktop window.
- Keep the ASCII aesthetic — colored monospace glyphs, same sprites.
- Keep the interactive controls (`f` feed, `a` add fish, `s` shark, `q` quit)
  when the window is focused; otherwise it's passively ambient.
- Add as little dependency weight as possible and keep the new rendering code
  pure and unit-testable.
- Preserve the existing architecture: `lib.rs` stays backend-agnostic; the
  window frontend is symmetric with the terminal frontend.

**Non-goals**
- No always-on-top, click-through, or desktop-wallpaper window behavior — a
  plain top-level window only. (Possible later via window flags; out of scope.)
- No new simulation behavior, entities, or keybindings. Same `Tank`, same
  `Action`s.
- No mouse interaction (no click-to-feed). Keyboard only, as today.
- No GPU, no TTF/font-shaping, no UI widgets/buttons/HUD.
- No change to the terminal frontend's behavior or dependencies.

## User-facing behavior

- Launch with `cargo run --features gui --bin aquarium`. A window titled
  `rustzilla` opens (default ~960×600) showing six seeded fish swimming, just
  like the terminal app's startup.
- The window is **resizable**; the tank reflows to fill it (more screen → more
  cells → a bigger tank), mirroring how the terminal app reacts to terminal
  resizes.
- While the window is **focused**: `f` drops food, `a` adds a fish, `s` summons
  the shark, `q` quits. Clicking away (unfocusing) leaves it running passively.
- Closing the window (close button or `q`) exits cleanly.
- The terminal app is launched exactly as before (`cargo run`) and is
  unaffected.

## Architecture

The shape mirrors today's terminal frontend exactly. Today:

```
main.rs  ──ticks──▶ Tank (lib)        input.rs  ── crossterm keys ─▶ Action
   │                                   render.rs ── Color ─▶ crossterm color
   └─ Frame ─▶ flush_diff ─▶ terminal
```

After this change, a parallel path is added (nothing existing is removed):

```
bin/aquarium.rs ──ticks──▶ Tank (lib)   minifb keys ─▶ char ─▶ action_for_key ─▶ Action
   │                                     raster.rs ── Color ─▶ u32 pixel
   └─ Frame ─▶ raster::blit ─▶ Vec<u32> ─▶ window.update_with_buffer
```

`render.rs` maps `Color → crossterm`; the new `raster.rs` maps `Color → u32`.
`input.rs::poll_input` reads crossterm; the new driver reads minifb keys. Both
funnel through the *same* pure `action_for_key` and tick the *same* `Tank`.

### Module/file inventory

| File | Status | Role |
|------|--------|------|
| `Cargo.toml` | edit | add optional `minifb` dep + `gui` feature + `[[bin]] aquarium` |
| `src/lib.rs` | edit | `pub mod raster;` |
| `src/raster.rs` | **new (lib)** | embedded font, `Color → u32`, `blit`, `grid_dims` — pure, tested |
| `src/tank.rs` | edit | extract `Tank::draw(&self, &mut Frame)`; tested |
| `src/main.rs` | edit | call `tank.draw(&mut frame)` instead of the inline draw loop |
| `src/bin/aquarium.rs` | **new (gui bin)** | minifb window + loop; thin, untested |

No changes to `entity.rs`, `fish.rs`, `geom.rs`, `sprite.rs`, or `input.rs`'s
logic. `sprite.rs` stays free of any backend (it already is).

### Cargo packaging

```toml
[features]
gui = ["dep:minifb"]

[dependencies]
crossterm = "0.27"
minifb = { version = "0.27", optional = true }

[[bin]]
name = "aquarium"
required-features = ["gui"]
```

- `minifb` is **optional** and pulled in **only** by the `gui` feature.
- The `aquarium` binary declares `required-features = ["gui"]`, so a plain
  `cargo build` / `cargo test` / `cargo clippy --all-targets` never compiles it
  and never pulls `minifb`. The default build stays exactly as lean as today.
- The terminal binary (`rustzilla`, from `main.rs`) and the `preview` example
  are unaffected.
- CI/lint note: `cargo clippy --all-targets` (no feature) lints everything
  except the gui bin; to keep the gui bin warning-free we also run
  `cargo clippy --all-targets --features gui`. Both must be clean.

> Version note: the exact `minifb` version is pinned at implementation time to
> the current release; `0.27` is the expected line. The API surface we use
> (`Window::new`, `WindowOptions { resize, .. }`, `limit_update_rate`,
> `update_with_buffer`, `is_open`, `get_size`, `get_keys_pressed`) is stable
> across recent versions.

## `src/raster.rs` — the hand-rolled renderer (pure, testable)

This module is the heart of the "hand roll it" decision. It has **no windowing
dependency** — it only turns a `Frame` into a `Vec<u32>` of `0x00RR_GGBB`
pixels. minifb never appears here; the binary owns that.

### Embedded font

- A public-domain **8×8 bitmap font** (the classic `font8x8` "basic" set)
  covering printable ASCII `0x20..=0x7E`, stored as a `const` table: 8 bytes
  per glyph, one byte per row, bit `n` = pixel `n` lit.
- `fn glyph(c: char) -> [u8; 8]` returns the glyph rows for a char, falling
  back to all-zero (blank) for anything outside the covered range. Pure.

### Color mapping

- `fn rgb(color: Color) -> u32` maps each `sprite::Color` variant to a packed
  `0x00RR_GGBB`. Symmetric with `render.rs::to_ct`. Bold is rendered by
  selecting a **brighter** shade (a separate `rgb_bold`), so bold sprites (the
  shark) still read as heavier without faux-bolding the glyph geometry.
- `const BG: u32` is the tank background (a deep blue-black, e.g.
  `0x00_0A_14_28`).

### Blitting

```rust
/// Render a Frame into a width×height pixel buffer at integer `scale`.
/// Each cell is (8*scale) × (8*scale) px. Cells past the buffer edge are
/// clipped; leftover pixels on the right/bottom stay BG.
pub fn blit(frame: &Frame, scale: u32, px_w: usize, px_h: usize) -> Vec<u32>
```

- Allocates `px_w * px_h` pixels filled with `BG`.
- For each cell `(x, y)`, reads `frame.styled(x, y)`; spaces are skipped
  (background shows through). For a non-space glyph, looks up its 8×8 bits and
  writes the lit pixels — scaled into `scale × scale` blocks — in the cell's
  resolved color (`rgb` / `rgb_bold` from its `Style`).
- Bounds-checked so a cell partially off the buffer is clipped, never panics
  (parallels `Frame::draw_sprite`'s clipping).

### Grid sizing

```rust
/// Window pixels → tank grid. Floors; clamps each dim to ≥ 1.
pub fn grid_dims(px_w: usize, px_h: usize, cell_w: usize, cell_h: usize) -> (u16, u16)
```

Used by the driver to translate the live window size into `(cols, rows)` for
`Tank::resize`. `cell_w == cell_h == 8 * scale`.

### Why this lives in `lib`, not the binary

Putting the font + blitter + sizing in a library module (not in
`bin/aquarium.rs`) keeps them **unit-testable** in the normal `cargo test`
suite and reusable. They contain no windowing concept — a `Vec<u32>` is plain
data — so `lib.rs` stays backend-agnostic, exactly as it is free of crossterm
today (crossterm only appears in `render.rs`'s flush half and `input.rs`).

## `Tank::draw` — small shared refactor

Today `main.rs` inlines the entity→frame loop:

```rust
let mut frame = Frame::new(cols, rows);
for e in tank.entities() {
    let p = e.pos();
    frame.draw_sprite(p.x.round() as i32, p.y.round() as i32, &e.sprite());
}
```

Both frontends need exactly this. Extract it to:

```rust
impl Tank {
    /// Draw every entity into `frame` at its rounded cell position.
    pub fn draw(&self, frame: &mut Frame) { /* the loop above */ }
}
```

`main.rs` and `bin/aquarium.rs` both call `tank.draw(&mut frame)`. This removes
duplication and gives the draw step a unit test it lacks today. Behavior is
identical; it's a pure extraction.

## `src/bin/aquarium.rs` — the minifb driver (thin, untested)

Keeps the exact shape of `main.rs`'s loop, swapping terminal I/O for minifb.
Like `flush_diff`/`TerminalGuard`/`main.rs`, this is I/O glue and is **not**
unit-tested — it's verified by running the app.

```rust
const SCALE: u32 = 3;            // 8×8 font → 24×24 px cells
const CELL: usize = 8 * SCALE as usize;
const MAX_DT: f32 = 0.1;         // clamp so a restored/occluded window doesn't teleport fish

fn main() {
    let mut opts = WindowOptions::default();
    opts.resize = true;
    let (mut px_w, mut px_h) = (960usize, 600usize);
    let mut window = Window::new("rustzilla", px_w, px_h, opts).expect("window");
    window.limit_update_rate(Some(Duration::from_millis(60)));  // ~16 FPS

    let (mut cols, mut rows) = raster::grid_dims(px_w, px_h, CELL, CELL);
    let mut tank = Tank::new(cols, rows);
    for _ in 0..6 { tank.add_fish_at(); }

    let mut last = Instant::now();
    while window.is_open() {
        // input: focused keys → char → action_for_key
        for key in window.get_keys_pressed(KeyRepeat::No) {
            if let Some(c) = key_char(key) {
                match action_for_key(c) {
                    Some(Action::Quit)    => return,
                    Some(Action::Feed)    => tank.feed(),
                    Some(Action::AddFish) => tank.add_fish_at(),
                    Some(Action::Shark)   => tank.summon_shark(),
                    None => {}
                }
            }
        }

        // resize: window pixels → grid
        let (w, h) = window.get_size();
        if (w, h) != (px_w, px_h) {
            px_w = w; px_h = h;
            let (c, r) = raster::grid_dims(px_w, px_h, CELL, CELL);
            if (c, r) != (cols, rows) { cols = c; rows = r; tank.resize(cols, rows); }
        }

        // update
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(MAX_DT);
        last = now;
        tank.update(dt);

        // render
        let mut frame = Frame::new(cols, rows);
        tank.draw(&mut frame);
        let buf = raster::blit(&frame, SCALE, px_w, px_h);
        window.update_with_buffer(&buf, px_w, px_h).expect("present");
    }
}
```

`key_char(minifb::Key) -> Option<char>` is a small local mapping of the four
keys we care about (`F`/`A`/`S`/`Q`) to their chars; everything else returns
`None` and is ignored. This keeps the single source of truth for "what does a
key do" in the shared `action_for_key`.

## Data flow (per frame)

```
bin/aquarium loop
  ├─ get_keys_pressed → key_char → action_for_key → Tank::{feed,add_fish_at,summon_shark} / quit
  ├─ get_size → grid_dims → (if changed) Tank::resize
  ├─ dt = clamp(now - last, MAX_DT) → Tank::update(dt)      (same update pass as terminal)
  └─ Frame::new(cols,rows) → Tank::draw(&mut frame)
        → raster::blit(frame, SCALE, px_w, px_h) → Vec<u32>
        → Window::update_with_buffer
```

The simulation half (`feed`/`add_fish_at`/`summon_shark`/`update`/`draw`) is
byte-for-byte the same code the terminal app runs. Only the input source and
the output sink differ.

## Testing (logic-only, no window)

New unit tests live in `raster.rs` and `tank.rs`, matching existing style:

- **`raster.rs`**
  - `grid_dims_floors_and_clamps` — exact floor for an even fit; a sub-cell
    window still yields `(1, 1)`; a zero dimension yields `1`, never `0`.
  - `blit_buffer_size_matches` — output length is exactly `px_w * px_h`.
  - `blit_blank_frame_is_all_background` — an all-space frame produces a buffer
    of only `BG`.
  - `blit_draws_known_glyph` — a frame with a single known glyph (e.g. a full
    block or a char with a known top row) lights the expected fg pixels at the
    expected scaled offsets and leaves the rest `BG`.
  - `blit_clips_glyph_past_edge` — a glyph whose cell extends past `px_w`/`px_h`
    does not panic and writes only in-bounds pixels.
  - `bold_uses_distinct_color` — `rgb_bold(c) != rgb(c)` for at least the
    shark's color (bold is visibly distinct).
  - `glyph_falls_back_to_blank` — a char outside the covered range returns an
    all-zero glyph.
- **`tank.rs`**
  - `draw_places_entities_at_rounded_positions` — after `Tank::draw`, the cell
    at an entity's rounded position holds (the head of) its sprite; an empty
    region stays blank. (Asserts the extraction preserves `main.rs`'s behavior.)

`action_for_key` is already covered by `input.rs` tests and is reused as-is.
The minifb window, the loop, `key_char`, and `update_with_buffer` are I/O glue
and are **not** unit-tested — verified by running `cargo run --features gui
--bin aquarium`, consistent with how `flush_diff`/`TerminalGuard`/`main.rs` are
treated today.

All existing tests must stay green; this feature adds code paths and refactors
one inline loop into a tested method, changing no existing behavior.

## Risks & mitigations

- **A windowing dep still enters the tree.** Accepted and bounded: `minifb` is
  small, optional, and gated behind `gui`. Default builds/tests/lint are
  unchanged. This was a deliberate, user-approved trade (one small crate vs. a
  GPU/font stack).
- **8×8 glyphs are tiny.** Mitigated by integer `SCALE` (default 3 → 24 px
  cells). `SCALE` is a single named constant, trivially tunable.
- **Huge `dt` after minimize/occlude teleports fish.** Mitigated by clamping
  `dt` to `MAX_DT` each frame (the terminal app never needed this because it
  runs continuously; a window can be paused by the OS).
- **Degenerate 0-size window during resize.** Mitigated by `grid_dims`'
  `≥ 1` clamp, paralleling `main.rs`'s `.max(1)` guard on terminal size.
- **Per-frame full repaint (no diff).** Accepted: the buffer is small and
  redrawn at ~16 FPS; the diff machinery is a terminal bandwidth optimization
  with no analog for a local pixel buffer, so `blit` simply redraws each frame.
- **`minifb` font/license.** We embed `font8x8` (public domain / CC0); its
  origin and license are noted in a comment in `raster.rs`.

## Out of scope (explicitly)

- Always-on-top / click-through / wallpaper-layer window behavior.
- Mouse interaction (click-to-feed), buttons, menus, or any HUD/score.
- A prettier tank (gradients, bubbles, borders) — the user chose the plain
  "keep the ASCII look" treatment; polish can come later.
- Per-platform packaging (`.app` bundles, icons) — `cargo run`/`cargo build`
  is the delivery vehicle for now.
- Reducing frame rate when unfocused/occluded for battery — a nice-to-have,
  not required for a first version.
