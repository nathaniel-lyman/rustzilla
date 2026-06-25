use crate::render::Frame;
use crate::sprite::Color;

/// Tank background (deep blue-black), packed 0x00RRGGBB.
pub const BG: u32 = 0x000A_1428;

/// Map a logical color to a packed 0x00RRGGBB pixel. Symmetric with
/// `render::to_ct` (which maps the same enum to crossterm).
pub fn rgb(color: Color) -> u32 {
    match color {
        Color::Red => 0x00D8_4A4A,
        Color::Yellow => 0x00F2_C641,
        Color::Green => 0x004F_CF6F,
        Color::Cyan => 0x0049_D0E0,
        Color::Blue => 0x005B_8CFF,
        Color::White => 0x00F2_F2F2,
        Color::Orange => 0x00E8_902F,
        Color::Black => 0x0014_1414,
        Color::Grey => 0x008A_93A0,
        Color::Belly => 0x00C9_D0D8,
    }
}

/// Window pixels → tank grid `(cols, rows)`. Cells are `scale` wide and
/// `2*scale` tall so on-screen pixels stay square. Clamps each dim to >= 1.
pub fn grid_dims(px_w: usize, px_h: usize, scale: usize) -> (u16, u16) {
    let cols = (px_w / scale.max(1)).max(1) as u16;
    let rows = (px_h / (2 * scale.max(1))).max(1) as u16;
    (cols, rows)
}

/// Render a `Frame` into a `px_w × px_h` buffer. Each pixel is a
/// `scale × scale` square block; the top pixel sits at window-y `2*cy*scale`,
/// the bottom at `(2*cy+1)*scale`. Transparent pixels leave water (`BG`).
/// Blocks past the buffer edge clip — never a panic.
pub fn blit(frame: &Frame, scale: usize, px_w: usize, px_h: usize) -> Vec<u32> {
    let mut buf = vec![BG; px_w * px_h];
    let s = scale.max(1);
    let mut paint = |sub_x: usize, sub_y: usize, color: u32| {
        for dy in 0..s {
            let y = sub_y * s + dy;
            if y >= px_h {
                break;
            }
            for dx in 0..s {
                let x = sub_x * s + dx;
                if x >= px_w {
                    break;
                }
                buf[y * px_w + x] = color;
            }
        }
    };
    for cy in 0..frame.height {
        let cy = cy as usize;
        for cx in 0..frame.width {
            let cx = cx as usize;
            // Compute the pixel-row offsets in usize to avoid any u16 overflow.
            let top = 2 * cy;
            let bottom = 2 * cy + 1;
            if let Some(c) = frame.pixel(cx as u16, top as u16) {
                paint(cx, top, rgb(c));
            }
            if let Some(c) = frame.pixel(cx as u16, bottom as u16) {
                paint(cx, bottom, rgb(c));
            }
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite::Color;

    #[test]
    fn every_entity_sprite_is_well_formed() {
        // Regression guard: every entity's pixel sprite must be rectangular and
        // light at least one pixel, so nothing renders as a fully blank hole.
        use crate::entity::{Entity, Food, Shark};
        use crate::fish::{Cool, Ducky, Googly, Upsidedown};
        use crate::geom::Vec2;
        let p = Vec2 { x: 0.0, y: 0.0 };
        let cast: Vec<Box<dyn Entity>> = vec![
            Box::new(Googly::new(p, 1.0)),
            Box::new(Cool::new(p, 1.0)),
            Box::new(Upsidedown::new(p, 1.0)),
            Box::new(Ducky::new(p, 1.0)),
            Box::new(Food::new(p)),
            Box::new(Shark::new(p, 1.0)),
        ];
        for e in &cast {
            let rows = e.sprite().rendered_rows();
            assert!(!rows.is_empty(), "sprite has no rows");
            let w = rows[0].len();
            assert!(rows.iter().all(|r| r.len() == w), "sprite rows are ragged");
            assert!(
                rows.iter().flatten().any(|p| p.is_some()),
                "sprite renders fully blank"
            );
        }
    }

    #[test]
    fn grid_dims_uses_tall_cells() {
        // Cells are `scale` wide and `2*scale` tall, so vertical divides by 2*scale.
        assert_eq!(grid_dims(240, 240, 6), (40, 20));
        assert_eq!(grid_dims(10, 10, 6), (1, 1)); // clamps to >= 1
    }

    #[test]
    fn blit_blank_frame_is_all_water() {
        let f = Frame::new(3, 2);
        let buf = blit(&f, 4, 12, 16);
        assert!(buf.iter().all(|&p| p == BG));
    }

    #[test]
    fn blit_lights_correct_half_blocks() {
        let mut f = Frame::new(2, 1); // canvas 2x2 px
        f.set_pixel(0, 0, Color::Cyan); // top half of cell (0,0)
        f.set_pixel(0, 1, Color::Red); // bottom half of cell (0,0)
        let buf = blit(&f, 1, 2, 2); // scale 1 -> 1px blocks; buffer 2x2
        assert_eq!(buf[0], rgb(Color::Cyan)); // (0,0) top: row 0, col 0
        assert_eq!(buf[2], rgb(Color::Red)); // (0,1) bottom: row 1, col 0
        assert_eq!(buf[1], BG); // neighbour cell stays water: row 0, col 1
    }

    #[test]
    fn blit_clips_past_the_edge() {
        let mut f = Frame::new(2, 2);
        f.set_pixel(1, 3, Color::Cyan);
        let buf = blit(&f, 2, 3, 3); // deliberately too-small buffer
        assert_eq!(buf.len(), 3 * 3); // reached here = no panic
    }

    #[test]
    fn blit_clips_a_lit_pixel_past_the_x_edge() {
        // A lit pixel in a cell column that, scaled, would write past px_w must
        // clip rather than panic (mirrors the y-axis clip test above).
        let mut f = Frame::new(3, 1); // canvas 3px wide
        f.set_pixel(2, 0, Color::Cyan); // rightmost column lit
        let px_w = 5;
        let px_h = 4;
        let buf = blit(&f, 2, px_w, px_h); // scale 2: col 2 starts at x=4, spills past 5
        assert_eq!(buf.len(), px_w * px_h); // reached here = no panic
    }
}
