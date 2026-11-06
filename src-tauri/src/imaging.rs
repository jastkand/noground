use std::{io::BufWriter, path::Path};

use anyhow::{Context, Result};
use image::{
    codecs::{jpeg::JpegEncoder, png::PngEncoder},
    imageops::FilterType,
    metadata::Orientation,
    DynamicImage, GrayImage, ImageBuffer, ImageDecoder, ImageReader, Luma, RgbImage, Rgba,
    RgbaImage,
};

use crate::{error::AppError, models::ModelSpec, settings::OutputFormat};

/// Decode with EXIF orientation applied (rembg applies it before anything else).
pub fn load_oriented(path: &Path) -> Result<DynamicImage> {
    let reader = ImageReader::open(path)
        .with_context(|| format!("open {}", path.display()))?
        .with_guessed_format()?;
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder
        .orientation()
        .unwrap_or(Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder)?;
    img.apply_orientation(orientation);
    Ok(img)
}

/// rembg base.py: resize LANCZOS to model input, divide by the image's own
/// max pixel (NOT 255), then per-channel (x - mean) / std, CHW layout.
/// Returns the flat [1, 3, s, s] data.
pub fn preprocess(img: &DynamicImage, spec: &ModelSpec) -> Vec<f32> {
    let s = spec.input_size as usize;
    let rgb = img.to_rgb8();
    let resized = image::imageops::resize(&rgb, s as u32, s as u32, FilterType::Lanczos3);

    let max = resized
        .pixels()
        .map(|p| p[0].max(p[1]).max(p[2]))
        .max()
        .unwrap_or(0) as f32;
    let scale = 1.0 / max.max(1e-6);

    let mut data = Vec::with_capacity(3 * s * s);
    for c in 0..3 {
        for y in 0..s {
            for x in 0..s {
                let px = resized.get_pixel(x as u32, y as u32)[c] as f32 * scale;
                data.push((px - spec.mean[c]) / spec.std[c]);
            }
        }
    }
    data
}

/// rembg postprocessing: optional sigmoid, min-max normalize, optional clip,
/// ×255 u8, LANCZOS resize back to the original size. `plane_len` is
/// input_size² — the mask is always the first plane of the output.
pub fn postprocess(
    data: &[f32],
    plane_len: usize,
    orig_w: u32,
    orig_h: u32,
    spec: &ModelSpec,
) -> Result<GrayImage> {
    if data.len() < plane_len {
        return Err(AppError::Other(format!(
            "model output too small: {} < {plane_len}",
            data.len()
        ))
        .into());
    }
    let side = (plane_len as f64).sqrt().round() as u32;
    let plane = &data[..plane_len];

    let mut mi = f32::MAX;
    let mut ma = f32::MIN;
    let mut vals = Vec::with_capacity(plane_len);
    for &x in plane {
        let v = if spec.sigmoid {
            1.0 / (1.0 + (-x).exp())
        } else {
            x
        };
        mi = mi.min(v);
        ma = ma.max(v);
        vals.push(v);
    }
    let range = ma - mi;

    let mut mask = ImageBuffer::<Luma<u8>, Vec<u8>>::new(side, side);
    for (i, &v) in vals.iter().enumerate() {
        let mut n = if range.abs() < 1e-12 {
            0.0
        } else {
            (v - mi) / range
        };
        if spec.clip {
            n = n.clamp(0.0, 1.0);
        }
        let x = (i as u32) % side;
        let y = (i as u32) / side;
        mask.put_pixel(x, y, Luma([(n * 255.0) as u8]));
    }

    Ok(image::imageops::resize(&mask, orig_w, orig_h, FilterType::Lanczos3))
}

/// rembg naive_cutout: alpha = mask, rgb premultiplied by mask (soft edges
/// blend toward black — this matches the Python reference byte-for-byte).
pub fn compose(img: &DynamicImage, mask: &GrayImage) -> RgbaImage {
    let rgb = img.to_rgb8();
    let (w, h) = (rgb.width(), rgb.height());
    let mut out = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let a = mask.get_pixel(x, y)[0] as u16;
            let p = rgb.get_pixel(x, y);
            let scaled = |c: u8| ((c as u16 * a + 127) / 255) as u8;
            out.put_pixel(x, y, Rgba([scaled(p[0]), scaled(p[1]), scaled(p[2]), a as u8]));
        }
    }
    out
}

pub fn encode_and_write(img: &RgbaImage, out_path: &Path, format: OutputFormat) -> Result<()> {
    let part = out_path.with_extension("part");
    if let Some(dir) = part.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = BufWriter::new(std::fs::File::create(&part)?);
    match format {
        OutputFormat::Png => {
            DynamicImage::ImageRgba8(img.clone()).write_with_encoder(PngEncoder::new(file))?
        }
        OutputFormat::Jpeg => {
            let flattened = flatten_white(img);
            DynamicImage::ImageRgb8(flattened)
                .write_with_encoder(JpegEncoder::new_with_quality(file, 92))?
        }
    }
    std::fs::rename(&part, out_path)?;
    Ok(())
}

fn flatten_white(img: &RgbaImage) -> RgbImage {
    let (w, h) = (img.width(), img.height());
    let mut out = RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            let a = p[3] as u32;
            let over_white = |fg: u8| {
                ((fg as u32 * a + 255 * (255 - a) + 127) / 255) as u8
            };
            out.put_pixel(x, y, image::Rgb([over_white(p[0]), over_white(p[1]), over_white(p[2])]));
        }
    }
    out
}
