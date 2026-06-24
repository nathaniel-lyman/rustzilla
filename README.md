# 🐟 rustzilla

An elegant and intentionally silly terminal aquarium, written in Rust. ASCII
fish with visual gags drift around your terminal, living their little lives.
You can drop food and watch them swarm it, conjure more fish out of nowhere,
or summon a shark and watch the room clear — except for the rubber duck, who
is blissfully unaware of everything.

It exists mostly as a showcase of tidy, idiomatic Rust: a `Vec<Box<dyn Entity>>`
menagerie behind a single trait, a double-buffered render loop that only redraws
changed cells, and an RAII terminal guard that restores your terminal even if the
program panics.

## Run it

```bash
cargo run
```

Best in a terminal at least ~40 columns wide. Press `q` to quit (your terminal
is always restored cleanly on exit).

## Controls

| Key | Action            |
|-----|-------------------|
| `f` | Drop food — pellets sink and nearby fish chase them |
| `a` | Add a random fish (up to 30) |
| `s` | Summon the shark — it cruises across and scatters everyone |
| `q` | Quit |

## The cast

- **Googly** `><(((°>` — your baseline drifter; the oversized eyes are the gag.
- **Cool** `><(((⊙>` — a fish in shades, too cool to hurry (cruises at half-speed).
- **Upside-down** — periodically flips and swims inverted for a while, as one does.
- **Ducky** `_(°)<` — a rubber duck bobbing at the surface, clearly in the wrong
  app. Ignores food, and does **not** flee the shark (it has no idea).

## How it works

- `src/geom.rs` — `Vec2` / `Rect` math (pure, fully unit-tested).
- `src/sprite.rs` — `Sprite`: a char grid with horizontal facing and vertical flip.
- `src/entity.rs` — the `Entity` trait, the `TankCtx` read-only world snapshot,
  and the non-fish actors (`Food`, `Shark`).
- `src/fish.rs` — the fish cast and the shared `swim_step` movement vocabulary.
- `src/tank.rs` — the `Tank`: holds every actor, ticks them, resolves
  fish-vs-food collisions, and enforces the population cap.
- `src/render.rs` — the in-memory `Frame` buffer + diffing, and the
  `TerminalGuard` that owns raw-mode/alt-screen setup and teardown.
- `src/input.rs` — non-blocking key polling mapped to `Action`s.
- `src/main.rs` — the ~16 FPS game loop tying it together.

Each tick the tank builds an owned `TankCtx` (food and shark positions), then
lets every entity `update()` itself against that read-only snapshot — which is
how fish read the world while the world is being mutated, without fighting the
borrow checker.

## Tests

```bash
cargo test
```

The suite targets logic, not pixels: movement and bounds, food-seeking,
collision, the shark lifecycle, the population cap, and resize clamping — all
runnable without a real terminal.
