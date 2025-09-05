use std::io::Cursor;

use anyhow::{Context, Result, bail};
use bytes::Bytes;
use png::{BitDepth, ColorType, Decoder, Encoder};

use crate::transform::shared::TransformProcess;

// --- Helper functions ---

#[inline]
fn sign_extend_24(d: i32) -> i32 {
    (d << 8) >> 8
}

#[inline]
fn gsi_rgb_to_cm(r: u8, g: u8, b: u8) -> i32 {
    let d = ((r as i32) << 16) | ((g as i32) << 8) | (b as i32);
    let mut cm = sign_extend_24(d);
    if cm == -8_388_608 {
        cm = 0; // sentinel -> 0 cm
    }
    cm
}

#[inline]
fn round_decimeters(cm: i32) -> i32 {
    // JS Math.round semantics for cm/10: floor(x + 0.5)
    let a = cm + 5;
    if a >= 0 { a / 10 } else { -(((-a) + 9) / 10) }
}

#[inline]
fn terrain_value_from_cm(cm: i32) -> i32 {
    100_000 + round_decimeters(cm)
}

/// Convert a GSI DEM-packed RGB triplet to Terrain-RGB (R,G,B)
pub(crate) fn dem_rgb_to_terrain_rgb(r: u8, g: u8, b: u8) -> [u8; 3] {
    let cm = gsi_rgb_to_cm(r, g, b);
    let v = terrain_value_from_cm(cm);
    [
        ((v >> 16) & 0xFF) as u8,
        ((v >> 8) & 0xFF) as u8,
        (v & 0xFF) as u8,
    ]
}

#[inline]
fn transform_palette_rgb_in_place(palette: &mut [u8]) {
    for i in (0..palette.len()).step_by(3) {
        let rgb = dem_rgb_to_terrain_rgb(palette[i], palette[i + 1], palette[i + 2]);
        palette[i] = rgb[0];
        palette[i + 1] = rgb[1];
        palette[i + 2] = rgb[2];
    }
}

#[inline]
fn expand_rgb8_to_rgba8(src: &[u8]) -> Vec<u8> {
    let mut out = vec![255u8; (src.len() / 3) * 4];
    for (dst, s) in out.chunks_exact_mut(4).zip(src.chunks_exact(3)) {
        dst[0] = s[0];
        dst[1] = s[1];
        dst[2] = s[2];
        // dst[3] already 255
    }
    out
}

#[inline]
fn transform_rgba8_in_place(data: &mut [u8]) {
    for px in data.chunks_exact_mut(4) {
        let rgb = dem_rgb_to_terrain_rgb(px[0], px[1], px[2]);
        px[0] = rgb[0];
        px[1] = rgb[1];
        px[2] = rgb[2];
        px[3] = 255;
    }
}

#[derive(Debug, Clone)]
pub struct GsiDemPngToTerrainRgbPng;

impl TransformProcess for GsiDemPngToTerrainRgbPng {
    fn new() -> Self {
        Self
    }

    fn transform(&self, input: &[u8]) -> Result<Bytes> {
        // Decode
        let cursor = Cursor::new(input);
        let decoder = Decoder::new(cursor);
        let mut reader = decoder.read_info().context("read png info")?;
        let mut buf = vec![0u8; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).context("decode frame")?;
        let (w, h) = (info.width, info.height);

        // Fast path: Indexed color — transform palette entries instead of pixels
        if info.color_type == ColorType::Indexed {
            let src = &buf[..info.buffer_size()];
            let info_ref = reader.info();
            let palette = info_ref
                .palette
                .as_ref()
                .context("indexed PNG missing palette")?;
            let entries = palette.len() / 3;
            if entries == 0 {
                bail!("indexed PNG has empty palette");
            }

            // Transform each palette color using the same Terrain-RGB conversion
            let mut new_palette = palette.to_vec();
            transform_palette_rgb_in_place(&mut new_palette);

            // Encode as indexed with transformed palette; keep original bit depth and tRNS if present
            let mut out = Vec::with_capacity(src.len() + 1024);
            {
                let mut enc = Encoder::new(&mut out, w, h);
                enc.set_color(ColorType::Indexed);
                enc.set_depth(info.bit_depth);
                enc.set_compression(png::Compression::Fast);
                enc.set_palette(new_palette);
                if let Some(trns) = &info_ref.trns {
                    enc.set_trns(trns.clone());
                }
                let mut writer = enc.write_header().context("write header (indexed)")?;
                writer
                    .write_image_data(src)
                    .context("encode indexed image data")?;
            }
            return Ok(out.into());
        }

        // Normalize to RGBA8
        let mut data = match (info.color_type, info.bit_depth) {
            (ColorType::Rgba, BitDepth::Eight) => {
                buf.truncate(info.buffer_size());
                buf
            }
            (ColorType::Rgb, BitDepth::Eight) => {
                let src = &buf[..info.buffer_size()];
                expand_rgb8_to_rgba8(src)
            }
            _ => bail!(
                "Only 8-bit RGB/RGBA PNG supported, got: {:?} {:?}",
                info.color_type,
                info.bit_depth
            ),
        };

        // Transform in-place, serial
        transform_rgba8_in_place(&mut data);

        // Encode with low-latency settings
        let mut out = Vec::with_capacity(data.len() + 1024);
        {
            let mut enc = Encoder::new(&mut out, w, h);
            enc.set_color(ColorType::Rgba);
            enc.set_depth(BitDepth::Eight);
            enc.set_compression(png::Compression::Fast);
            let mut writer = enc.write_header().context("write header")?;
            writer
                .write_image_data(&data)
                .context("encode image data")?;
        }
        Ok(out.into())
    }
}
