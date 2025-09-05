use std::io::Cursor;

use anyhow::{Context, Result, bail};
use bytes::Bytes;
use png::{BitDepth, ColorType, Decoder, Encoder};

use crate::transform::shared::TransformProcess;

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

        // Normalize to RGBA8
        let mut data = match (info.color_type, info.bit_depth) {
            (ColorType::Rgba, BitDepth::Eight) => {
                buf.truncate(info.buffer_size());
                buf
            }
            (ColorType::Rgb, BitDepth::Eight) => {
                let src = &buf[..info.buffer_size()];
                let mut out = vec![255u8; (w as usize) * (h as usize) * 4];
                for (dst, s) in out.chunks_exact_mut(4).zip(src.chunks_exact(3)) {
                    dst[0] = s[0];
                    dst[1] = s[1];
                    dst[2] = s[2];
                    // dst[3] already 255
                }
                out
            }
            _ => bail!("Only 8-bit RGB/RGBA PNG supported"),
        };

        // Transform in-place, serial
        for px in data.chunks_exact_mut(4) {
            let d = ((px[0] as i32) << 16) | ((px[1] as i32) << 8) | (px[2] as i32);
            let mut cm = (d << 8) >> 8; // sign-extend 24-bit to i32
            if cm == -8_388_608 {
                cm = 0; // sentinel -> 0 cm
            }
            // round(cm / 10.0) with JS Math.round semantics: floor(x + 0.5)
            let a = cm + 5;
            let rounded_dm = if a >= 0 { a / 10 } else { -(((-a) + 9) / 10) };
            let v = 100_000 + rounded_dm; // Terrain-RGB value

            px[0] = ((v >> 16) & 0xFF) as u8;
            px[1] = ((v >> 8) & 0xFF) as u8;
            px[2] = (v & 0xFF) as u8;
            px[3] = 255;
        }

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
