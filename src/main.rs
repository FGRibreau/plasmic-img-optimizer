use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use log::info;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

// Re-export from lib.rs
use img_optimizer::{
    cache::ImageCache, direct_image_handler, health_check, list_errors, optimize_image_handler,
    AppState,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let cache_dir = PathBuf::from("cache");

    // Ensure cache directory exists
    fs::create_dir_all(&cache_dir).await?;

    let app_state = AppState {
        cache: Arc::new(RwLock::new(ImageCache::new(cache_dir))),
        client: reqwest::Client::new(),
    };

    info!("Starting image optimizer service on port {port}");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allowed_methods(vec!["GET", "OPTIONS"])
                    .allowed_headers(vec!["Origin", "X-Requested-With", "Content-Type", "Accept"])
                    .max_age(3600),
            )
            .route("/health", web::get().to(health_check))
            .route("/errors", web::get().to(list_errors))
            .route(
                "/img-optimizer/v1/img",
                web::get().to(optimize_image_handler),
            )
            .route(
                "/img-optimizer/v1/img/{image_id}",
                web::get().to(direct_image_handler),
            )
    })
    .bind(format!("0.0.0.0:{port}"))?
    .run()
    .await
}
