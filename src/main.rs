use crossterm::execute;
use rustzilla::input::{poll_input, Action, Input};
use rustzilla::render::{flush_diff, Frame, TerminalGuard};
use rustzilla::tank::Tank;
use std::time::{Duration, Instant};

fn main() -> std::io::Result<()> {
    let mut guard = TerminalGuard::enter()?;
    let (raw_cols, raw_rows) = crossterm::terminal::size()?;
    // Guard against a degenerate 0-sized terminal (can happen transiently on
    // some resizes) so we never busy-spin rendering an empty frame.
    let (mut cols, mut rows) = (raw_cols.max(1), raw_rows.max(1));

    let mut tank = Tank::new(cols, rows);
    // Seed a few fish so the tank isn't empty on launch. add_fish_at spreads
    // them across the tank with varied speeds, so they don't pile up.
    for _ in 0..6 {
        tank.add_fish_at();
    }

    let frame_budget = Duration::from_millis(60); // ~16 FPS
    let mut prev = Frame::new(cols, rows);
    let mut last = Instant::now();
    let mut needs_full = true;

    loop {
        let tick_start = Instant::now();

        // --- input ---
        if let Some(input) = poll_input(Duration::from_millis(1))? {
            match input {
                Input::Action(Action::Quit) => break,
                Input::Action(Action::Feed) => tank.feed(),
                Input::Action(Action::AddFish) => tank.add_fish_at(),
                Input::Action(Action::Shark) => tank.summon_shark(),
                Input::Resize(w, h) => {
                    cols = w.max(1);
                    rows = h.max(1);
                    tank.resize(cols, rows);
                    prev = Frame::new(cols, rows);
                    needs_full = true;
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
        tank.draw(&mut frame);
        let changes = if needs_full {
            frame.full_changes()
        } else {
            frame.diff(&prev)
        };
        flush_diff(guard.stdout(), &changes)?;
        needs_full = false;
        prev = frame;

        // --- frame budget ---
        let elapsed = tick_start.elapsed();
        if elapsed < frame_budget {
            std::thread::sleep(frame_budget - elapsed);
        }
    }

    Ok(()) // TerminalGuard restores the terminal as it drops here.
}
