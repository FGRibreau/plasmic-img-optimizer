use actix_web::{test, web, App};
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use img_optimizer::{
    cache::ImageCache, direct_image_handler, health_check, optimize_image_handler, AppState,
};

// Create a small test image - using a valid 1x1 PNG
fn create_test_png() -> Vec<u8> {
    // This is a valid 1x1 transparent PNG image
    let base64_png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";
    use base64::{engine::general_purpose, Engine as _};
    general_purpose::STANDARD.decode(base64_png).unwrap()
}

fn create_app_state(cache_dir: PathBuf) -> AppState {
    AppState {
        cache: Arc::new(RwLock::new(ImageCache::new(cache_dir))),
        client: reqwest::Client::new(),
    }
}

#[actix_rt::test]
async fn test_health_check() {
    let mut app =
        test::init_service(App::new().route("/health", web::get().to(health_check))).await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&mut app, req).await;

    assert!(resp.status().is_success());
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "img-optimizer");
}

#[actix_rt::test]
async fn test_missing_src_parameter() {
    let temp_dir = TempDir::new().unwrap();
    let app_state = create_app_state(temp_dir.path().to_path_buf());

    let mut app = test::init_service(App::new().app_data(web::Data::new(app_state)).route(
        "/img-optimizer/v1/img",
        web::get().to(optimize_image_handler),
    ))
    .await;

    let req = test::TestRequest::get()
        .uri("/img-optimizer/v1/img")
        .to_request();
    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["title"], "Bad Request");
    assert_eq!(body["errorCode"], "VAL_003");
}

#[actix_rt::test]
async fn test_invalid_url() {
    let temp_dir = TempDir::new().unwrap();
    let app_state = create_app_state(temp_dir.path().to_path_buf());

    let mut app = test::init_service(App::new().app_data(web::Data::new(app_state)).route(
        "/img-optimizer/v1/img",
        web::get().to(optimize_image_handler),
    ))
    .await;

    let req = test::TestRequest::get()
        .uri("/img-optimizer/v1/img?src=not-a-url")
        .to_request();
    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["title"], "Bad Request");
    assert_eq!(body["errorCode"], "IMG_001");
}

#[actix_rt::test]
async fn test_svg_redirect() {
    let temp_dir = TempDir::new().unwrap();
    let app_state = create_app_state(temp_dir.path().to_path_buf());

    let mut app = test::init_service(App::new().app_data(web::Data::new(app_state)).route(
        "/img-optimizer/v1/img",
        web::get().to(optimize_image_handler),
    ))
    .await;

    let svg_url = "https://example.com/test.svg";
    let req = test::TestRequest::get()
        .uri(&format!("/img-optimizer/v1/img?src={}", svg_url))
        .to_request();
    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), 302);
    assert_eq!(
        resp.headers().get("location").unwrap().to_str().unwrap(),
        svg_url
    );
}

#[actix_rt::test]
async fn test_image_optimization_with_mock_server() {
    let mock_server = MockServer::start().await;
    let test_image = create_test_png();

    Mock::given(method("GET"))
        .and(path("/test-image.png"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(test_image.clone())
                .insert_header("content-type", "image/png"),
        )
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().unwrap();
    let app_state = create_app_state(temp_dir.path().to_path_buf());

    let mut app = test::init_service(App::new().app_data(web::Data::new(app_state)).route(
        "/img-optimizer/v1/img",
        web::get().to(optimize_image_handler),
    ))
    .await;

    let image_url = format!("{}/test-image.png", &mock_server.uri());
    let req = test::TestRequest::get()
        .uri(&format!("/img-optimizer/v1/img?src={}", &image_url))
        .to_request();
    let resp = test::call_service(&mut app, req).await;

    assert!(resp.status().is_success());
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "image/png"
    );
}

#[actix_rt::test]
async fn test_image_format_conversion() {
    let mock_server = MockServer::start().await;
    let test_image = create_test_png();

    Mock::given(method("GET"))
        .and(path("/convert-test.png"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(test_image)
                .insert_header("content-type", "image/png"),
        )
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().unwrap();
    let app_state = create_app_state(temp_dir.path().to_path_buf());

    let mut app = test::init_service(App::new().app_data(web::Data::new(app_state)).route(
        "/img-optimizer/v1/img",
        web::get().to(optimize_image_handler),
    ))
    .await;

    // Convert to JPEG
    let image_url = format!("{}/convert-test.png", &mock_server.uri());
    let req = test::TestRequest::get()
        .uri(&format!("/img-optimizer/v1/img?src={}&f=jpeg", &image_url))
        .to_request();
    let resp = test::call_service(&mut app, req).await;

    assert!(resp.status().is_success());
}

#[actix_rt::test]
async fn test_direct_image_id_format() {
    let mut app = test::init_service(App::new().route(
        "/img-optimizer/v1/img/{image_id}",
        web::get().to(direct_image_handler),
    ))
    .await;

    // Test valid format (32 hex chars + extension)
    let req = test::TestRequest::get()
        .uri("/img-optimizer/v1/img/f86d5d7ae700c37dd8db36806074f231.png")
        .to_request();
    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), 422); // Expected as we don't have internal storage

    // Test invalid format
    let req = test::TestRequest::get()
        .uri("/img-optimizer/v1/img/invalid-id.png")
        .to_request();
    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), 400);
}

#[actix_rt::test]
async fn test_max_width_constraint() {
    let mock_server = MockServer::start().await;
    let test_image = create_test_png();

    Mock::given(method("GET"))
        .and(path("/large-image.png"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(test_image)
                .insert_header("content-type", "image/png"),
        )
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().unwrap();
    let app_state = create_app_state(temp_dir.path().to_path_buf());

    let mut app = test::init_service(App::new().app_data(web::Data::new(app_state)).route(
        "/img-optimizer/v1/img",
        web::get().to(optimize_image_handler),
    ))
    .await;

    // Test width above MAX_WIDTH (3840)
    let image_url = format!("{}/large-image.png", &mock_server.uri());
    let req = test::TestRequest::get()
        .uri(&format!("/img-optimizer/v1/img?src={}&w=5000", &image_url))
        .to_request();
    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), 400); // Width validation should fail
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["errorCode"], "VAL_001");
}

#[actix_rt::test]
async fn test_cache_functionality() {
    let mock_server = MockServer::start().await;
    let test_image = create_test_png();

    Mock::given(method("GET"))
        .and(path("/cached-image.png"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(test_image)
                .insert_header("content-type", "image/png"),
        )
        .expect(1) // Should only be called once due to caching
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().unwrap();
    let app_state = create_app_state(temp_dir.path().to_path_buf());

    let mut app = test::init_service(App::new().app_data(web::Data::new(app_state)).route(
        "/img-optimizer/v1/img",
        web::get().to(optimize_image_handler),
    ))
    .await;

    let image_url = format!("{}/cached-image.png", &mock_server.uri());

    // First request
    let req = test::TestRequest::get()
        .uri(&format!("/img-optimizer/v1/img?src={}&w=100", &image_url))
        .to_request();
    let resp1 = test::call_service(&mut app, req).await;
    assert!(resp1.status().is_success());
    let body1 = test::read_body(resp1).await;

    // Second request (should hit cache)
    let req = test::TestRequest::get()
        .uri(&format!("/img-optimizer/v1/img?src={}&w=100", &image_url))
        .to_request();
    let resp2 = test::call_service(&mut app, req).await;
    assert!(resp2.status().is_success());
    let body2 = test::read_body(resp2).await;

    // Bodies should be identical
    assert_eq!(body1, body2);
}

#[actix_rt::test]
async fn test_quality_parameter_bounds() {
    let mock_server = MockServer::start().await;
    let test_image = create_test_png();

    Mock::given(method("GET"))
        .and(path("/quality-test.png"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(test_image)
                .insert_header("content-type", "image/png"),
        )
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().unwrap();
    let app_state = create_app_state(temp_dir.path().to_path_buf());

    let mut app = test::init_service(App::new().app_data(web::Data::new(app_state)).route(
        "/img-optimizer/v1/img",
        web::get().to(optimize_image_handler),
    ))
    .await;

    let image_url = format!("{}/quality-test.png", &mock_server.uri());

    // Test quality below minimum
    let req = test::TestRequest::get()
        .uri(&format!("/img-optimizer/v1/img?src={}&q=0", &image_url))
        .to_request();
    let resp = test::call_service(&mut app, req).await;
    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["errorCode"], "VAL_002");

    // Test quality above maximum
    let req = test::TestRequest::get()
        .uri(&format!("/img-optimizer/v1/img?src={}&q=101", &image_url))
        .to_request();
    let resp = test::call_service(&mut app, req).await;
    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["errorCode"], "VAL_002");
}

#[actix_rt::test]
async fn test_plasmic_compatible_webp_format() {
    let mock_server = MockServer::start().await;
    let test_image = create_test_png();

    Mock::given(method("GET"))
        .and(path("/plasmic-test.png"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(test_image)
                .insert_header("content-type", "image/png"),
        )
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().unwrap();
    let app_state = create_app_state(temp_dir.path().to_path_buf());

    let mut app = test::init_service(App::new().app_data(web::Data::new(app_state)).route(
        "/img-optimizer/v1/img",
        web::get().to(optimize_image_handler),
    ))
    .await;

    // Test WebP format conversion (important for Plasmic)
    let image_url = format!("{}/plasmic-test.png", &mock_server.uri());
    let req = test::TestRequest::get()
        .uri(&format!(
            "/img-optimizer/v1/img?src={}&f=webp&q=80",
            &image_url
        ))
        .to_request();
    let resp = test::call_service(&mut app, req).await;

    assert!(resp.status().is_success());
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "image/webp"
    );
}

#[actix_rt::test]
async fn test_plasmic_compatible_resize_and_format() {
    let mock_server = MockServer::start().await;
    let test_image = create_test_png();

    Mock::given(method("GET"))
        .and(path("/resize-format.png"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(test_image)
                .insert_header("content-type", "image/png"),
        )
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().unwrap();
    let app_state = create_app_state(temp_dir.path().to_path_buf());

    let mut app = test::init_service(App::new().app_data(web::Data::new(app_state)).route(
        "/img-optimizer/v1/img",
        web::get().to(optimize_image_handler),
    ))
    .await;

    // Test resize + format conversion (common Plasmic use case)
    let image_url = format!("{}/resize-format.png", &mock_server.uri());
    let req = test::TestRequest::get()
        .uri(&format!(
            "/img-optimizer/v1/img?src={}&w=800&f=jpeg&q=90",
            &image_url
        ))
        .to_request();
    let resp = test::call_service(&mut app, req).await;

    assert!(resp.status().is_success());
}

#[actix_rt::test]
async fn test_plasmic_error_format_rfc7807() {
    let temp_dir = TempDir::new().unwrap();
    let app_state = create_app_state(temp_dir.path().to_path_buf());

    let mut app = test::init_service(App::new().app_data(web::Data::new(app_state)).route(
        "/img-optimizer/v1/img",
        web::get().to(optimize_image_handler),
    ))
    .await;

    let req = test::TestRequest::get()
        .uri("/img-optimizer/v1/img?src=invalid-url")
        .to_request();
    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;

    // Verify RFC7807 Problem Details format
    assert!(body["type"].is_string());
    assert!(body["title"].is_string());
    assert!(body["status"].is_number());
    assert!(body["detail"].is_string());
    assert!(body["errorCode"].is_string());
    assert!(body["howToFix"].is_string());
    assert!(body["moreInfo"].is_string());
}
