use std::sync::Arc;

use crate::{
    server::responses::{ApiResponse, Status},
    state::{rate_limit::RateLimitStateTrait, AppState},
};
use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::{Json, Response},
    Extension,
};

pub async fn rate_limit_middleware<B>(
    Extension(state): Extension<Arc<AppState>>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, (StatusCode, Json<ApiResponse>)> {
    if state.rate_limit.can_fetch().await {
        return Ok(next.run(request).await);
    }
    Err((
        StatusCode::UNAUTHORIZED,
        Json(ApiResponse {
            status: Status::Unauthorized,
            data: "Too many requests".into(),
        }),
    ))
}
