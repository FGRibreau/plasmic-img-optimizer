use crate::error::{AppError, AppResult};
use image::{DynamicImage, ImageFormat, ImageReader};
use std::io::Cursor;
use webp::Encoder;

pub struct ImageProcessor;

impl ImageProcessor {
    pub async fn process(
        image_data: Vec<u8>,
        width: Option<u32>,
        quality: u8,
        format: Option<&str>,
    ) -> AppResult<Vec<u8>> {
        // Load image
        let reader = ImageReader::new(Cursor::new(image_data))
            .with_guessed_format()
            .map_err(|e| AppError::ImageProcessingFailed {
                reason: format!("Failed to read image: {e}"),
            })?;

        let mut img = reader
            .decode()
            .map_err(|e| AppError::ImageProcessingFailed {
                reason: format!("Failed to decode image: {e}"),
            })?;

        // Resize if needed
        if let Some(target_width) = width {
            let (current_width, current_height) = (img.width(), img.height());
            if target_width < current_width {
                let target_height =
                    (target_width as f32 * current_height as f32 / current_width as f32) as u32;
                img = img.resize_exact(
                    target_width,
                    target_height,
                    image::imageops::FilterType::Lanczos3,
                );
            }
        }

        // Convert format and encode
        let output_format = match format {
            Some(f) => match f {
                "jpeg" | "jpg" => OutputFormat::Jpeg,
                "png" => OutputFormat::Png,
                "webp" => OutputFormat::WebP,
                _ => {
                    return Err(AppError::InvalidImageFormat {
                        format: f.to_string(),
                    })
                }
            },
            None => detect_format(&img),
        };

        encode_image(&img, output_format, quality)
    }
}

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Jpeg,
    Png,
    WebP,
}

fn detect_format(img: &DynamicImage) -> OutputFormat {
    // Default to JPEG for photos, PNG for images with transparency
    if img.color().has_alpha() {
        OutputFormat::Png
    } else {
        OutputFormat::Jpeg
    }
}

fn encode_image(img: &DynamicImage, format: OutputFormat, quality: u8) -> AppResult<Vec<u8>> {
    let mut output = Vec::new();
    let mut cursor = Cursor::new(&mut output);

    match format {
        OutputFormat::Jpeg => {
            // JPEG doesn't support transparency, convert to RGB
            let rgb_img = img.to_rgb8();
            rgb_img
                .write_to(&mut cursor, ImageFormat::Jpeg)
                .map_err(|e| AppError::ImageProcessingFailed {
                    reason: format!("Failed to encode JPEG: {e}"),
                })?;
        }
        OutputFormat::Png => {
            img.write_to(&mut cursor, ImageFormat::Png).map_err(|e| {
                AppError::ImageProcessingFailed {
                    reason: format!("Failed to encode PNG: {e}"),
                }
            })?;
        }
        OutputFormat::WebP => {
            let rgba_img = img.to_rgba8();
            let (width, height) = rgba_img.dimensions();
            let encoder = Encoder::from_rgba(&rgba_img, width, height);
            let webp_data = encoder.encode(quality as f32);
            output.extend_from_slice(&webp_data);
        }
    }

    Ok(output)
}
