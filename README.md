# 🐟 rustzilla

An elegant and intentionally silly terminal aquarium, written in Rust. Pixel-art
fish with visual gags drift around your terminal, living their little lives.
You can drop food and watch them swarm it, conjure more fish out of nowhere,
or summon a shark and watch the room clear — except for the rubber duck, who
is blissfully unaware of everything.

It exists mostly as a showcase of tidy, idiomatic Rust: a `Vec<Box<dyn Entity>>`
menagerie behind a single trait, a double-buffered render loop that only redraws
changed cells, and an RAII terminal guard that restores your terminal even if the
program panics.

> **Requires a 24-bit truecolor terminal** (iTerm2, Kitty, WezTerm, Windows
> Terminal, most modern emulators). The half-block pixel art uses truecolor
> fg+bg per cell; 256-color or 16-color terminals will look wrong.

## Run it

```bash
cargo run
```

Best in a terminal at least ~40 columns wide. Press `q` to quit (your terminal
is always restored cleanly on exit).

### …or on your desktop

The same aquarium runs in a resizable desktop window — a passive, ambient fish
tank you can park in a corner of your screen:

```bash
cargo run --features gui --bin aquarium
```

It keeps the same pixel-art look (each pixel is painted as a square block — no
font, no GPU, one small windowing crate) and the same controls (`f`/`a`/`s`/`q`)
when the window is focused. Resize the window and the tank
reflows to fill it. The window frontend is gated behind the optional `gui`
feature, so the default build stays dependency-light.

## Controls

| Key | Action            |
|-----|-------------------|
| `f` | Drop food — pellets sink and nearby fish chase them |
| `a` | Add a random fish (up to 30) |
| `s` | Summon the shark — it cruises across and scatters everyone |
| `q` | Quit |

## The cast

- **Googly** — your baseline drifter; cyan pixel body with a pair of oversized
  white eyes and black pupils. The eyes are the gag.
- **Cool** — blue pixel body crossed by a black sunglasses bar; too cool to hurry
  (half-speed).
- **Upside-down** — green; periodically flips vertically and swims inverted, as one does.
- **Ducky** — a yellow rubber duck with an orange beak, bobbing near the surface,
  clearly in the wrong app. Ignores food, and does **not** flee the shark (it has no idea).

Weight and personality come from per-pixel palette color, not from a bold/style
layer: the shark is grey pixel art with a black eye and a red mouth; food is a
small orange pellet; the fish each carry their own tints from a flat 10-color
palette. There is no `Style`/bold layer — the pixels do the work.

## How it works

- `src/geom.rs` — `Vec2` / `Rect` math (pure, fully unit-tested).
- `src/sprite.rs` — `PixelSprite`: a rectangular grid of `Option<Color>` pixels
  (None = transparent), with horizontal facing (column reversal) and vertical
  flip. Authored facing right; `from_art` builds from palette-indexed strings.
  `Color` is a terminal-agnostic 10-variant palette mapped per backend.
- `src/entity.rs` — the `Entity` trait, the `TankCtx` read-only world snapshot,
  and the non-fish actors (`Food`, `Shark`).
- `src/fish.rs` — the fish cast and the shared `swim_step` movement vocabulary.
- `src/tank.rs` — the `Tank`: holds every actor, ticks them, resolves
  fish-vs-food collisions, and enforces the population cap.
- `src/render.rs` — the in-memory `Frame` of half-block cells (each `Cell` holds
  a top and bottom `Option<Color>` pixel); `flush_diff` emits the `▀` half-block
  character with truecolor fg = top pixel and bg = bottom pixel (transparent →
  water `#0A1428`). Also owns `TerminalGuard` (raw-mode/alt-screen RAII).
- `src/raster.rs` — the window render path: `blit` paints each pixel as a
  `scale×scale` square block over a water fill; `grid_dims` divides window height
  by `2*scale` so cells are `scale` wide × `2*scale` tall and pixels stay square.
  Pure and unit-tested; no font, no windowing crate.
- `src/input.rs` — non-blocking key polling mapped to `Action`s; the pure
  `action_for_key` mapping is shared by both frontends.
- `src/main.rs` — the ~16 FPS terminal game loop tying it together.
- `src/bin/aquarium.rs` — the desktop-window frontend (`--features gui`): the
  same `Tank`, rendered via `raster::blit` as square pixel blocks into a `minifb`
  window.

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
