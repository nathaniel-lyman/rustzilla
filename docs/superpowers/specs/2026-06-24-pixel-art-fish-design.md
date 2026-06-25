# Pixel-Art Fish — Design

**Date:** 2026-06-24
**Status:** Approved (pending spec review)

## Summary

The aquarium currently draws every actor as a one-line ASCII sprite (`><(((°>`)
in one of six named terminal colors. This feature replaces that with **flat
pixel art**: the whole tank — fish, shark, food — becomes a grid of colored
square pixels, rendered in the terminal with the half-block character `▀` and
24-bit truecolor, and in the desktop window with square pixel blocks.

The four personalities (Googly, Cool, Upside-down, Ducky), the shark, and the
food pellet are all re-authored as pixel sprites. The **simulation layer does
not move**: positions, drift/seek/flee, collisions, the fish cap, spawning,
and the shark hunt all stay in terminal-**cell** units exactly as today. Only
the art representation and the two render backends change.

**Goals**
- Render true pixel art with **square** pixels (half-block `▀`: one cell = two
  vertically-stacked pixels), so sprites look like the reference art rather
  than vertically stretched.
- Keep the terminal and window frontends **symmetric** over the same `Tank`,
  preserving the project's "two thin frontends, backend-agnostic `lib`" shape.
- Preserve every existing behavior and personality — this is a re-skin, not a
  gameplay change.
- Stay logic-testable without a TTY/window: the pixel `Frame`, the half-block
  diff, the `PixelSprite` transforms, and the `raster` blitter are all pure.

**Non-goals**
- No shading/gradients/anti-aliasing — **flat** solid colors only (chosen knob).
- No new entities, behaviors, keybindings, HUD, score, or sound.
- No runtime image/PNG loading — art is authored inline as palette grids; no
  new crates.
- No animation frames per fish beyond the existing facing-flip and Upside-down
  vertical flip (no fin-wiggle cycles).

## Visual model

A terminal character cell is ~twice as tall as it is wide. The half-block `▀`
fills the **top** half of a cell with the foreground color and leaves the
**bottom** half showing the background color. Setting `fg = top pixel` and
`bg = bottom pixel` therefore packs **two square pixels into one cell**:

```
cell renders ▀ with fg=cyan, bg=water  →   ██   (top pixel = cyan)
                                           ▒▒   (bottom pixel = water)
```

This is the single trick that makes the art square. Transparent pixels render
as **water** (`#0A1428`), so the tank is a filled water field rather than the
terminal's default background.

### Coordinate mapping (the key invariant)

The world stays in **cells**; sprites are authored in **pixels**.

- **Horizontal:** 1 cell = 1 pixel wide. A sprite `W` pixels wide spans `W`
  cells.
- **Vertical:** 1 cell = 2 pixels tall. A sprite `H` pixels tall spans
  `ceil(H/2)` cells.
- A sprite drawn at cell `(cx, cy)` places its top-left pixel at pixel
  `(cx, 2·cy)`. Because `2·cy` is always even, sprite row `r` lands in cell
  `(cx+col, cy + r/2)` at half `r % 2` (even rows = top, odd rows = bottom).
  Sprites thus snap to even pixel rows — 2px vertical granularity — which is
  what lets all geometry stay in whole cells.

So a 13×8px fish occupies **13×4 cells** (today's fish were 7×1). Fish are
physically larger; see *Crowding* under Risks.

### Palette

A curated, fixed, flat palette (chosen over arbitrary RGB to keep art coherent
and both backends a simple lookup, matching today's enum-mapped-per-backend
pattern). Transparency is `None`, not a palette entry.

| Variant | RGB | Used by |
|---------|-----|---------|
| `Cyan`   | `#49D0E0` | Googly body |
| `Blue`   | `#5B8CFF` | Cool body |
| `Green`  | `#4FCF6F` | Upside-down body |
| `Yellow` | `#F2C641` | Ducky body |
| `Orange` | `#E8902F` | Ducky beak, food pellet |
| `White`  | `#F2F2F2` | eyes |
| `Black`  | `#141414` | pupils, shades |
| `Grey`   | `#8A93A0` | shark body |
| `Belly`  | `#C9D0D8` | shark underside |
| `Red`    | `#D84A4A` | shark mouth |

`Water` (`#0A1428`) is the background constant, not a sprite color. `bold` is
removed from the model entirely — flat art needs no weight layer.

## Architecture changes

The simulation modules (`geom.rs`, `input.rs`, the movement helpers in
`fish.rs`, the collision/cap/spawn logic in `tank.rs`) are **logically
unchanged**. The turnover is in the art + render layer.

### `sprite.rs` (rewrite)

- **`enum Color`** becomes the 10-variant palette above (was 6 ANSI names). It
  keeps `#[derive(Clone, Copy, Debug, PartialEq, Eq)]` (as today) so `Cell`
  stays `Copy`/`Eq` and `diff` comparisons stay cheap.
- **`Style` and `bold` are removed.**
- **`struct PixelSprite { pub pixels: Vec<Vec<Option<Color>>>, pub facing:
  Facing, pub flip_v: bool }`** replaces `Sprite`. `pixels` is row-major; each
  entry is a pixel color or `None` (transparent). The fields stay **`pub`** and
  the existing direct-assignment authoring style is preserved — entities build a
  sprite and set `s.facing = …` / `s.flip_v = …` exactly as they do today (e.g.
  `Upsidedown` toggles `s.flip_v`).
- **Authoring helper** `PixelSprite::from_art(rows: &[&str], map: &[(char,
  Color)]) -> PixelSprite`: maps each character via `map`; `'.'`, `' '`, and
  any unmapped char become `None`. Rows may be ragged; shorter rows are
  right-padded with `None` to the max width. This keeps art as readable as
  today's string rows, e.g.
  `from_art(&[".bb.bbbbwwwbb", ...], &[('b', Cyan), ('w', White), ('k', Black)])`.
  Note the trade-off: a mistyped palette char maps to transparent rather than
  erroring — accepted, because `every_entity_sprite_is_well_formed` (below)
  catches a sprite that ends up blank, so `from_art` stays total (no panic path).
- **`rendered_rows() -> Vec<Vec<Option<Color>>>`** applies `facing` then
  `flip_v`:
  - `Facing::Left` **reverses each row** (column mirror). There is no
    glyph-swapping anymore — `mirror_char`/`mirror_row` are deleted. (Reversing
    pixels is the correct mirror for blocks.)
  - `flip_v` reverses the row order (Upside-down still relies on this).
- **Dimensions:** `width()` and `height()` return pixel extents. Add
  `cell_w() -> usize` = `width()` and `cell_h() -> usize` = `(height()+1)/2`,
  the cell footprint used by entities for bounds and vertical clamping.

### `render.rs` (rewrite of the cell model + flush)

- **`struct Cell { top: Option<Color>, bottom: Option<Color> }`** (was
  `{ ch, style }`); `blank()` = `{ None, None }`. `Frame` still stores
  `width × height` cells, addressed in **cells**; the pixel canvas is
  `width × height·2`. The old char accessors **`set(x,y,char)` and
  `cell(x,y)->char` are removed** along with the `ch` field.
- **`set_pixel(&mut self, px: i32, py: i32, color: Color)`** — `px ∈ [0,width)`,
  `py ∈ [0, height·2)`; writes cell `(px, py/2)` half `py%2`; out-of-range is
  clipped (no panic).
- **`pixel(&self, px: u16, py: u16) -> Option<Color>`** — reads the pixel at
  pixel coords (`py/2`/`py%2` resolved to the cell half). This is the pure
  read accessor the relocated `tank.rs`/`render.rs` tests assert on (the pixel
  analog of the deleted `cell()`).
- **`draw_sprite(&mut self, ox: i32, oy: i32, sprite: &PixelSprite)`** — `ox,oy`
  in cells (from `pos.round()`, as today). For each `Some(color)` pixel at
  sprite `(row, col)`: `set_pixel(ox + col, 2·oy + row, color)`. `None` pixels
  are transparent (skipped); everything clips to the frame.
- **`diff(&self, prev) -> Vec<(u16,u16,Cell)>`** is unchanged in shape — it
  compares whole `Cell`s, so a change in either half repaints the cell.
- **`full_changes(&self) -> Vec<(u16,u16,Cell)>`** — every cell as a change;
  used to force a full first paint (see *Water fill* below).
- **`flush_diff`** emits, per changed cell: `MoveTo(x,y)`,
  `SetForegroundColor(rgb(top))`, `SetBackgroundColor(rgb(bottom))`,
  `Print('▀')`, where `rgb(None)` = the water RGB. After the loop, one
  `ResetColor`. Because every cell sets both fg and bg explicitly, styles never
  bleed between cells. `to_ct(Color)` now returns `crossterm::Color::Rgb{..}`
  (truecolor, for exact palette colors).
- **`TerminalGuard`** is unchanged.

#### Water fill (terminal only)

Transparent pixels must read as water, not the terminal default. Moving fish
already repaint correctly (a vacated cell becomes `{None,None}` → diff reports
it → it paints water). The only gap is cells that are blank on the very first
frame (blank-vs-blank produces no diff). Fix: the terminal loop paints the
**full** frame once at startup and once after each resize (it already
`Clear`s on resize), via `full_changes`, then resumes diffing. The window path
needs nothing — `blit` repaints the whole buffer every frame over a water fill.

### `raster.rs` (rewrite of the blit; delete the font path)

- **Delete** the 8×8 font path: `font8x8.rs` (and its `mod` in `lib.rs`),
  `glyph`, `ascii_fallback`, and the glyph/`every_entity_glyph_renders_non_blank`
  tests. There are no text glyphs anymore.
- **`BG`** (water) is unchanged. **`rgb(Color) -> u32`** maps the new palette
  (no bold variants).
- **`grid_dims(px_w, px_h, scale) -> (u16,u16)`** — `cols = px_w/scale`,
  `rows = px_h/(2·scale)`, each clamped `≥ 1`. (Cells are `scale` wide and
  `2·scale` tall so window pixels stay square.)
- **`blit(frame, scale, px_w, px_h) -> Vec<u32>`** — fill `BG`; for each cell,
  paint its `top` pixel as a `scale×scale` block at `(cx·scale, 2·cy·scale)`
  and its `bottom` pixel at `(cx·scale, (2·cy+1)·scale)`; `None` halves are
  left as water; blocks clip at the buffer edge (no panic).

### `entity.rs` & `fish.rs` (re-skin)

- **`Entity::sprite(&self) -> PixelSprite`** (was `-> Sprite`) across every impl.
- **`bounds()`** for each actor uses the cell footprint:
  `w = sprite.cell_w() as f32`, `h = sprite.cell_h() as f32`. Movement helpers
  that take `sprite_w/sprite_h` (`swim_step`, `clamp_y`, `wrap_x`) are fed
  `cell_w()/cell_h()` — their signatures and logic are unchanged.
- **Fish art** (flat, ~13×8px, shared body+tail silhouette; personality is an
  overlay):
  - **Googly** — Cyan body, large White eye block with a Black pupil.
  - **Cool** — Blue body, a Black bar across the eyes (shades). (No bold; the
    bar carries the read.)
  - **Upside-down** — Green body with a small Black eye; `flip_v` toggled by
    its existing 5-second timer.
  - **Ducky** — Yellow duck silhouette with an Orange beak and a Black eye;
    still pinned to the top row, faced by `vx`.
  Facing is applied via the existing `facing_right` → `Facing` mapping.
- **Shark** — Grey body with a `Belly` underside, Black eye, Red mouth, ~17×8px.
  The "fatten per kill" tell is preserved by **inserting body pixel-columns**
  per kill (the pixel analog of today's `"#".repeat(7 + eaten)`): the sprite is
  built from `eaten`, widening the mid-body. `facing_right` mirrors it.
- **Food** — a small Orange pellet (2×2px → 2×1 cells). It still sinks, rests,
  and dissolves on the existing timers; the slightly larger bounds only makes
  it marginally easier to eat, which is harmless.

### `main.rs` & `src/bin/aquarium.rs` (frontend glue)

- **`main.rs`** — frame sizing (cols/rows = terminal size) is unchanged. Add a
  `needs_full_redraw` flag, set at startup and on resize, that flushes
  `frame.full_changes(&prev)`… i.e. paints all cells once, then reverts to
  `frame.diff(&prev)`.
- **`aquarium.rs`** — replace the font-cell constants with a pixel-block edge
  `PIXEL` (default `6`): `cell_w = PIXEL`, `cell_h = 2·PIXEL`; call the new
  `grid_dims(px_w, px_h, PIXEL)` and `blit(&frame, PIXEL, px_w, px_h)`. Initial
  window stays `960×600`. Key handling and the loop are otherwise unchanged.

### `examples/preview.rs`

Update to dump each entity's `PixelSprite` to stdout as ANSI half-blocks
(the same `▀` + truecolor encoding the terminal uses), so `cargo run --example
preview` still renders every sprite headlessly for eyeballing.

### `lib.rs`

Remove `pub mod font8x8;`. All other module declarations stand.

### Docs (`README.md`, `CLAUDE.md`)

Both currently describe the **ASCII** design and will actively misdescribe the
app after this change, so updating them is an explicit deliverable, not an
afterthought:

- `README.md` describes the cast by their ASCII art (`><(((°>`, `_(°)>`), the
  `Style { bold, color }` layer, and the window being "blitted from an embedded
  8×8 bitmap font." Rewrite those passages for the pixel-art model (palette,
  half-block terminal rendering, square-block window rendering, no font).
- `CLAUDE.md`'s architecture notes for `sprite.rs`/`render.rs`/`raster.rs`
  (the char-grid sprite, the `Style` layer, `flip_v` glyph-swapping via
  `mirror_char`, the `font8x8`/`ascii_fallback` path and its
  `every_entity_glyph_renders_non_blank` guard) are updated to the pixel model,
  and the **truecolor terminal requirement** is recorded.

## Data flow (unchanged simulation, new render)

```
Tank::update(dt)                 ← unchanged: build_ctx → update → resolve → retain
Tank::draw(&mut Frame)           ← unchanged call site; e.sprite() now → PixelSprite
  └─ frame.draw_sprite(pos.round().x, pos.round().y, &PixelSprite)
        └─ set_pixel(ox+col, 2·oy+row, color)   for each Some pixel

terminal:  frame.diff(prev) | frame.full_changes()  → flush_diff → ▀ fg=top bg=bottom
window:    blit(frame, PIXEL, px_w, px_h)           → square blocks over water fill
```

## Testing (logic-only, no TTY/window)

Unchanged and must stay green: the **position/count/bounds** tests in
`fish.rs`, `entity.rs`, `tank.rs` — movement, seek, flee, wrap/clamp, fish cap,
spread, food sink/dissolve, shark hunt/steer/cruise/despawn lifecycle,
`build_ctx` snapshots, `summon_shark`, `dead_entities_are_removed`,
`resize_clamps_entities`. None of these touch sprite glyphs or the char cell.

**Three existing tests are coupled to the deleted char model and must be
rewritten (not "unchanged"):**

- **`tank.rs::draw_places_entities_at_rounded_positions`** asserts
  `frame.cell(5,0) == '•'` / `frame.cell(0,9) == ' '`. The char `Cell`/`cell()`
  and the `'•'` glyph are gone. Rewrite to assert via the new pixel accessor:
  the dropped pellet lights at least one pixel at its rounded cell (e.g.
  `frame.pixel(px, py).is_some()` within the pellet's cell), and an empty
  region reads `None`.
- **`tank.rs::shark_eats_overlapping_fish`** asserts `shark.sprite().width()
  == 11` (today's ASCII base-10 + 1/kill). Under the pixel shark, `width()` is
  pixels and fattening inserts columns, so the literal is wrong. Rewrite to
  pin the same `resolve_shark → on_kill` wiring by asserting the shark's
  `sprite().width()` is **strictly greater after the kill than a fresh shark's**
  (no magic number).
- **`entity.rs::shark_fattens_as_it_eats`** compares `sprite().width()` before
  and after `on_kill()`. The invariant "width strictly increases per kill" still
  holds under column-insertion, so the assertion stands — but it is explicitly
  re-confirmed here because `width()`'s meaning changes from cells to pixels.

Rewritten / new:

- **`sprite.rs`**
  - `from_art` maps palette chars and treats `.`/space/unknown as transparent;
    ragged rows pad to max width.
  - `rendered_rows` mirrors **columns** for `Facing::Left` and reverses **rows**
    for `flip_v`; length is preserved.
  - `cell_w() == width()`; `cell_h() == ceil(height()/2)` (e.g. 8px → 4 cells,
    7px → 4 cells).
- **`render.rs`** — the old char-based tests (`blank_frame_is_all_spaces`,
  `draw_sprite_places_chars_and_skips_spaces`, `draw_clips_out_of_bounds`,
  `diff_*`, `draw_sprite_carries_style`) are replaced by:
  - a blank `Frame` reads `pixel(x,y) == None` everywhere.
  - `set_pixel` writes the correct cell half (even py → top, odd py → bottom),
    readable back via `pixel`, and clips out-of-range without panic.
  - `draw_sprite` places a sprite's pixels at `(ox+col, 2·oy+row)`, skips
    transparent pixels (a transparent pixel does not erase an existing one),
    and clips past the edge.
  - `diff` reports a cell when **either** half changes; `full_changes` returns
    every cell.
- **`raster.rs`**
  - `grid_dims` floors/clamps with the `2·scale` tall cell (e.g.
    `grid_dims(240, 240, 6) == (40, 20)`).
  - `blit` over a blank frame is all `BG`; a single colored pixel lights a
    `scale×scale` block in the correct half of its cell and leaves neighbors
    `BG`; a sprite past the edge clips without panic.
- **Replacement for the deleted glyph guard:** `every_entity_sprite_is_well_formed`
  — for the full cast (`Googly`, `Cool`, `Upsidedown`, `Ducky`, `Food`,
  `Shark`), `rendered_rows()` is non-empty, rectangular after padding, and
  contains at least one non-transparent pixel (so nothing renders as an empty
  hole). This is the pixel-era analog of `every_entity_glyph_renders_non_blank`.
- **Facing tests** in `fish.rs` are rewritten to assert reversed **pixel rows**
  (mirror) instead of reversed strings.

## Risks & mitigations

- **Truecolor requirement.** Half-block art needs 24-bit fg+bg. Modern
  terminals support it; this is documented in `CLAUDE.md`/README as a runtime
  requirement. (No graceful 256-color fallback — out of scope.)
- **Crowding.** Fish grow from 7×1 to ~13×4 cells. At `MAX_FISH = 30` in a
  150×40 terminal that is ~26% coverage — acceptable. `add_fish_at`'s spawn
  margins (`bounds.w - 8`, `bounds.h - 2`) widen to the new fish size
  (`-13`, `-4`) so spawns don't clip edges. If it still feels dense on a small
  terminal, `MAX_FISH` is the single lever; left at 30 unless eyeballing says
  otherwise.
- **Look needs eyeballing.** Per the project's TDD-logic-only convention, the
  actual art is verified via `cargo run --example preview`,
  `cargo build --features gui`, and a manual launch — not unit tests.
- **Per-fish art repetition.** Each fish hardcodes its own pixel grid, as each
  already hardcodes its own sprite string today. Accepted; a shared sprite-sheet
  abstraction is YAGNI for six actors.

## Out of scope (explicitly)

- 256-color / no-truecolor fallback rendering.
- Animation cycles, particle/bubble effects, backgrounds or décor pixels.
- Per-pixel arbitrary RGB (the curated palette is deliberate).
- Any change to controls, the shark behavior, the fish cap, or spawning logic
  beyond the spawn-margin size bump.
