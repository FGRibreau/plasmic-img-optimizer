use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use serde::Serialize;
use strum::EnumIter;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, EnumIter, thiserror::Error)]
pub enum AppError {
    #[error("IMG_001: Invalid image URL - The provided URL is not valid")]
    InvalidImageUrl,

    #[error("IMG_002: Image fetch failed - Unable to download image from {url}")]
    ImageFetchFailed { url: String },

    #[error("IMG_003: Image processing failed - Error processing image: {reason}")]
    ImageProcessingFailed { reason: String },

    #[error("IMG_004: Invalid image format - Format '{format}' is not supported")]
    InvalidImageFormat { format: String },

    #[error("IMG_005: Image too large - Image dimensions exceed maximum allowed size")]
    ImageTooLarge,

    #[error("VAL_001: Invalid width - Width must be between 1 and 3840, got {width}")]
    InvalidWidth { width: u32 },

    #[error("VAL_002: Invalid quality - Quality must be between 1 and 100, got {quality}")]
    InvalidQuality { quality: u8 },

    #[error("VAL_003: Missing required parameter - {param} is required")]
    MissingRequiredParameter { param: String },

    #[error("CACHE_001: Cache error - Failed to access cache: {reason}")]
    CacheError { reason: String },

    #[error("SYS_001: Internal server error - An unexpected error occurred")]
    InternalServerError,

    #[error("SYS_002: Service unavailable - The service is temporarily unavailable")]
    ServiceUnavailable,
}

#[derive(Serialize)]
pub struct ProblemDetails {
    #[serde(rename = "type")]
    error_type: String,
    title: String,
    status: u16,
    detail: String,
    instance: Option<String>,
    #[serde(rename = "errorCode")]
    error_code: String,
    #[serde(rename = "howToFix")]
    how_to_fix: String,
    #[serde(rename = "moreInfo")]
    more_info: String,
}

impl AppError {
    fn error_code(&self) -> &'static str {
        match self {
            AppError::InvalidImageUrl => "IMG_001",
            AppError::ImageFetchFailed { .. } => "IMG_002",
            AppError::ImageProcessingFailed { .. } => "IMG_003",
            AppError::InvalidImageFormat { .. } => "IMG_004",
            AppError::ImageTooLarge => "IMG_005",
            AppError::InvalidWidth { .. } => "VAL_001",
            AppError::InvalidQuality { .. } => "VAL_002",
            AppError::MissingRequiredParameter { .. } => "VAL_003",
            AppError::CacheError { .. } => "CACHE_001",
            AppError::InternalServerError => "SYS_001",
            AppError::ServiceUnavailable => "SYS_002",
        }
    }

    fn how_to_fix(&self) -> String {
        match self {
            AppError::InvalidImageUrl => {
                "Provide a valid URL starting with http:// or https://".to_string()
            }
            AppError::ImageFetchFailed { .. } => {
                "Ensure the image URL is accessible and the server is responding".to_string()
            }
            AppError::ImageProcessingFailed { .. } => {
                "Try a different image or check if the image file is corrupted".to_string()
            }
            AppError::InvalidImageFormat { format } => {
                format!("Use one of the supported formats: jpeg, jpg, png, webp. Got '{format}'")
            }
            AppError::ImageTooLarge => {
                "Reduce the image dimensions or use a smaller source image".to_string()
            }
            AppError::InvalidWidth { .. } => "Provide a width value between 1 and 3840".to_string(),
            AppError::InvalidQuality { .. } => {
                "Provide a quality value between 1 and 100".to_string()
            }
            AppError::MissingRequiredParameter { param } => {
                format!("Include the '{param}' parameter in your request")
            }
            AppError::CacheError { .. } => {
                "Try again later or contact support if the issue persists".to_string()
            }
            AppError::InternalServerError => {
                "Try again later. If the problem persists, contact support".to_string()
            }
            AppError::ServiceUnavailable => {
                "The service is temporarily down. Please try again in a few minutes".to_string()
            }
        }
    }

    pub fn list_all_errors() -> Vec<String> {
        use strum::IntoEnumIterator;
        AppError::iter()
            .map(|e| format!("{}: {}", e.error_code(), e))
            .collect()
    }

    pub fn to_response(&self) -> ProblemDetails {
        ProblemDetails {
            error_type: format!(
                "https://github.com/fgribreau/plasmic-img-optimizer/errors/{}",
                self.error_code()
            ),
            title: match self {
                AppError::InvalidImageUrl
                | AppError::InvalidImageFormat { .. }
                | AppError::InvalidWidth { .. }
                | AppError::InvalidQuality { .. }
                | AppError::MissingRequiredParameter { .. } => "Bad Request",
                AppError::ImageFetchFailed { .. }
                | AppError::ImageProcessingFailed { .. }
                | AppError::ImageTooLarge
                | AppError::CacheError { .. } => "Processing Error",
                AppError::InternalServerError => "Internal Server Error",
                AppError::ServiceUnavailable => "Service Unavailable",
            }
            .to_string(),
            status: self.status_code().as_u16(),
            detail: self.to_string(),
            instance: None,
            error_code: self.error_code().to_string(),
            how_to_fix: self.how_to_fix(),
            more_info: format!(
                "https://github.com/fgribreau/plasmic-img-optimizer#error-{}",
                self.error_code().to_lowercase()
            ),
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::ImageFetchFailed {
            url: err.url().map(|u| u.to_string()).unwrap_or_default(),
        }
    }
}

impl From<image::ImageError> for AppError {
    fn from(err: image::ImageError) -> Self {
        AppError::ImageProcessingFailed {
            reason: err.to_string(),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::CacheError {
            reason: err.to_string(),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(_: anyhow::Error) -> Self {
        AppError::InternalServerError
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let problem_details = ProblemDetails {
            error_type: format!(
                "https://github.com/fgribreau/plasmic-img-optimizer/errors/{}",
                self.error_code()
            ),
            title: match self {
                AppError::InvalidImageUrl
                | AppError::InvalidImageFormat { .. }
                | AppError::InvalidWidth { .. }
                | AppError::InvalidQuality { .. }
                | AppError::MissingRequiredParameter { .. } => "Bad Request",
                AppError::ImageFetchFailed { .. }
                | AppError::ImageProcessingFailed { .. }
                | AppError::ImageTooLarge
                | AppError::CacheError { .. } => "Processing Error",
                AppError::InternalServerError => "Internal Server Error",
                AppError::ServiceUnavailable => "Service Unavailable",
            }
            .to_string(),
            status: status_code.as_u16(),
            detail: self.to_string(),
            instance: None,
            error_code: self.error_code().to_string(),
            how_to_fix: self.how_to_fix(),
            more_info: format!(
                "https://github.com/fgribreau/plasmic-img-optimizer#error-{}",
                self.error_code().to_lowercase()
            ),
        };

        HttpResponse::build(status_code).json(problem_details)
    }

    fn status_code(&self) -> StatusCode {
        match self {
            AppError::InvalidImageUrl
            | AppError::InvalidImageFormat { .. }
            | AppError::InvalidWidth { .. }
            | AppError::InvalidQuality { .. }
            | AppError::MissingRequiredParameter { .. } => StatusCode::BAD_REQUEST,
            AppError::ImageFetchFailed { .. }
            | AppError::ImageProcessingFailed { .. }
            | AppError::ImageTooLarge
            | AppError::CacheError { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}
