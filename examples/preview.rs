// Throwaway visual check: print each entity with its real bold/color styling
// (via raw ANSI) so we can eyeball the look. Run: cargo run --example preview
use rustzilla::entity::{Entity, Food, Shark};
use rustzilla::fish::{Cool, Ducky, Googly, Upsidedown};
use rustzilla::geom::Vec2;
use rustzilla::sprite::{Color, Sprite};

fn ansi_prefix(s: &Sprite) -> String {
    let mut out = String::new();
    if s.style.bold {
        out.push_str("\x1b[1m");
    }
    if let Some(c) = s.style.color {
        let code = match c {
            Color::Red => 31,
            Color::Green => 32,
            Color::Yellow => 33,
            Color::Blue => 34,
            Color::Cyan => 36,
            Color::White => 37,
        };
        out.push_str(&format!("\x1b[{code}m"));
    }
    out
}

fn show(label: &str, e: &dyn Entity) {
    let s = e.sprite();
    let pre = ansi_prefix(&s);
    println!("{label}:");
    for row in s.rendered_rows() {
        println!("  {pre}{row}\x1b[0m");
    }
}

fn main() {
    let p = Vec2 { x: 0.0, y: 0.0 };
    println!("== moving RIGHT (vx > 0) ==");
    show("Googly", &Googly::new(p, 3.0));
    show("Cool", &Cool::new(p, 3.0));
    show("Upsidedown", &Upsidedown::new(p, 3.0));
    show("Ducky", &Ducky::new(p, 3.0));
    show("Food", &Food::new(p));
    show("Shark", &Shark::new(p, 10.0));

    println!("\n== moving LEFT (vx < 0) ==");
    show("Googly", &Googly::new(p, -3.0));
    show("Cool", &Cool::new(p, -3.0));
    show("Ducky", &Ducky::new(p, -3.0));
}
