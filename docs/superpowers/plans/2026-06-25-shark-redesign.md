# Shark Visual Redesign Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the shark's pixel-art sprite with a recognizable shark — heterocercal forked tail, dorsal fin, pectoral fin, gill slits, eye, and a closed white-tooth grin — without changing any behavior.

**Architecture:** Art-only change confined to `src/entity.rs`. `shark_rows(eaten)` keeps its `tail + stretchable mid + head` construction (so `m = 7 + eaten` still fattens the shark one column per kill); only the art constants change, plus the `from_art` palette map swaps the old red mouth (`'r'`/`Color::Red`) for white teeth (`'w'`/`Color::White`). All collision/flee/hunt/spawn logic reads `bounds()` from the sprite, so the larger footprint propagates automatically.

**Tech Stack:** Rust, `cargo test` / `cargo clippy` / `cargo fmt`, `cargo run --example preview` for eyeballing.

**Spec:** `docs/superpowers/specs/2026-06-25-shark-redesign-design.md`

---

## Chunk 1: Shark redesign

### Task 1: New shark sprite art + palette swap (TDD)

**Files:**
- Modify: `src/entity.rs:46-59` (the `shark_rows` function)
- Modify: `src/entity.rs:182-190` (the `from_art` palette map in `Shark::sprite`)
- Test: `src/entity.rs` `#[cfg(test)] mod tests` (add three tests near the existing shark tests, ~line 278)

**Context the implementer needs:**
- `PixelSprite` (`src/sprite.rs`) exposes `pub pixels: Vec<Vec<Option<Color>>>`, plus `width()` (pixels wide) and `height()` (pixels tall). Tests inspect colors by flattening `pixels`.
- `Color` and `Vec2` are already in scope in the test module via `use super::*`.
- The existing `shark_fattens_as_it_eats` test (relative width growth) and `every_entity_sprite_is_well_formed` (in `src/raster.rs`) must stay green — the new art is rectangular and grows by one column per kill, so they will.
- `Color::Red` stays in the enum: it is still matched in `render.rs`/`raster.rs` and constructed in their tests, so removing it from the shark map does **not** create dead-code warnings.

- [ ] **Step 1: Write the three failing tests**

Add to the `tests` module in `src/entity.rs` (next to `shark_fattens_as_it_eats`):

```rust
#[test]
fn shark_sprite_has_expected_base_size() {
    let s = Shark::new(Vec2 { x: 0.0, y: 0.0 }, 1.0).sprite();
    assert_eq!(s.width(), 20, "base shark is 20 px wide");
    assert_eq!(s.height(), 12, "base shark is 12 px tall");
}

#[test]
fn shark_sprite_uses_white_teeth_not_red() {
    let s = Shark::new(Vec2 { x: 0.0, y: 0.0 }, 1.0).sprite();
    let px: Vec<Color> = s.pixels.iter().flatten().flatten().copied().collect();
    assert!(px.contains(&Color::White), "teeth should be white");
    assert!(!px.contains(&Color::Red), "the old red mouth pixel is gone");
}

#[test]
fn shark_sprite_keeps_eye_and_gills() {
    let s = Shark::new(Vec2 { x: 0.0, y: 0.0 }, 1.0).sprite();
    let px: Vec<Color> = s.pixels.iter().flatten().flatten().copied().collect();
    let blacks = px.iter().filter(|c| **c == Color::Black).count();
    // One eye pixel + two 2-px gill slits = 5 black (the old art had just 1).
    assert!(blacks >= 3, "eye and gill slits should be present (got {blacks})");
}
```

- [ ] **Step 2: Run the new tests to verify they fail**

Run: `cargo test --lib shark_sprite_`
Expected: all three FAIL — size test sees 15×8, teeth test finds `Red`/no `White`, eye+gills test sees only 1 black pixel.

- [ ] **Step 3: Rewrite `shark_rows` with the new art**

Replace the body of `shark_rows` (`src/entity.rs:46-59`) with:

```rust
fn shark_rows(eaten: usize) -> Vec<String> {
    let m = 7 + eaten;
    // Fixed tail (left) + stretchable mid (one column repeated `m` times, grows
    // one column per kill) + fixed head (right). Every feature — fins, eye,
    // gill slits, teeth — lives in a fixed block, so a kill only lengthens the
    // smooth midsection. Palette: 'g' body, 'e' belly, 'k' eye+gills, 'w' teeth.
    let tail = [
        "g...", "gg..", ".gg.", "..gg", "...g", "...g", "...g", "...g", "..gg",
        ".gg.", "gg..", "....",
    ];
    let mid = ['.', '.', '.', 'g', 'g', 'g', 'g', 'e', 'e', '.', '.', '.'];
    let head = [
        "..gg.....", ".gggg....", ".ggggg...", "gggggggg.", "ggggggkgg",
        "ggkgkgggg", "ggkgkgggg", "eeeeewwww", "eeeeeeee.", ".ggg.....",
        ".gg......", ".........",
    ];
    (0..12)
        .map(|r| format!("{}{}{}", tail[r], mid[r].to_string().repeat(m), head[r]))
        .collect()
}
```

- [ ] **Step 4: Swap the palette map in `Shark::sprite`**

In `src/entity.rs` `Shark::sprite` (~line 182), change the `from_art` map from:

```rust
            &[
                ('g', Color::Grey),
                ('e', Color::Belly),
                ('k', Color::Black),
                ('r', Color::Red),
            ],
```

to:

```rust
            &[
                ('g', Color::Grey),
                ('e', Color::Belly),
                ('k', Color::Black),
                ('w', Color::White),
            ],
```

- [ ] **Step 5: Run the new tests + the full suite**

Run: `cargo test`
Expected: PASS — the three new tests pass, and all existing tests (including `shark_fattens_as_it_eats`, `every_entity_sprite_is_well_formed`, and the shark movement suite) stay green.

- [ ] **Step 6: Lint and format**

Run: `cargo clippy --all-targets && cargo clippy --all-targets --features gui && cargo fmt --check`
Expected: zero warnings, no formatting diff. (Run `cargo fmt` if the check reports a diff, then re-run the check.)

- [ ] **Step 7: Eyeball the sprite**

Run: `cargo run --example preview`
Expected: the `Shark:` block now shows fins, a forked tail, an eye, two gill slits, and a row of white teeth — facing right. (Sanity-check the silhouette looks like a shark.)

- [ ] **Step 8: Commit**

```bash
git add src/entity.rs
git commit -m "feat: redesign the shark sprite with fins, gills, and teeth"
```

### Task 2: Update the docs

**Files:**
- Modify: `CLAUDE.md` (the "Rendering & weight" line describing the shark)

- [ ] **Step 1: Update the shark description**

In `CLAUDE.md`, find the sentence that reads:

> the shark is grey pixel art with a black eye and a red mouth

Replace "a black eye and a red mouth" with a description matching the new art, e.g.:

> the shark is grey pixel art with fins, a black eye, gill slits, and a row of white teeth

(Keep the surrounding sentence about food being a small orange pellet intact.)

- [ ] **Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: describe the redesigned shark sprite"
```

---

## Verification (whole chunk)

- `cargo test` — full suite green.
- `cargo clippy --all-targets` and `cargo clippy --all-targets --features gui` — zero warnings.
- `cargo fmt --check` — clean.
- `cargo run --example preview` — shark renders as a recognizable shark, facing right.
- No changes outside `src/entity.rs` and `CLAUDE.md`.
