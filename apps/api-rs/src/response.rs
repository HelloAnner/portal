use axum::{
    body::{to_bytes, Body},
    http::{Request, Response, StatusCode},
};
use serde::Serialize;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::error;

#[derive(Serialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub data: Option<T>,
    pub request_id: String,
    pub error: Option<ApiErrorBody>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T, request_id: String) -> Self {
        Self {
            data: Some(data),
            request_id,
            error: None,
        }
    }
}

#[derive(Clone)]
pub struct EnvelopeLayer;

impl<S> Layer<S> for EnvelopeLayer {
    type Service = EnvelopeService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        EnvelopeService { inner }
    }
}

#[derive(Clone)]
pub struct EnvelopeService<S> {
    inner: S,
}

impl<S, ReqBody> Service<Request<ReqBody>> for EnvelopeService<S>
where
    S: Service<Request<ReqBody>, Response = Response<Body>>,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let request_id = req
            .extensions()
            .get::<String>()
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let future = self.inner.call(req);
        Box::pin(async move {
            let response = future.await?;
            let (parts, body) = response.into_parts();
            let status = parts.status;

            let bytes = match to_bytes(body, usize::MAX).await {
                Ok(b) => b,
                Err(e) => {
                    error!("failed to read response body: {}", e);
                    let fallback = serde_json::json!({
                        "data": null,
                        "request_id": request_id,
                        "error": { "code": "INTERNAL_ERROR", "message": "failed to read response body" }
                    });
                    return Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header("content-type", "application/json")
                        .body(Body::from(fallback.to_string()))
                        .unwrap());
                }
            };

            let content_type = parts
                .headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if !content_type.starts_with("application/json") || bytes.is_empty() {
                let response = Response::from_parts(parts, Body::from(bytes));
                return Ok(response);
            }

            let inner: Value = match serde_json::from_slice(&bytes) {
                Ok(v) => v,
                Err(_) => {
                    let response = Response::from_parts(parts, Body::from(bytes));
                    return Ok(response);
                }
            };

            let envelope = if status.is_client_error() || status.is_server_error() {
                serde_json::json!({
                    "data": null,
                    "request_id": request_id,
                    "error": inner
                })
            } else {
                serde_json::json!({
                    "data": inner,
                    "request_id": request_id,
                    "error": null
                })
            };

            let mut builder = Response::builder().status(status);
            for (k, v) in parts.headers.iter().filter(|(k, _)| k.as_str() != "content-length") {
                builder = builder.header(k, v);
            }
            Ok(builder
                .header("content-type", "application/json")
                .body(Body::from(envelope.to_string()))
                .unwrap())
        })
    }
}
