// Image conversion using the `image` crate (no external tools needed).
// Targets: jpg, png, webp, bmp, gif, tiff, ico, tga.
// Explicit output size wins; otherwise PNG/JPG/WebP/BMP/TGA/GIF largest side
// is capped at 4000 px. ICO is saved as a single 256 px frame (see MECHANICS.md).

use image::imageops::FilterType;
use image::ImageEncoder;
use std::path::Path;

pub const MAX_SIDE: u32 = 4000;
pub const MAX_OUTPUT_SIDE: u32 = 12000;
pub const ICO_SIZE: u32 = 256;
pub const JPG_QUALITY: u8 = 90;

pub struct ImageResult {
    pub ok: bool,
    pub note: String,
}

pub fn convert(src: &Path, target: &str, dst: &Path, resize: Option<(u32, u32)>) -> ImageResult {
    let opened = image::ImageReader::open(src).and_then(|r| r.with_guessed_format());
    let img = match opened {
        Ok(r) => match r.decode() {
            Ok(i) => i,
            Err(e) => {
                return ImageResult {
                    ok: false,
                    note: format!("无法解码图片: {e}"),
                }
            }
        },
        Err(e) => {
            return ImageResult {
                ok: false,
                note: format!("无法打开文件: {e}"),
            }
        }
    };

    // Explicit output size takes precedence over the default downscale cap.
    // ICO stays a single small frame, so a custom size is ignored there.
    let img = if target == "ico" {
        img
    } else if let Some((w, h)) = resize {
        if w > MAX_OUTPUT_SIDE || h > MAX_OUTPUT_SIDE {
            return ImageResult {
                ok: false,
                note: format!("输出尺寸过大，单边最大 {MAX_OUTPUT_SIDE} px"),
            };
        }
        match (w, h) {
            (w, h) if w > 0 && h > 0 => img.resize_exact(w, h, FilterType::Lanczos3),
            (w, 0) => img.resize(w, u32::MAX, FilterType::Lanczos3),
            (0, h) => img.resize(u32::MAX, h, FilterType::Lanczos3),
            _ => img,
        }
    } else {
        let (w, h) = (img.width(), img.height());
        let largest = w.max(h);
        if largest > MAX_SIDE {
            let scale = MAX_SIDE as f32 / largest as f32;
            let nw = ((w as f32) * scale).round() as u32;
            let nh = ((h as f32) * scale).round() as u32;
            img.resize(nw, nh, FilterType::Lanczos3)
        } else {
            img
        }
    };
    let out_size = (img.width(), img.height());

    let result: Result<(), String> = match target {
        "jpg" | "jpeg" => {
            let rgb = image::DynamicImage::ImageRgb8(img.to_rgb8());
            let mut out = Vec::new();
            let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPG_QUALITY);
            match rgb.write_with_encoder(enc) {
                Ok(_) => std::fs::write(dst, &out).map_err(|e| e.to_string()),
                Err(e) => Err(e.to_string()),
            }
        }
        "png" => img
            .save_with_format(dst, image::ImageFormat::Png)
            .map_err(|e| e.to_string()),
        "webp" => {
            let rgb = image::DynamicImage::ImageRgb8(img.to_rgb8());
            let mut out = Vec::new();
            let enc = image::codecs::webp::WebPEncoder::new_lossless(&mut out);
            match rgb.write_with_encoder(enc) {
                Ok(_) => std::fs::write(dst, &out).map_err(|e| e.to_string()),
                Err(e) => Err(e.to_string()),
            }
        }
        "bmp" => img
            .save_with_format(dst, image::ImageFormat::Bmp)
            .map_err(|e| e.to_string()),
        "gif" => img
            .save_with_format(dst, image::ImageFormat::Gif)
            .map_err(|e| e.to_string()),
        "tiff" => img
            .save_with_format(dst, image::ImageFormat::Tiff)
            .map_err(|e| e.to_string()),
        "tga" => img
            .save_with_format(dst, image::ImageFormat::Tga)
            .map_err(|e| e.to_string()),
        "ico" => {
            let small = img.resize(ICO_SIZE, ICO_SIZE, FilterType::Lanczos3);
            let rgba = small.to_rgba8();
            let mut out = Vec::new();
            let enc = image::codecs::ico::IcoEncoder::new(&mut out);
            match enc.write_image(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                image::ExtendedColorType::Rgba8,
            ) {
                Ok(_) => std::fs::write(dst, &out).map_err(|e| e.to_string()),
                Err(e) => Err(e.to_string()),
            }
        }
        other => Err(format!("未知图片格式: {other}")),
    };

    match result {
        Ok(_) => ImageResult {
            ok: true,
            note: if target == "ico" {
                "完成 · ICO 单帧".into()
            } else {
                format!("完成 · {}×{}", out_size.0, out_size.1)
            },
        },
        Err(e) => ImageResult { ok: false, note: e },
    }
}
