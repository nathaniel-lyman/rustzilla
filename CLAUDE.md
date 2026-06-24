# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`rustzilla` is a CLI aquarium: a single Rust binary that animates ASCII gag-fish in the terminal at ~16 FPS, with keyboard interaction (`f` feed, `a` add fish, `s` summon shark, `q` quit). It's a deliberately silly toy that doubles as a showcase of tidy Rust.

## Commands

```bash
cargo run                     # launch the live aquarium (interactive TUI)
cargo run --features gui --bin aquarium   # launch the desktop-window aquarium (interactive window)
cargo run --example preview   # print every entity's sprite + ANSI styling (no TUI; safe to run headless)
cargo test                    # full logic suite
cargo test --lib fish         # one module's tests
cargo test fish_flips_to_face_the_food_it_chases   # a single test by name
cargo clippy --all-targets    # lint (CI-clean: keep zero warnings)
cargo fmt                     # format (keep `cargo fmt --check` clean)
```

**Do not run `cargo run` to verify changes in a headless/non-interactive context — it enters an alternate-screen loop that only exits on `q` and will hang.** The same applies to `cargo run --features gui --bin aquarium` — it opens a desktop-window event loop that only exits on `q`/window-close and will hang a headless run; verify that frontend with `cargo build --features gui --bin aquarium` (compiles, doesn't launch) and `cargo test` for its logic, and eyeball it via a manual launch. To eyeball sprite/art/style changes, use `cargo run --example preview` instead (it renders to stdout and exits). Logic changes are verified via `cargo test`.

## Architecture

A classic game loop over a double-buffered terminal canvas. There are two thin frontends over the same `lib`: `main.rs` (terminal, default `cargo run`) and `src/bin/aquarium.rs` (desktop window, `--features gui`). Both tick the same `Tank` and render it via `Tank::draw`; everything testable lives in `lib.rs` modules.

- `geom.rs` — `Vec2` / `Rect` math. Positions/velocities are `f32` in terminal **cell** units; rounded to integers only at draw time.
- `sprite.rs` — `Sprite` (a char grid) plus `Facing`, `flip_v`, and a `Style { bold, color }`. **Base art is authored facing right**; `rendered_rows()` mirrors it for `Facing::Left` (swapping paired glyphs like `<`↔`>`). Authoring art left-facing is the classic bug here — it renders reversed. `Color` is a terminal-agnostic enum so this module has no crossterm dependency.
- `entity.rs` — the `Entity` trait, the `Kind` enum (`Fish`/`Food`/`Shark`), the `TankCtx` snapshot, and the non-fish actors `Food` and `Shark`.
- `fish.rs` — the fish cast (`Googly`, `Cool`, `Upsidedown`, `Ducky`) plus shared movement helpers (`swim_step`, `wrap_x`, `clamp_y`, `step_toward`, `step_away`) and tank-scaled `sense_radius`/`fear_radius`.
- `tank.rs` — `Tank`: owns `Vec<Box<dyn Entity>>`, ticks the world, resolves collisions, enforces the fish cap, and spawns.
- `render.rs` — the in-memory `Frame` of styled `Cell`s (with `diff`), the `flush_diff` that emits per-cell crossterm bold/color, and `TerminalGuard`.
- `raster.rs` — the **window** render path: an embedded public-domain 8×8 bitmap font (`font8x8.rs`), `Color`→`u32` maps, and `blit` (a `Frame` → `Vec<u32>` pixel buffer). Pure and unit-tested; knows nothing about minifb. The terminal path (`render.rs`→crossterm) and window path (`raster.rs`→minifb) are symmetric frontends over the same `Tank`, so `lib` stays backend-agnostic.
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

The `Frame` stores styled `Cell`s; `diff` reports a change when **either** the char or the style differs; `flush_diff` applies bold/color then resets so styles don't bleed. "Weight" on screen comes from this style layer (e.g. the shark is bold red), keeping the underlying art simple ASCII. `sprite.rs` defines the color enum; `render.rs` maps it to crossterm.

### Interaction scaling

`sense_radius`/`fear_radius` are **proportional to tank size** (not fixed constants) — a fixed radius is invisible in a 150-cell-wide terminal, so reactions never fire. Spawns spread position/depth/speed/direction via a low-discrepancy sequence so fish don't pile up in a column or move in lockstep.

## Conventions

- **TDD, logic-only.** Tests target movement/bounds/seek/flee/collision/cap/spread/facing and the `Frame` buffer — all runnable without a TTY. Rendering and terminal I/O (`flush_diff`, `TerminalGuard`, the loop) have no unit tests; verify them via `cargo run --example preview` or by running the app manually.
- Keep `cargo clippy --all-targets` and `cargo fmt --check` clean — both are expected to pass with zero warnings.
- Commits are small and per-task; the spec and implementation plan live in `docs/superpowers/`.
