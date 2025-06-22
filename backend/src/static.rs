use crate::header;
use axum::response::IntoResponse;

pub async fn style() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css")],
        include_bytes!("../assets/style.css"),
    )
}

pub async fn form_style() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css")],
        include_bytes!("../assets/style-form.css"),
    )
}

pub async fn script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript")],
        include_bytes!("../assets/script.js"),
    )
}

pub async fn font() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/octet-stream")],
        "", //include_bytes!("../assets/font.ttf"),
    )
}

pub async fn icon() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/x-icon")],
        "", //include_bytes!("../assets/icon.ico"),
    )
}

pub async fn robots() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/plain")],
        include_str!("../assets/robots.txt"),
    )
}
