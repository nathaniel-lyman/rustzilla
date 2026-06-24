# Shark Eats Fish — Design

**Date:** 2026-06-24
**Status:** Approved (pending spec review)

## Summary

The shark currently cruises across the tank in a straight line and scares
fish (they flee within `fear_radius`), but it has no teeth: nothing happens
when it reaches a fish. This feature closes that predator loop. The shark
becomes an **active hunter** that steers toward the nearest fish, eats fish
it overlaps, visibly fattens with each kill, and leaves once it is full.

**Goals**
- Give the existing flee mechanic real stakes — being caught now means
  being eaten.
- Keep the change idiomatic and small: reuse the existing snapshot-then-mutate
  update pass and the `resolve_food` collision pattern rather than inventing
  new machinery.
- Stay fully unit-testable without a TTY (movement / overlap / counting / the
  full-then-leave lifecycle).

**Non-goals**
- No new keybinding — the feature rides the existing `s` (summon shark).
- No new rendering path, HUD, or score display.
- No persistent state, no fish "respawn on death" logic beyond the existing
  `a` (add fish) control.
- No multiple simultaneous sharks (`summon_shark` already allows only one).

## Behavior design

A summoned shark has one of two modes each tick, decided fresh from the
read-only `TankCtx` snapshot:

1. **Hunting** — when it has eaten fewer than `FULL_AFTER` fish **and** at
   least one fish lies within `hunt_radius`: it steers in 2D toward the
   nearest such fish at `HUNT_SPEED`, clamped inside the tank vertically.
2. **Cruising** — otherwise (full, or no prey in range): it moves straight
   horizontally at its summon velocity `vx`, exactly as today, and despawns
   once fully past the far edge.

Because cruising and the existing off-edge despawn are unchanged, the shark
**always leaves**: it reverts to a straight cruise the moment it is full
*or* has no prey in range, and that cruise carries it off an edge. The tank
can never be fully emptied by a single shark — it eats at most `FULL_AFTER`
fish, then goes.

**Eating.** A fish whose bounds overlap the shark's bounds is eaten: it is
marked dead and removed this tick, and the shark's kill counter increments.

**Visual feedback.** The shark's body widens by one segment per kill and
stays bold red, so its fullness is legible at a glance:

```
0 eaten:  <#######°>
1 eaten:  <########°>
2 eaten:  <#########°>
3 eaten:  <##########°>   (full → cruises off)
```

**Facing.** Like fish, the shark tracks the horizontal direction it actually
moved (`facing_right`) and mirrors its sprite accordingly, so a shark that
veers left to chase prey faces left. (Today the shark derives facing from the
sign of `vx`; hunting motion is 2D, so facing must follow actual `dx`.)

### Emergent consequences (no extra code)

- **Ducky is easy prey.** Ducky never flees (`fears_shark` is effectively
  false — it ignores the shark entirely) and is pinned to the top row, so a
  hunting shark that steers to the surface eats it readily.
- **Fleers can survive.** Googly, Cool, and Upsidedown flee at `FLEE_SPEED`
  (8.0). The shark hunts at `HUNT_SPEED` (10.0), slightly faster, so it
  closes distance over time — but a fish that breaks out of `hunt_radius`
  drops the shark back to cruising and may outlast it. Dodging is possible,
  not guaranteed.

## Tuning constants

| Constant | Value | Rationale |
|----------|-------|-----------|
| `FULL_AFTER` | `3` | Bounds the carnage; guarantees the shark leaves; small enough to see the sprite fatten fully. |
| `HUNT_SPEED` | `10.0` | ≥ fish `FLEE_SPEED` (8.0) so the shark can close on fleeing prey, but not so fast that dodging is pointless. Matches today's summon `vx` magnitude. |
| `hunt_radius(bounds)` | `(max(w,h) * 0.40).max(16.0)` | Tank-scaled like `sense_radius`/`fear_radius`, so the shark commits to prey at a visible range in large terminals. Slightly larger than `fear_radius` so a fish that has *started* fleeing is still, briefly, a target. |

These three numbers are the only balance levers and live as named constants
(no magic numbers at call sites).

## Architecture changes

Touches three modules only: `entity.rs`, `fish.rs`, `tank.rs`. No changes to
`render.rs`, `main.rs`, `input.rs`, `geom.rs`, or `sprite.rs`.

### `entity.rs`

- **`TankCtx` gains `pub fish: Vec<Vec2>`** — the positions of all fish this
  tick, collected the same way `food` and `shark` already are. This is the
  read-only channel through which the shark finds prey; entities still never
  mutate the tank.
- **`Entity` trait gains `fn on_kill(&mut self) {}`** — a default no-op hook,
  the predator-side parallel to the existing `on_eaten()`. Only `Shark`
  overrides it (to increment its kill counter). Rationale: the shark cannot
  self-count kills in its own `update()`, because `update()` runs against the
  pre-collision snapshot and has no authority to mark fish dead — collision
  resolution and removal stay solely in `Tank`, so `Tank` must inform the
  shark. A trait hook mirrors `on_eaten()` and avoids `Any`/downcasting.
- **`Shark` gains two fields:** `eaten: usize` (kills so far) and
  `facing_right: bool` (actual heading for sprite mirroring). `vx` is retained
  as the cruise/exit velocity (its sign still chooses the exit edge).
- **`Shark::update`** becomes mode-based:
  - If `eaten < FULL_AFTER` and `nearest(pos, &ctx.fish, hunt_radius(bounds))`
    yields a target: `step_toward(pos, target, HUNT_SPEED * dt)`, then
    `clamp_y` into bounds; update `facing_right` from the horizontal `dx`.
  - Else: `pos.x += vx * dt` (today's cruise); set `facing_right` from `vx`.
  - The off-edge despawn (`gone`) is unchanged.
- **`Shark::sprite`** builds the body width from `eaten`: a base run of `#`
  plus one `#` per kill, e.g. `format!("<{}°>", "#".repeat(7 + self.eaten))`,
  staying `.bold().colored(Color::Red)`, mirrored by `facing_right`.
- **`Shark::on_kill`** sets `self.eaten += 1`.

### `fish.rs`

- **Fish become mortal.** `Googly`, `Cool`, `Upsidedown`, and `Ducky` each
  gain an `eaten: bool` field (default `false`), override `on_eaten()` to set
  it `true`, and return it from `dead()` (today all four hardcode `dead() ->
  false`). This reuses the exact `Entity::on_eaten` hook `Food` already uses.
- **Generalize the nearest-target helper.** The private `nearest_food(p, food,
  radius)` becomes `pub fn nearest(p: Vec2, points: &[Vec2], radius: f32) ->
  Option<Vec2>`; the fish food-seek path and the shark hunt path both call it.
  Pure, already unit-shaped.
- **Add `pub fn hunt_radius(bounds: Rect) -> f32`** next to `sense_radius` /
  `fear_radius`, so all three tank-scaled radii live together.
- `step_toward`, `clamp_y`, and `wrap_x` are already `pub` and reused as-is.

### `tank.rs`

- **`build_ctx`** also collects fish positions (`filter(Kind::Fish).map(pos)`)
  into the new `TankCtx.fish` field, alongside the existing `food`/`shark`
  collection.
- **New `fn resolve_shark(&mut self)`**, called from `update()` after
  `resolve_food` and before the `retain` sweep. It mirrors `resolve_food`'s
  borrow-safe shape:
  1. Snapshot the shark's bounds into an owned `Option<Rect>` (releases the
     borrow). If `None`, return early.
  2. First mutable pass: for each `Kind::Fish` whose bounds overlap the shark
     bounds, call `on_eaten()` and count how many were newly eaten.
  3. Second mutable pass: find the single `Kind::Shark` and call `on_kill()`
     once per fish eaten this tick.
  4. The existing `entities.retain(|e| !e.dead())` in `update()` removes the
     eaten fish.

  Two separate passes (read shark bounds → mark fish → bump shark) means no
  simultaneous aliasing of `entities`, preserving the snapshot-then-mutate
  rule the codebase relies on.

### Update-pass ordering

`Tank::update` becomes:

```
let ctx = self.build_ctx(dt);     // now also snapshots fish positions
for e in &mut entities { e.update(&ctx); }
self.resolve_food();              // fish eat pellets (unchanged)
self.resolve_shark();             // shark eats fish (new)
entities.retain(|e| !e.dead());   // sweep eaten food + eaten fish + gone shark
```

Resolving food before shark is arbitrary (they touch disjoint kinds) but keeps
the “fish act, then consequences apply” reading.

## Data flow

```
Tank::update
  └─ build_ctx → TankCtx { bounds, dt, food[], fish[], shark? }   (owned snapshot)
       └─ Shark::update(ctx)
            ├─ hunting:  target = nearest(pos, ctx.fish, hunt_radius)
            │            pos = clamp_y(step_toward(pos, target, HUNT_SPEED·dt))
            └─ cruising: pos.x += vx·dt   (+ off-edge despawn)
  └─ resolve_shark
       ├─ shark_bounds (owned)            → mark overlapping fish on_eaten()
       └─ count kills                     → shark.on_kill() ×count
  └─ retain(!dead)                        → eaten fish + gone shark removed
```

The shark reads fish positions but never touches the fish; `Tank` alone marks
fish dead and tells the shark it scored. Authority stays centralized.

## Testing (logic-only, no TTY)

New tests (in `entity.rs`/`tank.rs` test modules, matching existing style):

- **`shark_steers_toward_nearest_fish`** — a shark with a fish off its axis
  moves so its distance to that fish strictly decreases over a tick.
- **`shark_eats_fish_on_overlap`** — after a tick where a fish overlaps the
  shark, the fish's `dead()` is `true` and the shark's `eaten` is `1`.
- **`shark_becomes_full_and_leaves`** — after `FULL_AFTER` kills, the shark
  ignores a fish placed directly in `hunt_radius` (no further steering toward
  it) and despawns off the edge within a bounded number of ticks.
- **`hungry_shark_with_no_prey_cruises_off`** — a shark with no fish present
  cruises straight and despawns off the edge (it leaves even while hungry).
- **`ducky_is_easy_prey`** — a Ducky in the shark's path is eaten (it never
  flees), confirming the emergent consequence.
- **`tankctx_includes_fish_positions`** — `build_ctx` populates `fish` with
  one entry per fish in the tank.
- **`shark_sprite_fattens_with_kills`** — `sprite().width()` is strictly
  greater after a kill than before.

Existing tests must stay green, including: `shark_cruises_horizontally`,
`shark_dies_after_leaving_the_far_side` (a shark with no fish still exits),
`summon_shark_adds_one_and_only_one`, and the food/cap/spread/facing suites.

## Risks & mitigations

- **Shark never leaves if prey stays in range.** Mitigated by `FULL_AFTER`:
  once full it stops hunting regardless of nearby fish, then cruises off.
- **`facing_right` regression on the existing cruise.** Mitigated by setting
  `facing_right` from `vx` in the cruise branch, preserving today's behavior
  for a shark that never finds prey.
- **Repetition across the four fish structs** (`eaten` field + `on_eaten` +
  `dead`). Accepted: it mirrors how each fish already repeats `pos`/`bounds`/
  `kind`; introducing a shared base type is out of scope and would be a larger
  refactor than the feature warrants (YAGNI).

## Out of scope (explicitly)

- Bubble/particle kill effects (a richer ambient system; rejected in favor of
  the self-contained sprite-fattening tell).
- Shark hunger over time, multiple sharks, or fish that fight back.
- Any score/HUD surface or sound.
