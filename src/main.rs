mod captcha;
mod error;
mod mail;

use crate::captcha::Captchas;
use crate::error::Error;
use axum::{
    body::Body,
    extract::Extension,
    http::{Response, StatusCode},
    routing::{get, post},
    Json, Router,
};
use dotenv::dotenv;
use error::Result;
use http::{
    header::{HeaderValue, CONTENT_TYPE},
    Method,
};
use serde::Deserialize;
use std::{env, net::SocketAddr};
use tower::ServiceBuilder;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::warn;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    dotenv().ok();

    assert!(env::var("SMTP_RELAY").is_ok(), "env var SMTP_RELAY not set");
    assert!(env::var("SMTP_USER").is_ok(), "env var SMTP_USER not set");
    assert!(
        env::var("SMTP_PASSWORD").is_ok(),
        "env var SMTP_PASSWORD not set"
    );
    assert!(env::var("FROM").is_ok(), "env var FROM not set");
    assert!(env::var("TO").is_ok(), "env var TO not set");
    assert!(env::var("SUBJECT").is_ok(), "env var SUBJECT not set");

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port = dotenv::var("PORT")
        .unwrap_or_else(|_| "1313".into())
        .parse::<u16>()
        .expect("invalid PORT");

    let allowed_origins: String = env::var("CORS_ALLOWED_ORIGINS")
        .expect("CORS_ALLOWED_ORIGINS not set")
        .parse()
        .expect("fails to parse CORS_ALLOWED_ORIGINS");

    let allowed_origins = allowed_origins.split(',').into_iter().map(|v| {
        HeaderValue::from_str(v).expect("fails to convert CORS_ALLOWED_ORIGINS to HeaderValue")
    });

    let cors = CorsLayer::new()
        .allow_headers(vec![CONTENT_TYPE])
        .allow_credentials(true)
        .allow_methods(vec![Method::GET, Method::POST])
        .allow_origin(AllowOrigin::list(allowed_origins));

    let middleware_stack = ServiceBuilder::new()
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().include_headers(true))
                .on_response(
                    DefaultOnResponse::new()
                        .level(tracing::Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                ),
        )
        .layer(cors)
        .layer(Extension(Captchas::new()));

    let app = Router::new()
        .route("/health", get(health))
        .route("/captcha", get(get_captcha))
        .route("/mail", post(send_mail))
        .layer(middleware_stack);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("fails to start axum server");
}

async fn health() -> Result<String> {
    Ok("I'm ok.".into())
}

async fn get_captcha(Extension(captchas): Extension<Captchas>) -> Result<Response<Body>> {
    let image = captchas.insert()?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(image))
        .unwrap())
}

#[derive(Clone, Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct MailIn {
    captcha_text: String,
    mail_text: String,
}

async fn send_mail(
    Extension(mut captchas): Extension<Captchas>,
    Json(payload): Json<MailIn>,
) -> Result<()> {
    captchas.check(payload.captcha_text)?;

    if payload.mail_text.len() > 10_000 {
        warn!("received mail with more than 10k characters");

        return Err(Error::BadRequest(
            "Mail should not be longer than 10k characters".to_string(),
        ));
    }

    mail::send_mail(payload.mail_text).await?;

    Ok(())
}
