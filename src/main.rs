use rustzilla::input::{poll_action, Action};
use rustzilla::render::{flush_diff, Frame, TerminalGuard};
use rustzilla::tank::Tank;
use std::time::{Duration, Instant};

fn main() -> std::io::Result<()> {
    let mut guard = TerminalGuard::enter()?;
    let (cols, rows) = crossterm::terminal::size()?;

    let mut tank = Tank::new(cols, rows);
    // Seed a few fish so the tank isn't empty on launch.
    seed_fish(&mut tank, cols, rows);

    let frame_budget = Duration::from_millis(60); // ~16 FPS
    let mut prev = Frame::new(cols, rows);
    let mut last = Instant::now();

    loop {
        let tick_start = Instant::now();

        // --- input ---
        if let Some(action) = poll_action(Duration::from_millis(1))? {
            match action {
                Action::Quit => break,
                Action::Feed => tank.drop_food_at((cols / 2) as f32),
                Action::AddFish => tank.add_fish_at(top_left_spawn(rows)),
                Action::Shark => tank.summon_shark(),
            }
        }

        // --- update ---
        let now = Instant::now();
        let dt = (now - last).as_secs_f32();
        last = now;
        tank.update(dt);

        // --- render ---
        let mut frame = Frame::new(cols, rows);
        for e in tank.entities() {
            let p = e.pos();
            frame.draw_sprite(p.x.round() as i32, p.y.round() as i32, &e.sprite());
        }
        let changes = frame.diff(&prev);
        flush_diff(guard.stdout(), &changes)?;
        prev = frame;

        // --- frame budget ---
        let elapsed = tick_start.elapsed();
        if elapsed < frame_budget {
            std::thread::sleep(frame_budget - elapsed);
        }
    }

    Ok(()) // TerminalGuard restores the terminal as it drops here.
}

fn top_left_spawn(rows: u16) -> rustzilla::geom::Vec2 {
    rustzilla::geom::Vec2 { x: 2.0, y: (rows / 3) as f32 }
}

fn seed_fish(tank: &mut Tank, cols: u16, rows: u16) {
    let _ = cols;
    for i in 0..6 {
        tank.add_fish_at(rustzilla::geom::Vec2 { x: 2.0, y: (2 + i * 2 % rows.max(4)) as f32 });
    }
}
