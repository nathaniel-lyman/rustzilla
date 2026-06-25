# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`rustzilla` is a CLI aquarium: a single Rust binary that animates pixel-art gag-fish in the terminal at ~16 FPS, with keyboard interaction (`f` feed, `a` add fish, `s` summon shark, `q` quit). It's a deliberately silly toy that doubles as a showcase of tidy Rust. The terminal frontend requires **24-bit truecolor** (the half-block pixel art uses truecolor fg+bg per cell).

## Commands

```bash
cargo run                     # launch the live aquarium (interactive TUI)
cargo run --features gui --bin aquarium   # launch the desktop-window aquarium (interactive window)
cargo run --example preview   # print every entity's sprite as ANSI half-blocks (no TUI; safe to run headless)
cargo test                    # full logic suite
cargo test --lib fish         # one module's tests
cargo test fish_flips_to_face_the_food_it_chases   # a single test by name
cargo clippy --all-targets    # lint (CI-clean: keep zero warnings)
cargo clippy --all-targets --features gui   # also lints the window bin (skipped without the feature)
cargo fmt                     # format (keep `cargo fmt --check` clean)
```

**Do not run `cargo run` to verify changes in a headless/non-interactive context — it enters an alternate-screen loop that only exits on `q` and will hang.** The same applies to `cargo run --features gui --bin aquarium` — it opens a desktop-window event loop that only exits on `q`/window-close and will hang a headless run; verify that frontend with `cargo build --features gui --bin aquarium` (compiles, doesn't launch) and `cargo test` for its logic, and eyeball it via a manual launch. To eyeball sprite/art/style changes, use `cargo run --example preview` instead (it renders to stdout and exits). Logic changes are verified via `cargo test`.

## Architecture

A classic game loop over a double-buffered terminal canvas. There are two thin frontends over the same `lib`: `main.rs` (terminal, default `cargo run`) and `src/bin/aquarium.rs` (desktop window, `--features gui`). Both tick the same `Tank` and render it via `Tank::draw`; everything testable lives in `lib.rs` modules.

- `geom.rs` — `Vec2` / `Rect` math. Positions/velocities are `f32` in terminal **cell** units; rounded to integers only at draw time.
- `sprite.rs` — `PixelSprite`: a rectangular grid of `Option<Color>` pixels (None = transparent), plus `Facing` and `flip_v`. No `Style`/bold layer. **Base art is authored facing right**; `rendered_rows()` mirrors for `Facing::Left` by reversing each row's column order (pixel column reversal, not glyph-swapping). Authoring already-mirrored art is the classic bug here — it renders reversed. `Color` is a terminal-agnostic 10-variant flat palette (`Cyan`, `Blue`, `Green`, `Yellow`, `Orange`, `White`, `Black`, `Grey`, `Belly`, `Red`) mapped per backend; this module has no crossterm dependency. `from_art(&[&str], &[(char,Color)])` builds a sprite from palette-indexed string rows. `cell_w()`/`cell_h()` give the terminal-cell footprint (1 cell = 1 pixel wide, 2 pixels tall).
- `entity.rs` — the `Entity` trait, the `Kind` enum (`Fish`/`Food`/`Shark`), the `TankCtx` snapshot, and the non-fish actors `Food` and `Shark`.
- `fish.rs` — the fish cast (`Googly`, `Cool`, `Upsidedown`, `Ducky`) plus shared movement helpers (`swim_step`, `wrap_x`, `clamp_y`, `step_toward`, `step_away`) and tank-scaled `sense_radius`/`fear_radius`.
- `tank.rs` — `Tank`: owns `Vec<Box<dyn Entity>>`, ticks the world, resolves collisions, enforces the fish cap, and spawns.
- `render.rs` — the in-memory `Frame` of `Cell`s, where each `Cell` holds two vertically-stacked pixels (`top: Option<Color>`, `bottom: Option<Color>`). `diff` reports a change when either half differs from the previous frame. `flush_diff` emits the `▀` half-block character with truecolor fg = top pixel and bg = bottom pixel (transparent halves use water `#0A1428`); one `ResetColor` at the end so styles never bleed. `full_changes` forces a full repaint (startup/resize). Also owns `TerminalGuard` (raw-mode/alt-screen RAII).
- `raster.rs` — the **window** render path: `Color`→`u32` maps and `blit` (a `Frame` → `Vec<u32>` pixel buffer). `blit` paints each pixel as a `scale×scale` square block over a water fill (`BG = 0x000A_1428`); `grid_dims` divides window height by `2*scale` so cells are `scale` wide × `2*scale` tall and pixels stay square on screen. Pure and unit-tested; knows nothing about minifb and has no embedded font. The terminal path (`render.rs`→crossterm) and window path (`raster.rs`→minifb) are symmetric frontends over the same `Tank`, so `lib` stays backend-agnostic. The `every_entity_sprite_is_well_formed` test guards that every entity sprite is non-empty, rectangular, and lights at least one pixel.
- `input.rs` — non-blocking key polling → `Input` (`Action` | `Resize`). The pure `action_for_key(char)` mapping is shared by both frontends (the terminal reads crossterm keys; `src/bin/aquarium.rs` maps minifb keys).

### The update pass (the key pattern)

`Tank::update` is borrow-safe by snapshotting before mutating:

1. `build_ctx` collects an **owned** `TankCtx` (tank bounds, dt, all food positions, the shark position) — this releases the immutable borrow of `entities`.
2. Every entity's `update(&ctx)` runs against that read-only snapshot. **Entities never mutate the tank**; spawn/removal authority stays solely in `Tank`.
3. `resolve_food` collects fish bounds into an owned `Vec` first, then marks overlapping pellets eaten (avoids aliasing).
4. `entities.retain(|e| !e.dead())` removes consumed food / exited sharks.

This snapshot-then-mutate shape is how every actor reads the world while the world is being mutated, without `RefCell` or indices. Preserve it.

### Facing & movement

Facing must track **actual** horizontal motion, not the static drift `vx`: a fish chasing food behind it flips around. `swim_step` returns `(new_pos, intended_dx)`; each fish stores a `facing_right` flag updated from `dx` each tick (ignoring negligible/vertical-only motion). Behavior priority inside `swim_step`: flee shark > seek nearest food in range > drift.

### Rendering & weight

Each `Cell` stores two stacked pixel colors (top and bottom halves of a terminal cell). `diff` reports a change when **either** half differs from the previous frame; `flush_diff` sets both fg and bg per cell and emits `▀`, so there is no color bleed between cells. "Weight" on screen comes from per-pixel palette color — the shark is grey pixel art with a black eye and a red mouth; food is a small orange pellet — not from a bold/style layer (there is no `Style` struct). `sprite.rs` defines the palette enum (`Color`); `render.rs` maps it to crossterm truecolor and `raster.rs` maps it to `u32` for the window path.

### Interaction scaling

`sense_radius`/`fear_radius` are **proportional to tank size** (not fixed constants) — a fixed radius is invisible in a 150-cell-wide terminal, so reactions never fire. Spawns spread position/depth/speed/direction via a low-discrepancy sequence so fish don't pile up in a column or move in lockstep.

## Conventions

- **TDD, logic-only.** Tests target movement/bounds/seek/flee/collision/cap/spread/facing, the `Frame` buffer, and the pure `raster` blitter — all runnable without a TTY or window. I/O glue (`flush_diff`, `TerminalGuard`, the terminal loop, and the whole `src/bin/aquarium.rs` window driver) has no unit tests; verify it via `cargo run --example preview`, `cargo build --features gui`, or a manual launch.
- Keep `cargo clippy --all-targets` and `cargo fmt --check` clean — both zero warnings. The window bin is gated behind the `gui` feature, so `--all-targets` alone skips it; also run `cargo clippy --all-targets --features gui` to lint it.
- Commits are small and per-task; the spec and implementation plan live in `docs/superpowers/`.
