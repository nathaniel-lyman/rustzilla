use rustzilla::input::{poll_input, Action, Input};
use rustzilla::render::{flush_diff, Frame, TerminalGuard};
use crossterm::execute;
use rustzilla::tank::Tank;
use std::time::{Duration, Instant};

fn main() -> std::io::Result<()> {
    let mut guard = TerminalGuard::enter()?;
    let (mut cols, mut rows) = crossterm::terminal::size()?;

    let mut tank = Tank::new(cols, rows);
    // Seed a few fish so the tank isn't empty on launch.
    seed_fish(&mut tank, cols, rows);

    let frame_budget = Duration::from_millis(60); // ~16 FPS
    let mut prev = Frame::new(cols, rows);
    let mut last = Instant::now();

    loop {
        let tick_start = Instant::now();

        // --- input ---
        if let Some(input) = poll_input(Duration::from_millis(1))? {
            match input {
                Input::Action(Action::Quit) => break,
                Input::Action(Action::Feed) => tank.drop_food_at((cols / 2) as f32),
                Input::Action(Action::AddFish) => tank.add_fish_at(top_left_spawn(rows)),
                Input::Action(Action::Shark) => tank.summon_shark(),
                Input::Resize(w, h) => {
                    cols = w;
                    rows = h;
                    tank.resize(w, h);
                    prev = Frame::new(w, h);
                    execute!(
                        guard.stdout(),
                        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
                    )?;
                }
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
