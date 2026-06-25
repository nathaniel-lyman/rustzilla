# Shark Visual Redesign — Design

**Date:** 2026-06-25
**Status:** Approved (pending spec review)

## Summary

The shark's current sprite is a grey wedge with a lighter belly, a single
black eye, and one red mouth pixel — **no fins of any kind**. It reads more
like a generic fish than a shark. This change replaces the art so the shark is
unmistakable: a **heterocercal forked tail**, a triangular **dorsal fin**, a
downward **pectoral fin**, two **gill slits**, a black eye, and a **closed
white-tooth grin**.

It is an **art-only change**. No behavior, movement, collision, hunting, or
spawn logic changes. The redesign deliberately keeps the shark's existing
`tail + stretchable mid + head` row construction, so the "fattens one column
per kill" mechanic survives untouched — only the art constants and the sprite's
palette map change.

**Goals**
- Make the shark read as a shark at a glance, at ~16 FPS in a terminal.
- Preserve every existing behavior and the snapshot-then-mutate architecture:
  touch only the art.
- Keep the kill-stretch tell (`m = 7 + eaten`) working, with fins/eye/teeth
  fixed and only the smooth midsection growing.
- Stay fully unit-testable without a TTY (the sprite is pure data).

**Non-goals**
- No movement, hunting, collision, spawn-count, or lifecycle changes.
- No new palette variant (`White` already exists; the teeth reuse it).
- No new render path, animation, or per-fin behavior.
- No change to fish, food, `tank.rs`, `render.rs`, `raster.rs`, or `sprite.rs`.

## Visual design

Authored **facing right** (snout on the right), per the sprite convention;
`Facing::Left` is produced by the engine's per-row column reversal, so there is
no separate left-facing art.

Size grows from **15×8 px (15 cells × 4)** to **20×12 px (20 cells × 6)** at the
base (pre-kill) size. The sprite is 12 px tall = exactly 6 terminal cell rows
(no half-empty bottom row).

Palette (unchanged variants): `g` → `Grey` body, `e` → `Belly` light underside,
`k` → `Black` eye + gill slits, `w` → `White` teeth. The old `r` → `Red` mouth
pixel is **removed** — a closed grin shows white teeth, not red gums.

### Row construction (preserves the stretch mechanic)

`shark_rows(eaten)` keeps its three-part shape: a fixed **tail** block (left), a
single-column **mid** repeated `m = 7 + eaten` times (the only stretchable
part), and a fixed **head** block (right). All features live in the fixed
blocks; the mid is plain torso, so a kill simply lengthens the smooth
midsection.

**TAIL** (4 cols, fixed) — heterocercal fork, upper lobe longer:
```
g...
gg..
.gg.
..gg
...g
...g
...g
...g
..gg
.gg.
gg..
....
```

**MID** (1 col, repeated `m` times) — plain torso, grey over belly:
```
. . . g g g g e e . . .
```
(read top-to-bottom: rows 0–2 transparent, 3–6 `g`, 7–8 `e`, 9–11 transparent)

**HEAD** (9 cols, fixed) — dorsal fin (rows 0–2), eye (`k`, row 4), two gill
slits (`k`, rows 5–6), rounded snout (right), white teeth (`w`, row 7), pectoral
fin (rows 9–10):
```
..gg.....
.gggg....
.ggggg...
gggggggg.
ggggggkgg
ggkgkgggg
ggkgkgggg
eeeeewwww
eeeeeeee.
.ggg.....
.gg......
.........
```

Assembled base (`eaten = 0`, `m = 7`), 20×12:
```
g............gg.....
gg..........gggg....
.gg.........ggggg...
..ggggggggggggggggg.
...ggggggggggggggkgg
...ggggggggggkgkgggg
...ggggggggggkgkgggg
...geeeeeeeeeeeewwww
..ggeeeeeeeeeeeeeee.
.gg.........ggg.....
gg..........gg......
....................
```

The dorsal sits just past the stretch seam (slightly forward of dead-center, an
accepted stylization so it stays attached to the fixed head block). The pectoral
fin sits just behind/below the head. Gills are two black slits behind the eye.

## Architecture changes

Touches **one module**: `entity.rs`. Nothing else changes.

### `entity.rs`

- **`shark_rows(eaten)`** is rewritten with the new `TAIL` / `MID` / `HEAD`
  constants above. It keeps the same signature and the same `m = 7 + eaten`
  width growth, and keeps emitting rectangular rows (every row is
  `4 + m + 9` chars wide).
- **`Shark::sprite`**'s `from_art` palette map drops `('r', Color::Red)` and
  adds `('w', Color::White)`. The remaining entries (`g`/`e`/`k`) are unchanged.
  Facing handling is unchanged.

No change to `Shark`'s fields, `update`, `bounds`, `pos`, `kind`, `dead`, or
`on_kill`. `bounds`/`cell_w`/`cell_h` derive from the sprite, so the larger
footprint propagates to collision/flee/despawn automatically.

### Docs

- Update the CLAUDE.md line describing the shark ("grey pixel art with a black
  eye and a red mouth") to reflect the new eye + white-tooth grin and fins.

## Size / bounds impact

The shark is now 20 cells wide (was 15) and 6 cells tall (was 4):

- **Collision / flee / hunt** all read `bounds()` from the sprite, so they scale
  automatically — no logic change, just a slightly larger hitbox.
- **Despawn** uses `cell_w()`; the wider sprite still exits correctly (the
  `shark_dies_after_leaving_the_far_side` test uses a 40-wide tank, ample room).
- **Spawn** (`tank.rs::summon_shark`) places the shark at vertical center
  (`bounds.h * 0.5`) and offset left by 6 cells. At 6 cells tall it remains
  fully on-screen for any tank ≥ ~12 rows tall (real terminals); in a
  pathologically short tank it could clip the floor by a row, exactly as the
  4-cell sprite already could. Not worth special-casing — see Risks.

## Testing (logic-only, no TTY)

**Existing tests that must stay green** (all in `entity.rs`/`tank.rs`/`raster.rs`):
- `shark_fattens_as_it_eats` — `sprite().width()` strictly increases per
  `on_kill()`. Preserved: `m = 7 + eaten` still grows the mid by one column.
- `every_entity_sprite_is_well_formed` (raster.rs) — the shark sprite stays
  non-empty, rectangular, and lights ≥ 1 pixel. The new art satisfies all three.
- `shark_cruises_horizontally`, `shark_dies_after_leaving_the_far_side`,
  `shark_steers_toward_nearest_fish`, `full_shark_stops_hunting_and_cruises`,
  `hungry_shark_with_no_prey_cruises_off`, `summon_shark_adds_one_and_only_one`,
  and the food/cap/spread/facing suites — none depend on the exact art, so they
  remain green.

**New tests** (in the `entity.rs` test module, matching existing style):
- `shark_sprite_has_expected_base_size` — a fresh shark's `sprite().width()`
  and `sprite().height()` equal the new base dimensions (20 px wide, 12 px
  tall), pinning the art's footprint so accidental ragged edits are caught.
- `shark_sprite_uses_white_teeth_not_red` — the base sprite contains at least
  one `White` pixel and **no** `Red` pixel (guards the mouth swap and that the
  teeth render).
- `shark_keeps_eye_and_gills` — the base sprite contains `Black` pixels (eye +
  gills present).

(Pixel-color assertions read the sprite's pixel grid directly, the same way the
existing well-formed check inspects sprite cells — no TTY needed.)

## Risks & mitigations

- **Art looks wrong / mirrors wrong.** Mitigated by authoring strictly
  facing-right and verifying with `cargo run --example preview` (renders the
  shark and the auto-mirrored faces to stdout). The size/teeth/Black-pixel tests
  catch structural regressions.
- **Taller sprite clips the floor in a very short tank.** Pre-existing
  condition (the 4-cell sprite could too); real terminals are tall enough and
  the shark cruises through within a second. Out of scope to re-architect spawn
  clamping for a 2-row edge case.
- **A test secretly depends on the old exact width (15).** Verified none do —
  `shark_fattens_as_it_eats` only checks relative growth; no absolute-width
  assertion exists today. The new size test makes the footprint explicit going
  forward.

## Out of scope (explicitly)

- Animated fins, tail-swish, or bubbles.
- Any behavioral retuning (hunt speed, fear radius, fullness threshold).
- Re-theming the other entities (fish/food) to match.
- Spawn-clamping changes for short tanks.
