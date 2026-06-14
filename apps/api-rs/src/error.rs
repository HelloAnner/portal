use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("AUTH_REQUIRED")]
    AuthRequired,
    #[error("PERMISSION_DENIED")]
    PermissionDenied,
    #[error("SYSTEM_DISABLED")]
    SystemDisabled,
    #[error("TENANT_DISABLED")]
    TenantDisabled,
    #[error("INVALID_SUBSYSTEM_TICKET")]
    InvalidSubsystemTicket,
    #[error("VALIDATION_FAILED: {0}")]
    ValidationFailed(String),
    #[error("CONFLICT: {0}")]
    Conflict(String),
    #[error("NOT_FOUND")]
    NotFound,
    #[error("INTERNAL_ERROR")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::AuthRequired => "AUTH_REQUIRED",
            AppError::PermissionDenied => "PERMISSION_DENIED",
            AppError::SystemDisabled => "SYSTEM_DISABLED",
            AppError::TenantDisabled => "TENANT_DISABLED",
            AppError::InvalidSubsystemTicket => "INVALID_SUBSYSTEM_TICKET",
            AppError::ValidationFailed(_) => "VALIDATION_FAILED",
            AppError::Conflict(_) => "CONFLICT",
            AppError::NotFound => "NOT_FOUND",
            AppError::Internal(_) => "INTERNAL_ERROR",
        }
    }

    pub fn message(&self) -> String {
        match self {
            AppError::ValidationFailed(msg) => msg.clone(),
            AppError::Conflict(msg) => msg.clone(),
            _ => self.to_string(),
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::AuthRequired => StatusCode::UNAUTHORIZED,
            AppError::PermissionDenied => StatusCode::FORBIDDEN,
            AppError::SystemDisabled => StatusCode::FORBIDDEN,
            AppError::TenantDisabled => StatusCode::FORBIDDEN,
            AppError::InvalidSubsystemTicket => StatusCode::BAD_REQUEST,
            AppError::ValidationFailed(_) => StatusCode::BAD_REQUEST,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let code = self.code().to_string();
        let message = self.message();
        let body = Json(ErrorBody { code, message });
        (status, body).into_response()
    }
}

pub type ApiResult<T> = Result<T, AppError>;
