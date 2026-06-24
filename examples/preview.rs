// Throwaway visual check: print each fish facing both directions, plus the
// shark, so we can eyeball that art points the right way. Run: cargo run --example preview
use rustzilla::entity::{Entity, Shark};
use rustzilla::fish::{Cool, Ducky, Googly, Upsidedown};
use rustzilla::geom::Vec2;

fn show(label: &str, e: &dyn Entity) {
    println!("{label}:");
    for row in e.sprite().rendered_rows() {
        println!("  {row}");
    }
}

fn main() {
    let p = Vec2 { x: 0.0, y: 0.0 };
    println!("== moving RIGHT (vx > 0) ==");
    show("Googly", &Googly::new(p, 3.0));
    show("Cool", &Cool::new(p, 3.0));
    show("Upsidedown", &Upsidedown::new(p, 3.0));
    show("Ducky", &Ducky::new(p, 3.0));
    show("Shark", &Shark::new(p, 10.0));

    println!("\n== moving LEFT (vx < 0) ==");
    show("Googly", &Googly::new(p, -3.0));
    show("Cool", &Cool::new(p, -3.0));
    show("Ducky", &Ducky::new(p, -3.0));
}
