use anyhow::{Result, ensure};
use image::{Rgba, RgbaImage};
pub const CAVEAT: &[u8] = include_bytes!("../../../assets/fonts/Caveat.ttf");
pub const DANCING: &[u8] = include_bytes!("../../../assets/fonts/DancingScript.ttf");

/// A transparent, high-resolution appearance, embedded as an image XObject;
/// document pages and existing vector content remain intact.
pub fn typed(name: &str, style: usize) -> Result<(Vec<u8>, f32)> {
    ensure!(
        !name.trim().is_empty() && name.chars().count() <= 80,
        "Enter a name of up to 80 characters"
    );
    let font = fontdue::Font::from_bytes(
        if style == 0 { CAVEAT } else { DANCING },
        fontdue::FontSettings::default(),
    )
    .map_err(|e| anyhow::anyhow!(e))?;
    ensure!(
        name.chars()
            .all(|c| c == ' ' || font.lookup_glyph_index(c) != 0),
        "This font does not contain every character in your name. Try another style or draw your signature."
    );
    let mut glyphs = vec![];
    let mut x = 16f32;
    let mut previous = None;
    for c in name.chars() {
        if let Some(p) = previous {
            x += font.horizontal_kern(p, c, 96.).unwrap_or(0.);
        }
        let (metrics, bitmap) = font.rasterize(c, 96.);
        let advance = metrics.advance_width;
        glyphs.push((x, metrics, bitmap));
        x += advance;
        previous = Some(c);
    }
    let w = (x.ceil() as u32 + 32).min(4096);
    let h = 180;
    ensure!(x < 4064., "Signature is too wide; shorten the name");
    let mut image = RgbaImage::new(w, h);
    for (x, m, bitmap) in glyphs {
        for y in 0..m.height {
            for column in 0..m.width {
                let px = x as i32 + m.xmin + column as i32;
                let py = 128 - m.ymin - m.height as i32 + y as i32;
                if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                    let alpha = bitmap[y * m.width + column];
                    let old = image.get_pixel(px as u32, py as u32)[3];
                    image.put_pixel(px as u32, py as u32, Rgba([20, 46, 102, alpha.max(old)]));
                }
            }
        }
    }
    let mut bytes = std::io::Cursor::new(vec![]);
    image.write_to(&mut bytes, image::ImageFormat::Png)?;
    Ok((bytes.into_inner(), h as f32 / w as f32))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_signature_is_transparent_and_distinct_styles() {
        let (a, _) = typed("Alice Example", 0).unwrap();
        let (b, _) = typed("Alice Example", 1).unwrap();
        assert_ne!(a, b);
        let image = image::load_from_memory(&a).unwrap().to_rgba8();
        assert!(image.pixels().any(|p| p[3] == 0));
        assert!(image.pixels().any(|p| p[3] > 200));
        assert!(typed("\u{1f984}", 0).is_err());
    }
}
