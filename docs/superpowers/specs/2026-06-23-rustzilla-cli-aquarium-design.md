# Rustzilla — CLI Aquarium Design

**Date:** 2026-06-23
**Status:** Approved (pending spec review)

## Summary

`rustzilla` is a small, elegant, and intentionally silly command-line
aquarium written in Rust. ASCII fish with visual gags drift around the
terminal on their own, and the user can lightly interact with the world:
drop food, add fish, or summon a shark. The project exists as a showcase
of idiomatic Rust — trait objects, RAII cleanup, a tidy game loop — wrapped
in something charming rather than serious.

**Goals**
- Demonstrate elegant, idiomatic Rust in a self-contained CLI toy.
- Be genuinely fun and funny to run (visual-gag fish).
- Stay small enough to finish in an afternoon and tidy enough to read.

**Non-goals**
- No persistent state / save files (not a tamagotchi).
- No mouse support, no config files, no networking.
- No pixel-perfect rendering tests.

## Audience & tone

A silly ambient toy that runs in the CLI. The humor lives in **visual
gags**: how the fish look and what happens to them (googly eyes, a fish in
a tiny hat, a rubber duck that wandered into the wrong app, a shark that
clears the room).

## Architecture

Single binary, structured as a classic game loop over a terminal canvas.

```
src/
  main.rs      → terminal setup/teardown, run the loop
  tank.rs      → the World: holds entities, ticks them, handles spawns
  fish.rs      → the Entity trait + the cast of fish types
  entity.rs    → Food and Shark (non-fish actors)
  render.rs    → double-buffered frame → terminal cells (crossterm)
  input.rs     → non-blocking key polling → Action enum
```

The loop runs at ~15–20 FPS:

```
loop {
    poll input → maybe an Action (Feed | AddFish | Shark | Quit)
    tank.update(dt)        // every entity moves/thinks
    render(&tank)          // diff against last frame, draw only changes
    sleep to hit frame budget (skip sleep if the tick ran long)
}
```

**Library choice:** `crossterm` for raw mode, alternate screen, non-blocking
key polling, and cursor positioning. Chosen over `ratatui` (too much
framework for a silly toy; hides the loop we want to show off) and over
raw ANSI escapes (hand-rolling non-blocking input is fiddly and gets crufty).

## The Entity trait (the elegant core)

```rust
trait Entity {
    fn update(&mut self, ctx: &TankCtx);  // move, react to food/shark
    fn sprite(&self) -> Sprite;           // ascii + facing direction
    fn pos(&self) -> Vec2;
    fn bounds(&self) -> Rect;
}
```

The tank holds `Vec<Box<dyn Entity>>` — fish, food, and the shark all
coexist behind one trait. `TankCtx` is a read-only view passed into
`update()` giving an entity what it needs to make decisions (tank bounds,
food positions, shark position if present, dt). Entities never mutate the
tank directly; the tank applies spawns/removals after the update pass.

Shared movement helpers (drift, edge wrap/bounce, seek-toward, flee-from)
keep each fish's `update()` tiny.

**`Sprite` contract.** A `Sprite` is a small fixed grid of characters plus a
facing flag (left/right) and a vertical-flip flag (used by Upsidedown). Its
width/height define the fish's `bounds()` (a `Rect` anchored at `pos()`).
Rendering mirrors the grid horizontally when facing left and vertically when
flipped. Multi-glyph decorations (e.g. Tophat's hat row) are just rows of the
grid, so bounds account for them automatically.

**Timestep.** `dt` is measured wall-time (seconds since the previous tick),
so movement is frame-rate independent. Tests pass an explicit `dt`, making
movement math deterministic without a real clock.

### The cast of fish

- **Googly** — normal drifter; oversized googly eyes are the gag.
- **Tophat** — dignified slow swimmer wearing a tiny `_°_` hat.
- **Upsidedown** — periodically flips and swims inverted for a while.
- **Ducky** — a rubber duck bobbing at the surface, clearly in the wrong
  app; ignores food and ignores the shark.

Each fish is a small struct implementing `Entity`, differing mainly in its
`update()` behavior and `sprite()`.

## Entities & behavior

**Food** — pressing `f` drops a pellet from the top that sinks at a steady
rate (trivial gravity `update()`). Fish within a sense radius switch to a
`seek` state and steer toward the nearest pellet; on contact the pellet is
consumed and removed. A pellet that reaches the bottom dissolves after a
short delay. The pellet itself is passive — the fish do the chasing.
"Contact" means the fish's `bounds()` overlaps the pellet's cell.

**Shark** — pressing `s` summons it from one edge. It cruises straight
across for a few seconds; every fish within range flips to a `flee` state
(darts away from the shark's x-position, faster than normal drift). Ducky
does not flee. When the shark exits the far side it despawns and fish relax
back to drifting. The shark only *scares* — it never eats fish or removes
any entity, and it ignores pellets.

**Add fish** — pressing `a` spawns a random fish type at a random depth.

**Population guard** — total fish are capped (30). Past the cap, `a` is a
gentle no-op so key-spam can't melt the frame rate.

### Controls

| Key | Action               |
|-----|----------------------|
| `f` | Drop food            |
| `a` | Add a random fish    |
| `s` | Summon the shark     |
| `q` | Quit                 |

## Robustness & teardown

The one genuinely important correctness concern in a TUI is **always
restoring the terminal**, including on panic.

- **`TerminalGuard`** — a struct that enables raw mode + alternate screen on
  creation and restores both (and shows the cursor) in its `Drop` impl, so
  cleanup runs on normal exit *and* during panic unwinding. (Doubles as an
  RAII showcase.)
- **Resize** — on a terminal-resize event, re-read the size and clamp all
  entity positions into the new bounds.
- **Frame budget** — if a tick overruns the frame budget, skip the sleep
  rather than accumulating lag.

## Testing

The toy is visual, so tests target *logic*, not pixels. `update()` methods
are pure with respect to `TankCtx`, so they test without a terminal.

- **Movement/bounds** — a fish near an edge wraps/bounces correctly;
  positions stay within tank bounds after `update`.
- **Food seeking** — given a pellet in range, a fish's next position is
  closer to it; on contact the pellet is consumed/removed.
- **Flee** — with a shark present, an affected fish moves away from it;
  Ducky does not move away.
- **Population cap** — spawning past the cap does not grow the entity `Vec`.
- **Render smoke test** — building a frame buffer for a populated tank fills
  without panicking (no assertion on exact contents).

## Build sequence (suggested)

1. `Vec2`/`Rect` math + `TerminalGuard` + an empty loop that quits on `q`.
2. `render.rs` double-buffer drawing a static fish; confirm no flicker.
3. The `Entity` trait + one drifting fish with edge wrap.
4. The rest of the cast (Tophat, Upsidedown, Ducky).
5. Food (`f`) + seek behavior.
6. Shark (`s`) + flee behavior; Ducky exceptions.
7. Add-fish (`a`) + population cap.
8. Resize handling + frame-budget polish.
9. Logic tests throughout (TDD per behavior).
