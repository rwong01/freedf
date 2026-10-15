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
    let image = trim_transparent(image)?;
    let aspect = image.height() as f32 / image.width() as f32;
    let mut bytes = std::io::Cursor::new(vec![]);
    image.write_to(&mut bytes, image::ImageFormat::Png)?;
    Ok((bytes.into_inner(), aspect))
}

/// Remove transparent padding, preserving a small border and every visible pixel.
fn trim_transparent(image: RgbaImage) -> Result<RgbaImage> {
    let (mut left, mut top, mut right, mut bottom) = (image.width(), image.height(), 0, 0);
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] > 0 {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x + 1);
            bottom = bottom.max(y + 1);
        }
    }
    ensure!(
        right > left && bottom > top,
        "The image has no visible signature"
    );
    let (left, top) = (left.saturating_sub(6), top.saturating_sub(6));
    let (right, bottom) = (
        (right + 6).min(image.width()),
        (bottom + 6).min(image.height()),
    );
    Ok(image::imageops::crop_imm(&image, left, top, right - left, bottom - top).to_image())
}

pub fn imported(bytes: &[u8]) -> Result<(Vec<u8>, f32)> {
    ensure!(
        bytes.len() <= 20 * 1024 * 1024,
        "Signature image exceeds 20 MiB"
    );
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(80 * 1024 * 1024);
    reader.limits(limits);
    let image = trim_transparent(reader.decode()?.to_rgba8())?;
    let aspect = image.height() as f32 / image.width() as f32;
    let mut output = std::io::Cursor::new(vec![]);
    image.write_to(&mut output, image::ImageFormat::Png)?;
    Ok((output.into_inner(), aspect))
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

    #[test]
    fn imported_padding_is_trimmed_without_distorting_or_erasing_pixels() {
        let mut image = RgbaImage::new(100, 80);
        for y in 30..40 {
            for x in 20..60 {
                image.put_pixel(x, y, Rgba([20, 46, 102, 255]));
            }
        }
        let mut source = std::io::Cursor::new(vec![]);
        image
            .write_to(&mut source, image::ImageFormat::Png)
            .unwrap();
        let (bytes, aspect) = imported(source.get_ref()).unwrap();
        let result = image::load_from_memory(&bytes).unwrap().to_rgba8();
        assert_eq!((result.width(), result.height()), (52, 22));
        assert_eq!(result.pixels().filter(|p| p[3] > 0).count(), 400);
        assert!((aspect - 22. / 52.).abs() < 0.0001);
        assert!(trim_transparent(RgbaImage::new(2, 2)).is_err());
        let opaque = RgbaImage::from_pixel(20, 40, Rgba([255, 255, 255, 255]));
        assert_eq!(trim_transparent(opaque).unwrap().dimensions(), (20, 40));
    }
}
