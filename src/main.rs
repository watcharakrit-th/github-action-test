use axum::{Json, Router, extract::Path, routing::get};
use serde::Serialize;
use tokio::net::TcpListener;

#[derive(Serialize)]
struct Echo {
    echo: String,
}

/// `GET /echo/{message}` -> `{"echo":"<message>"}`
async fn echo(Path(message): Path<String>) -> Json<Echo> {
    Json(Echo { echo: message })
}

/// `GET /healthz` -> `ok` (used by the Kubernetes probes)
async fn healthz() -> &'static str {
    "ok"
}

fn app() -> Router {
    Router::new()
        .route("/echo/{message}", get(echo))
        .route("/healthz", get(healthz))
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    println!("echo-api listening on {}", listener.local_addr()?);

    axum::serve(listener, app())
        .with_graceful_shutdown(shutdown_signal())
        .await
}

/// Resolves on Ctrl-C or SIGTERM, so `docker stop` and pod termination exit promptly.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    async fn send(uri: &str) -> (StatusCode, String) {
        let response = app()
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn echoes_the_path_param() {
        let (status, body) = send("/echo/hello").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, r#"{"echo":"hello"}"#);
    }

    #[tokio::test]
    async fn decodes_percent_encoded_param() {
        let (status, body) = send("/echo/hello%20world").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, r#"{"echo":"hello world"}"#);
    }

    #[tokio::test]
    async fn escapes_json_in_the_param() {
        let (status, body) = send("/echo/%22quoted%22").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, r#"{"echo":"\"quoted\""}"#);
    }

    #[tokio::test]
    async fn missing_param_is_not_found() {
        let (status, _) = send("/echo/").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn healthz_returns_ok() {
        let (status, body) = send("/healthz").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "ok");
    }
}
