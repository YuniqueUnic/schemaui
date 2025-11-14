#![cfg(feature = "web")]

use std::{
    future::IntoFuture,
    net::{IpAddr, SocketAddr},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::{Context, Result, anyhow};
use axum::{
    Json, Router,
    body::Body,
    extract::{OriginalUri, State},
    http::{Response, StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use jsonschema::{Validator, validator_for};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tokio::{
    net::TcpListener,
    sync::{Mutex, oneshot},
};

use crate::io::{DocumentFormat, input::schema_with_defaults};

use super::{
    assets,
    blueprint::{WebBlueprint, blueprint_from_schema},
};

pub struct WebSessionBuilder {
    schema: Value,
    defaults: Option<Value>,
    title: Option<String>,
}

impl WebSessionBuilder {
    pub fn new(schema: Value) -> Self {
        Self {
            schema,
            defaults: None,
            title: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_initial_data(mut self, data: Value) -> Self {
        self.defaults = Some(data);
        self
    }

    pub fn build(mut self) -> Result<WebSessionConfig> {
        let data = self
            .defaults
            .take()
            .unwrap_or_else(|| Value::Object(Map::new()));
        let schema = schema_with_defaults(&self.schema, &data);
        let blueprint = blueprint_from_schema(&schema)?;
        Ok(WebSessionConfig {
            title: self.title,
            blueprint,
            data,
            schema,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WebSessionConfig {
    pub title: Option<String>,
    pub blueprint: WebBlueprint,
    pub data: Value,
    pub schema: Value,
}

#[derive(Debug, Clone, Copy)]
pub struct ServeOptions {
    pub host: IpAddr,
    pub port: u16,
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            host: IpAddr::from([127, 0, 0, 1]),
            port: 0,
        }
    }
}

pub struct BoundSession {
    router: Router,
    handles: SessionHandles,
    listener: TcpListener,
    addr: SocketAddr,
}

impl BoundSession {
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn run(self) -> Result<Value> {
        let (result_rx, shutdown_rx) = self.handles.into_parts();
        let server = axum::serve(self.listener, self.router.into_make_service())
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .into_future();
        tokio::pin!(server);
        tokio::pin!(result_rx);

        let value = tokio::select! {
            value = &mut result_rx => {
                value.map_err(|_| anyhow!("web session closed before emitting a value"))?
            }
            outcome = &mut server => {
                outcome.context("web server exited before the session finished")?;
                return Err(anyhow!("web server terminated prematurely"));
            }
        };

        let _ = server.await;
        Ok(value)
    }
}

pub async fn bind_session(config: WebSessionConfig, options: ServeOptions) -> Result<BoundSession> {
    let (router, handles) = session_router(config)?;
    let listener = TcpListener::bind(SocketAddr::new(options.host, options.port))
        .await
        .context("failed to bind web listener")?;
    let addr = listener
        .local_addr()
        .context("failed to read bound address")?;
    Ok(BoundSession {
        router,
        handles,
        listener,
        addr,
    })
}

pub async fn serve_session(config: WebSessionConfig, options: ServeOptions) -> Result<Value> {
    bind_session(config, options).await?.run().await
}

pub fn session_router(config: WebSessionConfig) -> Result<(Router, SessionHandles)> {
    let WebSessionConfig {
        title,
        blueprint,
        data,
        schema,
    } = config;
    let validator = Arc::new(validator_for(&schema).context("failed to compile JSON schema")?);
    let (result_tx, result_rx) = oneshot::channel();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let shared = SharedState {
        title,
        blueprint: Arc::new(blueprint),
        data: Arc::new(Mutex::new(data)),
        formats: DocumentFormat::available_formats(),
        validator,
        finish_line: Arc::new(FinishLine {
            result: Mutex::new(Some(result_tx)),
            shutdown: Mutex::new(Some(shutdown_tx)),
        }),
        finished: Arc::new(AtomicBool::new(false)),
    };

    let router = Router::new()
        .route("/api/session", get(get_session))
        .route("/api/save", post(post_save))
        .route("/api/exit", post(post_exit))
        .route("/api/validate", post(post_validate))
        .route("/api/preview", post(post_preview))
        .fallback(static_assets)
        .with_state(shared);

    Ok((
        router,
        SessionHandles {
            result_rx: Some(result_rx),
            shutdown_rx: Some(shutdown_rx),
        },
    ))
}

pub struct SessionHandles {
    result_rx: Option<oneshot::Receiver<Value>>,
    shutdown_rx: Option<oneshot::Receiver<()>>,
}

impl SessionHandles {
    pub fn into_parts(mut self) -> (oneshot::Receiver<Value>, oneshot::Receiver<()>) {
        let result = self
            .result_rx
            .take()
            .expect("result receiver already consumed");
        let shutdown = self
            .shutdown_rx
            .take()
            .expect("shutdown receiver already consumed");
        (result, shutdown)
    }
}

#[derive(Clone)]
struct SharedState {
    title: Option<String>,
    blueprint: Arc<WebBlueprint>,
    data: Arc<Mutex<Value>>,
    formats: Vec<DocumentFormat>,
    validator: Arc<Validator>,
    finish_line: Arc<FinishLine>,
    finished: Arc<AtomicBool>,
}

struct FinishLine {
    result: Mutex<Option<oneshot::Sender<Value>>>,
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
}

impl FinishLine {
    async fn complete(&self, value: Value) {
        if let Some(tx) = self.result.lock().await.take() {
            let _ = tx.send(value);
        }
        if let Some(tx) = self.shutdown.lock().await.take() {
            let _ = tx.send(());
        }
    }
}

#[derive(Serialize)]
struct SessionResponse {
    title: Option<String>,
    blueprint: WebBlueprint,
    data: Value,
    formats: Vec<String>,
}

async fn get_session(State(state): State<SharedState>) -> impl IntoResponse {
    let payload = SessionResponse {
        title: state.title.clone(),
        blueprint: (*state.blueprint).clone(),
        data: state.data.lock().await.clone(),
        formats: state.formats.iter().map(|f| f.to_string()).collect(),
    };
    Json(payload)
}

#[derive(Deserialize)]
struct SaveRequest {
    data: Value,
}

async fn post_save(
    State(state): State<SharedState>,
    Json(req): Json<SaveRequest>,
) -> (StatusCode, Json<Value>) {
    if state.finished.load(Ordering::SeqCst) {
        return (
            StatusCode::GONE,
            Json(json!({"error": "session already closed"})),
        );
    }
    *state.data.lock().await = req.data;
    (StatusCode::OK, Json(json!({"status": "saved"})))
}

#[derive(Deserialize)]
struct ExitRequest {
    data: Value,
    #[serde(default = "default_true")]
    commit: bool,
}

fn default_true() -> bool {
    true
}

async fn post_exit(
    State(state): State<SharedState>,
    Json(req): Json<ExitRequest>,
) -> (StatusCode, Json<Value>) {
    if state.finished.swap(true, Ordering::SeqCst) {
        return (
            StatusCode::GONE,
            Json(json!({"error": "session already closed"})),
        );
    }

    let final_value = if req.commit {
        req.data
    } else {
        state.data.lock().await.clone()
    };
    state.finish_line.complete(final_value).await;
    (StatusCode::OK, Json(json!({"status": "closing"})))
}

#[derive(Deserialize)]
struct ValidateRequest {
    data: Value,
}

#[derive(Serialize)]
struct ValidationResponse {
    ok: bool,
    errors: Vec<FieldError>,
}

#[derive(Serialize)]
struct FieldError {
    pointer: String,
    message: String,
}

async fn post_validate(
    State(state): State<SharedState>,
    Json(req): Json<ValidateRequest>,
) -> impl IntoResponse {
    let mut errors = Vec::new();
    for error in state.validator.iter_errors(&req.data) {
        errors.push(FieldError {
            pointer: error.instance_path.to_string(),
            message: error.to_string(),
        });
    }
    Json(ValidationResponse {
        ok: errors.is_empty(),
        errors,
    })
}

#[derive(Deserialize)]
struct PreviewRequest {
    data: Value,
    format: String,
    #[serde(default = "default_true")]
    pretty: bool,
}

#[derive(Serialize)]
struct PreviewResponse {
    payload: String,
}

async fn post_preview(
    State(state): State<SharedState>,
    Json(req): Json<PreviewRequest>,
) -> Result<Json<PreviewResponse>, (StatusCode, Json<Value>)> {
    render_payload(&req.data, &req.format, req.pretty, &state.formats)
        .map(|payload| Json(PreviewResponse { payload }))
        .map_err(|err| (StatusCode::BAD_REQUEST, Json(json!({"error": err}))))
}

fn render_payload(
    data: &Value,
    format_keyword: &str,
    pretty: bool,
    allowed: &[DocumentFormat],
) -> Result<String, String> {
    let format = DocumentFormat::from_keyword(format_keyword)?;
    if !allowed.contains(&format) {
        return Err(format!("format '{format}' is disabled for this build"));
    }
    encode_value(data, format, pretty).map_err(|err| err.to_string())
}

fn encode_value(
    value: &Value,
    format: DocumentFormat,
    pretty: bool,
) -> Result<String, anyhow::Error> {
    match format {
        DocumentFormat::Json => {
            if pretty {
                Ok(serde_json::to_string_pretty(value)?)
            } else {
                Ok(serde_json::to_string(value)?)
            }
        }
        #[cfg(feature = "yaml")]
        DocumentFormat::Yaml => Ok(serde_yaml::to_string(value)?),
        #[cfg(feature = "toml")]
        DocumentFormat::Toml => {
            if pretty {
                Ok(toml::to_string_pretty(value)?)
            } else {
                Ok(toml::to_string(value)?)
            }
        }
        #[cfg(all(not(feature = "yaml"), not(feature = "toml")))]
        _ => Err(anyhow!("unsupported format")),
    }
}

async fn static_assets(State(_state): State<SharedState>, uri: OriginalUri) -> impl IntoResponse {
    if let Some(asset) = assets::asset(uri.path()) {
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CACHE_CONTROL, "no-store")
            .header(header::CONTENT_TYPE, asset.mime)
            .body(Body::from(asset.contents))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .unwrap()
            })
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("not found"))
            .unwrap()
    }
}
