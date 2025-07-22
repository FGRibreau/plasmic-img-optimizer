pub mod cache;
pub mod error;
pub mod image_processor;

#[cfg(feature = "worker")]
pub mod worker;

use actix_web::{web, HttpResponse, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;

use cache::ImageCache;
use error::{AppError, AppResult};
use image_processor::ImageProcessor;

pub const MAX_WIDTH: u32 = 3840;
pub const DEFAULT_QUALITY: u8 = 75;
pub const MAX_IMAGE_SIZE: usize = 50 * 1024 * 1024; // 50MB

pub static IMAGE_ID_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([a-f0-9]{32})\.(\w+)$").expect("Failed to compile regex"));

#[derive(Debug, Deserialize)]
pub struct ImageParams {
    pub src: Option<String>,
    pub w: Option<u32>,
    pub q: Option<u8>,
    pub f: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub cache: Arc<RwLock<ImageCache>>,
    pub client: reqwest::Client,
}

pub async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "img-optimizer"
    })))
}

pub async fn list_errors() -> Result<HttpResponse> {
    let errors = AppError::list_all_errors();
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "errors": errors,
        "total": errors.len()
    })))
}

pub async fn process_image_request(
    params: ImageParams,
    state: &AppState,
) -> AppResult<(Vec<u8>, String)> {
    let src = params
        .src
        .as_ref()
        .ok_or_else(|| AppError::MissingRequiredParameter {
            param: "src".to_string(),
        })?;

    // Validate URL
    let url = Url::parse(src).map_err(|_| AppError::InvalidImageUrl)?;

    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(AppError::InvalidImageUrl);
    }

    // SVG files are not processed in the core logic
    if src.to_lowercase().ends_with(".svg") {
        return Err(AppError::InvalidImageFormat {
            format: "svg".to_string(),
        });
    }

    // Parse parameters
    let width = match params.w {
        Some(w) if w == 0 || w > MAX_WIDTH => return Err(AppError::InvalidWidth { width: w }),
        Some(w) => Some(w),
        None => None,
    };
    let quality = match params.q {
        Some(q) if q == 0 || q > 100 => return Err(AppError::InvalidQuality { quality: q }),
        Some(q) => q,
        None => DEFAULT_QUALITY,
    };
    let format = params.f.as_deref();

    // Generate cache key
    let cache_key = generate_cache_key(src, width, quality, format);

    // Check cache
    {
        let cache = state.cache.read().await;
        if let Some(cached_data) = cache.get(&cache_key).await {
            let content_type = guess_content_type(&cached_data);
            return Ok((cached_data, content_type.to_string()));
        }
    }

    // Fetch and process image
    let image_data = fetch_image(&state.client, src).await?;
    let processed_data = ImageProcessor::process(image_data, width, quality, format).await?;

    // Cache the result
    {
        let mut cache = state.cache.write().await;
        cache.put(cache_key, processed_data.clone()).await;
    }

    let content_type = guess_content_type(&processed_data);
    Ok((processed_data, content_type.to_string()))
}

pub async fn optimize_image_handler(
    query: web::Query<ImageParams>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    // Handle SVG redirect specially for actix-web
    if let Some(src) = &query.src {
        if src.to_lowercase().ends_with(".svg") {
            return Ok(HttpResponse::Found()
                .append_header(("Location", src.as_str()))
                .finish());
        }
    }

    match process_image_request(query.into_inner(), &state).await {
        Ok((data, content_type)) => Ok(HttpResponse::Ok().content_type(content_type).body(data)),
        Err(err) => Err(err.into()),
    }
}

pub async fn direct_image_handler(image_id: web::Path<String>) -> Result<HttpResponse> {
    // Validate image ID format
    if !IMAGE_ID_REGEX.is_match(&image_id) {
        return Err(AppError::InvalidImageUrl.into());
    }

    // In a real implementation, this would fetch from internal storage
    Err(AppError::ImageFetchFailed {
        url: image_id.to_string(),
    }
    .into())
}

pub async fn fetch_image(client: &reqwest::Client, url: &str) -> AppResult<Vec<u8>> {
    use futures_util::StreamExt;

    let response = client
        .get(url)
        .header("User-Agent", "Plasmic-Image-Optimizer/1.0")
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|_| AppError::ImageFetchFailed {
            url: url.to_string(),
        })?;

    if !response.status().is_success() {
        return Err(AppError::ImageFetchFailed {
            url: url.to_string(),
        });
    }

    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| AppError::ImageFetchFailed {
            url: url.to_string(),
        })?;
        bytes.extend_from_slice(&chunk);

        if bytes.len() > MAX_IMAGE_SIZE {
            return Err(AppError::ImageTooLarge);
        }
    }

    Ok(bytes)
}

pub fn generate_cache_key(
    src: &str,
    width: Option<u32>,
    quality: u8,
    format: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(src.as_bytes());
    if let Some(w) = width {
        hasher.update(w.to_string().as_bytes());
    }
    hasher.update(quality.to_string().as_bytes());
    if let Some(f) = format {
        hasher.update(f.as_bytes());
    }
    hex::encode(hasher.finalize())
}

pub fn guess_content_type(data: &[u8]) -> &'static str {
    if data.len() < 12 {
        return "application/octet-stream";
    }

    match &data[..12] {
        [0xFF, 0xD8, 0xFF, ..] => "image/jpeg",
        [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, ..] => "image/png",
        [0x52, 0x49, 0x46, 0x46, _, _, _, _, 0x57, 0x45, 0x42, 0x50] => "image/webp",
        _ => "application/octet-stream",
    }
}
