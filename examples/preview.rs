// Throwaway visual check: print each entity's pixel sprite to stdout as ANSI
// half-blocks (two pixel rows per text line) so we can eyeball the look.
// Run: cargo run --example preview
use rustzilla::entity::{Entity, Food, Shark};
use rustzilla::fish::{Cool, Ducky, Googly, Upsidedown};
use rustzilla::geom::Vec2;
use rustzilla::sprite::{Color, PixelSprite};

fn rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Cyan => (0x49, 0xD0, 0xE0),
        Color::Blue => (0x5B, 0x8C, 0xFF),
        Color::Green => (0x4F, 0xCF, 0x6F),
        Color::Yellow => (0xF2, 0xC6, 0x41),
        Color::Orange => (0xE8, 0x90, 0x2F),
        Color::White => (0xF2, 0xF2, 0xF2),
        Color::Black => (0x14, 0x14, 0x14),
        Color::Grey => (0x8A, 0x93, 0xA0),
        Color::Belly => (0xC9, 0xD0, 0xD8),
        Color::Red => (0xD8, 0x4A, 0x4A),
    }
}
const WATER: (u8, u8, u8) = (0x0A, 0x14, 0x28);

fn print_sprite(name: &str, s: &PixelSprite) {
    println!("\n{name}:");
    let rows = s.rendered_rows();
    let mut y = 0;
    while y < rows.len() {
        for x in 0..s.width() {
            let top = rows[y].get(x).copied().flatten().map(rgb).unwrap_or(WATER);
            let bottom = rows
                .get(y + 1)
                .and_then(|r| r.get(x))
                .copied()
                .flatten()
                .map(rgb)
                .unwrap_or(WATER);
            print!(
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                top.0, top.1, top.2, bottom.0, bottom.1, bottom.2
            );
        }
        println!("\x1b[0m");
        y += 2;
    }
}

fn main() {
    let p = Vec2 { x: 0.0, y: 0.0 };
    println!("== pixel sprites (facing RIGHT) ==");
    print_sprite("Googly", &Googly::new(p, 3.0).sprite());
    print_sprite("Cool", &Cool::new(p, 3.0).sprite());
    print_sprite("Upsidedown", &Upsidedown::new(p, 3.0).sprite());
    print_sprite("Ducky", &Ducky::new(p, 3.0).sprite());
    print_sprite("Food", &Food::new(p).sprite());
    print_sprite("Shark", &Shark::new(p, 10.0).sprite());
}
