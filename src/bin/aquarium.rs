//! Desktop-window frontend: the same Tank as the terminal app, rendered into a
//! resizable window via a hand-rolled glyph blitter over a minifb pixel buffer.
//! This is I/O glue (no unit tests) — verified by compiling and by launching.
//! Launch: `cargo run --features gui --bin aquarium`.
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use rustzilla::input::{action_for_key, Action};
use rustzilla::raster::{self, blit};
use rustzilla::render::Frame;
use rustzilla::tank::Tank;
use std::time::{Duration, Instant};

const SCALE: usize = 6; // each pixel is a 6x6 block; cells are 6 wide x 12 tall
const FRAME_BUDGET: Duration = Duration::from_millis(60); // ~16 FPS
const MAX_DT: f32 = 0.1; // clamp so a paused/occluded window doesn't teleport fish

/// The four keys the tank reacts to, mapped to the chars `action_for_key` knows.
fn key_char(key: Key) -> Option<char> {
    match key {
        Key::F => Some('f'),
        Key::A => Some('a'),
        Key::S => Some('s'),
        Key::Q => Some('q'),
        _ => None,
    }
}

fn main() {
    let (mut px_w, mut px_h) = (960usize, 600usize);
    let mut window = Window::new(
        "rustzilla",
        px_w,
        px_h,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )
    .expect("failed to open window");

    let (mut cols, mut rows) = raster::grid_dims(px_w, px_h, SCALE);
    let mut tank = Tank::new(cols, rows);
    for _ in 0..6 {
        tank.add_fish_at(); // seed a few fish, like the terminal app
    }

    let mut last = Instant::now();
    while window.is_open() {
        let tick_start = Instant::now();

        // --- input: focused keys → char → shared action_for_key ---
        for key in window.get_keys_pressed(KeyRepeat::No) {
            if let Some(c) = key_char(key) {
                match action_for_key(c) {
                    Some(Action::Quit) => return,
                    Some(Action::Feed) => tank.feed(),
                    Some(Action::AddFish) => tank.add_fish_at(),
                    Some(Action::Shark) => tank.summon_shark(),
                    None => {}
                }
            }
        }

        // --- resize: window pixels → grid ---
        let (w, h) = window.get_size();
        if (w, h) != (px_w, px_h) {
            px_w = w;
            px_h = h;
            let (c, r) = raster::grid_dims(px_w, px_h, SCALE);
            if (c, r) != (cols, rows) {
                cols = c;
                rows = r;
                tank.resize(cols, rows);
            }
        }

        // --- update ---
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(MAX_DT);
        last = now;
        tank.update(dt);

        // --- render ---
        let mut frame = Frame::new(cols, rows);
        tank.draw(&mut frame);
        let buf = blit(&frame, SCALE, px_w, px_h);
        window
            .update_with_buffer(&buf, px_w, px_h)
            .expect("failed to present frame");

        // --- frame budget (~16 FPS, mirrors main.rs) ---
        let elapsed = tick_start.elapsed();
        if elapsed < FRAME_BUDGET {
            std::thread::sleep(FRAME_BUDGET - elapsed);
        }
    }
}
