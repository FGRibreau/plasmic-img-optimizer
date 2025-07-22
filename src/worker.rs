use crate::{cache::ImageCache, health_check, list_errors, optimize_image_handler, AppState};
use std::sync::Arc;
use tokio::sync::RwLock;
use worker::*;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    // Set up CORS headers
    let cors_headers = Headers::new()
        .set("Access-Control-Allow-Origin", "*")?
        .set("Access-Control-Allow-Methods", "GET, OPTIONS")?
        .set(
            "Access-Control-Allow-Headers",
            "Origin, X-Requested-With, Content-Type, Accept",
        )?
        .set("Access-Control-Max-Age", "3600")?;

    // Handle CORS preflight
    if req.method() == Method::Options {
        return Response::empty().with_headers(cors_headers);
    }

    // Initialize app state
    let kv = env.kv("IMAGE_CACHE")?;
    let app_state = AppState {
        cache: Arc::new(RwLock::new(ImageCache::new_kv(kv))),
        client: reqwest::Client::new(),
    };

    // Route matching
    let url = req.url()?;
    let path = url.path();

    let response = match path {
        "/health" => handle_health(req).await,
        "/errors" => handle_errors(req).await,
        path if path.starts_with("/img-optimizer/v1/img") => {
            handle_optimize_image(req, app_state).await
        }
        _ => Response::error("Not Found", 404),
    };

    // Add CORS headers to response
    response.map(|mut resp| {
        for (key, value) in cors_headers.entries() {
            resp.headers_mut().set(&key, &value)?;
        }
        Ok(resp)
    })?
}

async fn handle_health(_req: Request) -> Result<Response> {
    Response::from_json(&serde_json::json!({
        "status": "ok",
        "service": "img-optimizer",
        "runtime": "cloudflare-workers"
    }))
}

async fn handle_errors(_req: Request) -> Result<Response> {
    let errors = crate::error::AppError::list_all_errors();
    Response::from_json(&serde_json::json!({
        "errors": errors,
        "total": errors.len()
    }))
}

async fn handle_optimize_image(req: Request, state: AppState) -> Result<Response> {
    // Parse query parameters
    let url = req.url()?;
    let query_params = url.query_pairs();

    let mut src = None;
    let mut width = None;
    let mut quality = None;
    let mut format = None;

    for (key, value) in query_params {
        match key.as_ref() {
            "src" => src = Some(value.to_string()),
            "w" => width = value.parse::<u32>().ok(),
            "q" => quality = value.parse::<u8>().ok(),
            "f" => format = Some(value.to_string()),
            _ => {}
        }
    }

    // Convert to actix-web compatible request
    let params = crate::ImageParams {
        src,
        w: width,
        q: quality,
        f: format,
    };

    // Call the existing handler logic
    match crate::process_image_request(params, &state).await {
        Ok((data, content_type)) => Response::from_bytes(data).map(|mut resp| {
            resp.headers_mut().set("Content-Type", &content_type)?;
            Ok(resp)
        })?,
        Err(err) => {
            let error_response = err.to_response();
            Response::from_json(&error_response).map(|mut resp| {
                resp = resp.with_status(err.status_code().as_u16());
                Ok(resp)
            })?
        }
    }
}
