use std::{
    future::IntoFuture,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
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
#[cfg(feature = "web-types")]
use ts_rs::TS;

use crate::draft::SessionDraft;
use crate::io::{DocumentFormat, input::schema_with_defaults, output::OutputOptions};
use crate::precompile::UiArtifactBundle;
use crate::schema::metadata::root_schema_header;

use super::assets::{EmbeddedAssets, FilesystemAssets, WebAssetProvider};
use super::theme::Theme;
use crate::core::frontend::SessionOutcome;
use crate::ui_ast::{UiAst, UiAstBundle, UiLayout, build_ui_ast_bundle};

/// The contract this server speaks, reported in every session response.
///
/// Paths are versioned too (`/api/v1/...`), which is what lets a client fail
/// fast on an incompatible server; this field distinguishes additive revisions
/// within that major version.
pub const API_VERSION: &str = "1.0";

/// An optional part of the contract this session actually offers.
///
/// A client reads this instead of probing endpoints: a 404 on a capability it
/// was never offered is not a useful thing to discover at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "web-types", derive(TS))]
#[cfg_attr(feature = "web-types", ts(export, export_to = "web/types/"))]
pub enum Capability {
    /// `GET /api/v1/theme.css` is served, and the frontend should link it.
    Theme,
    /// Saving writes a draft that outlives the process.
    Draft,
}

pub struct WebSessionBuilder {
    schema: Value,
    defaults: Option<Value>,
    title: Option<String>,
    description: Option<String>,
    asset_provider: Arc<dyn WebAssetProvider>,
    ui_bundle: Option<UiAstBundle>,
    ui_artifact_bundle: Option<UiArtifactBundle>,
    deadline: Option<Instant>,
    theme: Option<Theme>,
    draft: Option<SessionDraft>,
}

impl WebSessionBuilder {
    pub fn new(schema: Value) -> Self {
        Self {
            schema,
            defaults: None,
            title: None,
            description: None,
            #[allow(clippy::default_constructed_unit_structs)]
            asset_provider: Arc::new(EmbeddedAssets::default()),
            ui_bundle: None,
            ui_artifact_bundle: None,
            deadline: None,
            theme: None,
            draft: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_initial_data(mut self, data: Value) -> Self {
        self.defaults = Some(data);
        self
    }

    pub fn with_asset_provider(mut self, provider: Arc<dyn WebAssetProvider>) -> Self {
        self.asset_provider = provider;
        self
    }

    pub fn with_filesystem_assets<P: Into<PathBuf>>(mut self, root: P) -> Self {
        self.asset_provider = Arc::new(FilesystemAssets::new(root));
        self
    }

    /// Provide a prepared UiAst for this schema.
    pub fn with_ui_ast(mut self, ast: UiAst) -> Self {
        self.ui_bundle = Some(UiAstBundle::from_ui_ast(ast));
        self
    }

    /// Provide a prepared bundle of shared UI artifacts for this schema.
    pub fn with_ui_bundle(mut self, bundle: UiAstBundle) -> Self {
        self.ui_bundle = Some(bundle);
        self
    }

    /// Provide a prepared UI artifact bundle.
    pub fn with_ui_artifact_bundle(mut self, bundle: UiArtifactBundle) -> Self {
        self.ui_artifact_bundle = Some(bundle);
        self
    }

    /// Stop the session at `deadline`, reporting [`SessionOutcome::TimedOut`].
    pub fn with_deadline(mut self, deadline: Option<Instant>) -> Self {
        self.deadline = deadline;
        self
    }

    /// Serve `theme` alongside the built-in stylesheet.
    pub fn with_theme(mut self, theme: Option<Theme>) -> Self {
        self.theme = theme;
        self
    }

    /// Persist saves to `draft`, so an interrupted session can be resumed.
    pub fn with_draft(mut self, draft: Option<SessionDraft>) -> Self {
        self.draft = draft;
        self
    }

    pub fn build(mut self) -> Result<WebSessionConfig> {
        let data = self
            .defaults
            .take()
            .unwrap_or_else(|| Value::Object(Map::new()));
        let schema = schema_with_defaults(&self.schema, &data);
        let (schema_title, description) = root_schema_header(&schema);
        let bundle = if let Some(bundle) = self.ui_artifact_bundle.take() {
            bundle.ui
        } else if let Some(bundle) = self.ui_bundle.take() {
            bundle
        } else {
            build_ui_ast_bundle(&schema)?
        };
        let (ui_ast, layout) = bundle.into_parts();
        Ok(WebSessionConfig {
            title: self.title.or(schema_title),
            description: self.description.or(description),
            ui_ast,
            layout,
            data,
            schema,
            asset_provider: self.asset_provider,
            deadline: self.deadline,
            theme: self.theme,
            draft: self.draft,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WebSessionConfig {
    pub title: Option<String>,
    pub description: Option<String>,
    pub ui_ast: UiAst,
    pub layout: UiLayout,
    pub data: Value,
    pub schema: Value,
    pub asset_provider: Arc<dyn WebAssetProvider>,
    pub deadline: Option<Instant>,
    pub theme: Option<Theme>,
    pub draft: Option<SessionDraft>,
}

impl WebSessionConfig {
    pub fn session_response(&self) -> SessionResponse {
        SessionResponse {
            api_version: API_VERSION.to_string(),
            capabilities: capabilities(self.theme.as_ref(), self.draft.as_ref()),
            title: self.title.clone(),
            description: self.description.clone(),
            ui_ast: self.ui_ast.clone(),
            data: self.data.clone(),
            formats: DocumentFormat::available_formats()
                .into_iter()
                .map(|format| format.to_string())
                .collect(),
            layout: self.layout.clone(),
            expires_in_ms: remaining_ms(self.deadline),
            draft_restored: self.draft.as_ref().is_some_and(|draft| draft.restored),
        }
    }
}

/// What this session offers beyond the mandatory endpoints.
fn capabilities(theme: Option<&Theme>, draft: Option<&SessionDraft>) -> Vec<Capability> {
    let mut capabilities = Vec::new();
    if theme.is_some() {
        capabilities.push(Capability::Theme);
    }
    if draft.is_some() {
        capabilities.push(Capability::Draft);
    }
    capabilities
}

/// Milliseconds left before `deadline`, or `None` when the session is unbounded.
fn remaining_ms(deadline: Option<Instant>) -> Option<u64> {
    deadline.map(|deadline| {
        deadline
            .saturating_duration_since(Instant::now())
            .as_millis()
            .min(u64::MAX as u128) as u64
    })
}

/// How the temporary HTTP server is configured: where it listens, what it
/// serves, and what it serves it wearing.
///
/// Everything web-specific lives here rather than on the shared frontend
/// context, so a TUI-only build never has to know these concepts exist.
#[derive(Debug, Clone)]
pub struct ServeOptions {
    pub host: IpAddr,
    pub port: u16,
    /// What answers the paths the API does not claim. Defaults to the SPA
    /// embedded in the binary; point it at a directory to run a different
    /// frontend entirely.
    pub assets: Arc<dyn WebAssetProvider>,
    /// A user stylesheet, served at `/api/v1/theme.css`.
    pub theme: Option<Theme>,
}

impl ServeOptions {
    /// Listen on `host:port`, serving the embedded SPA with no user theme.
    pub fn new(host: IpAddr, port: u16) -> Self {
        Self {
            host,
            port,
            ..Self::default()
        }
    }

    /// Serve a frontend from `root` instead of the embedded SPA.
    pub fn with_frontend_dir(mut self, root: impl Into<PathBuf>) -> Self {
        self.assets = Arc::new(FilesystemAssets::new(root));
        self
    }

    pub fn with_theme(mut self, theme: Option<Theme>) -> Self {
        self.theme = theme;
        self
    }

    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            host: IpAddr::from([127, 0, 0, 1]),
            port: 0,
            #[allow(clippy::default_constructed_unit_structs)]
            assets: Arc::new(EmbeddedAssets::default()),
            theme: None,
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

    pub async fn run(self) -> Result<SessionOutcome> {
        let Self {
            router,
            handles,
            listener,
            addr: _,
        } = self;
        let (result_rx, shutdown_rx) = handles.into_parts();

        let server = axum::serve(listener, router.into_make_service())
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .into_future();
        let server = tokio::spawn(server);

        let outcome = match result_rx.await {
            Ok(outcome) => outcome,
            Err(_) => match server.await {
                Ok(Ok(())) => {
                    return Err(anyhow!("web session closed before emitting a value"));
                }
                Ok(Err(err)) => {
                    return Err(err).context("web server exited before the session finished");
                }
                Err(err) => {
                    return Err(anyhow!(err))
                        .context("web server task panicked before the session finished");
                }
            },
        };

        match server.await {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                return Err(err).context("web server exited before the session finished");
            }
            Err(err) => {
                return Err(anyhow!(err)).context("web server task panicked");
            }
        }
        Ok(outcome)
    }
}

/// Wire up a session and bind it to `addr`.
///
/// Takes an address rather than the whole [`ServeOptions`]: everything else in
/// that struct describes what to *serve*, and by this point it has already been
/// resolved into `config`. Passing both would invite the two to disagree.
pub async fn bind_session(config: WebSessionConfig, addr: SocketAddr) -> Result<BoundSession> {
    // Captured here rather than inside the blocking task so the timer lands on
    // the caller's runtime, which is the one that will drive the server.
    let runtime = tokio::runtime::Handle::current();
    let wiring = tokio::task::spawn_blocking(move || build_session_wiring(config))
        .await
        .context("failed to build web session state")??;
    wiring.arm_deadline(runtime);
    let listener = TcpListener::bind(addr)
        .await
        .context("failed to bind web listener")?;
    let addr = listener
        .local_addr()
        .context("failed to read bound address")?;
    Ok(BoundSession {
        router: wiring.router,
        handles: wiring.handles,
        listener,
        addr,
    })
}

pub async fn serve_session(config: WebSessionConfig, addr: SocketAddr) -> Result<SessionOutcome> {
    bind_session(config, addr).await?.run().await
}

/// Build the session's routes and shared state.
///
/// Custom HTTP stacks can mount the returned router instead of going through
/// [`bind_session`]. The deadline is armed here, on the calling runtime, so a
/// session wired up this way expires exactly like a bound one: a session that
/// silently never times out would be worse than one with no timeout at all,
/// because the caller would believe the deadline was in force.
///
/// # Panics
///
/// Panics if called outside a Tokio runtime, since there would be nowhere to
/// schedule the deadline on.
pub fn session_router(config: WebSessionConfig) -> Result<(Router, SessionHandles)> {
    let wiring = build_session_wiring(config)?;
    wiring.arm_deadline(tokio::runtime::Handle::current());
    Ok((wiring.router, wiring.handles))
}

/// The pieces a wired-up session is made of, before it is bound to a listener.
struct SessionWiring {
    router: Router,
    handles: SessionHandles,
    state: SharedState,
}

impl SessionWiring {
    /// Schedule the deadline. Does nothing when the session is unbounded.
    fn arm_deadline(&self, runtime: tokio::runtime::Handle) {
        let Some(deadline) = self.state.deadline else {
            return;
        };
        let state = self.state.clone();
        runtime.spawn(async move {
            tokio::time::sleep_until(deadline.into()).await;
            state.time_out().await;
        });
    }
}

fn build_session_wiring(config: WebSessionConfig) -> Result<SessionWiring> {
    let WebSessionConfig {
        title,
        description,
        ui_ast,
        layout,
        data,
        schema,
        asset_provider,
        deadline,
        theme,
        draft,
    } = config;
    let validator = Arc::new(validator_for(&schema).context("failed to compile JSON schema")?);
    let (result_tx, result_rx) = oneshot::channel();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let shared = SharedState {
        title,
        description,
        ui_ast: Arc::new(ui_ast),
        layout: Arc::new(layout),
        data: Arc::new(Mutex::new(data)),
        schema: Arc::new(schema),
        formats: DocumentFormat::available_formats(),
        validator,
        finish_line: Arc::new(FinishLine {
            result: Mutex::new(Some(result_tx)),
            shutdown: Mutex::new(Some(shutdown_tx)),
        }),
        finished: Arc::new(AtomicBool::new(false)),
        asset_provider,
        deadline,
        theme: theme.map(Arc::new),
        draft: draft.map(Arc::new),
    };

    let mut router = Router::new()
        .route("/api/v1/session", get(get_session))
        .route("/api/v1/session/export", get(get_session_export))
        .route("/api/v1/schema", get(get_schema))
        .route("/api/v1/save", post(post_save))
        .route("/api/v1/exit", post(post_exit))
        .route("/api/v1/validate", post(post_validate))
        .route("/api/v1/preview", post(post_preview));

    // Registered only when there is a stylesheet to serve, so the advertised
    // capability and the routing table cannot disagree.
    if shared.theme.is_some() {
        router = router.route("/api/v1/theme.css", get(get_theme_css));
    }

    let router = router.fallback(static_assets).with_state(shared.clone());

    Ok(SessionWiring {
        router,
        handles: SessionHandles {
            result_rx: Some(result_rx),
            shutdown_rx: Some(shutdown_rx),
        },
        state: shared,
    })
}

pub struct SessionHandles {
    result_rx: Option<oneshot::Receiver<SessionOutcome>>,
    shutdown_rx: Option<oneshot::Receiver<()>>,
}

impl SessionHandles {
    pub fn into_parts(mut self) -> (oneshot::Receiver<SessionOutcome>, oneshot::Receiver<()>) {
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
    description: Option<String>,
    ui_ast: Arc<UiAst>,
    layout: Arc<UiLayout>,
    data: Arc<Mutex<Value>>,
    schema: Arc<Value>,
    formats: Vec<DocumentFormat>,
    validator: Arc<Validator>,
    finish_line: Arc<FinishLine>,
    finished: Arc<AtomicBool>,
    asset_provider: Arc<dyn WebAssetProvider>,
    deadline: Option<Instant>,
    theme: Option<Arc<Theme>>,
    draft: Option<Arc<SessionDraft>>,
}

impl SharedState {
    /// End the session because its deadline passed. Reports `TimedOut` rather
    /// than the data collected so far: a half-filled form is not an answer, and
    /// conflating the two would let a caller persist one as if it were real.
    ///
    /// The draft is deliberately left on disk. A deadline firing is exactly
    /// when an unfinished form is worth recovering.
    async fn time_out(&self) {
        if self.finished.swap(true, Ordering::SeqCst) {
            return;
        }
        self.finish_line.complete(SessionOutcome::TimedOut).await;
    }
}

struct FinishLine {
    result: Mutex<Option<oneshot::Sender<SessionOutcome>>>,
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
}

impl FinishLine {
    async fn complete(&self, outcome: SessionOutcome) {
        if let Some(tx) = self.result.lock().await.take() {
            let _ = tx.send(outcome);
        }
        if let Some(tx) = self.shutdown.lock().await.take() {
            let _ = tx.send(());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
#[cfg_attr(feature = "web-types", ts(export, export_to = "web/types/"))]
pub struct SessionResponse {
    /// The contract revision this payload follows. See [`API_VERSION`].
    pub api_version: String,
    /// Optional parts of the contract this session actually offers.
    pub capabilities: Vec<Capability>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub ui_ast: UiAst,
    #[cfg_attr(feature = "web-types", ts(type = "Record<string, unknown>"))]
    pub data: Value,
    pub formats: Vec<String>,
    pub layout: UiLayout,
    /// Milliseconds until the session is aborted, or `None` when unbounded.
    /// A duration rather than a timestamp so a client on a skewed clock (a
    /// phone on the LAN, say) still counts down correctly.
    #[cfg_attr(feature = "web-types", ts(type = "number | null"))]
    pub expires_in_ms: Option<u64>,
    /// Whether `data` came back from a draft rather than from the caller.
    /// The user typed it once already and is owed the explanation.
    pub draft_restored: bool,
}

async fn get_session(State(state): State<SharedState>) -> impl IntoResponse {
    Json(build_session(&state).await)
}

async fn get_session_export(State(state): State<SharedState>) -> impl IntoResponse {
    let payload = build_session(&state).await;
    (
        [(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"session.json\"",
        )],
        Json(payload),
    )
}

async fn build_session(state: &SharedState) -> SessionResponse {
    SessionResponse {
        api_version: API_VERSION.to_string(),
        capabilities: capabilities(state.theme.as_deref(), state.draft.as_deref()),
        title: state.title.clone(),
        description: state.description.clone(),
        ui_ast: (*state.ui_ast).clone(),
        data: state.data.lock().await.clone(),
        formats: state.formats.iter().map(|f| f.to_string()).collect(),
        layout: (*state.layout).clone(),
        expires_in_ms: remaining_ms(state.deadline),
        draft_restored: state.draft.as_ref().is_some_and(|draft| draft.restored),
    }
}

/// The schema the session was built from, for frontends that would rather
/// render it themselves than consume the UI AST.
async fn get_schema(State(state): State<SharedState>) -> impl IntoResponse {
    Json((*state.schema).clone())
}

async fn get_theme_css(State(state): State<SharedState>) -> Response<Body> {
    let css = state
        .theme
        .as_ref()
        .map(|theme| theme.css().to_string())
        .unwrap_or_default();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/css; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(css))
        .expect("static header values are valid")
}

#[derive(Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
#[cfg_attr(feature = "web-types", ts(export, export_to = "web/types/"))]
pub(crate) struct SaveRequest {
    #[cfg_attr(feature = "web-types", ts(type = "Record<string, unknown>"))]
    pub data: Value,
}

/// Checkpoint the session: hold the value, and put it somewhere that outlives
/// the process.
///
/// An in-memory-only save would be the more dangerous half of a promise — the
/// user reads "saved" and stops worrying, while the only copy is in a process
/// one crash away from gone.
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

    if let Some(draft) = state.draft.as_ref()
        && let Err(err) = draft.store.save(&req.data)
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("failed to write draft: {err:#}")})),
        );
    }
    *state.data.lock().await = req.data;
    (StatusCode::OK, Json(json!({"status": "saved"})))
}

#[derive(Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
#[cfg_attr(feature = "web-types", ts(export, export_to = "web/types/"))]
pub(crate) struct ExitRequest {
    #[cfg_attr(feature = "web-types", ts(type = "Record<string, unknown>"))]
    pub data: Value,
    #[serde(default = "crate::web::session::default_true")]
    pub commit: bool,
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
    // The session produced a value; whatever the caller does with it, the
    // draft has nothing left to protect.
    if let Some(draft) = state.draft.as_ref() {
        draft.store.discard();
    }
    state
        .finish_line
        .complete(SessionOutcome::Completed(final_value))
        .await;
    (StatusCode::OK, Json(json!({"status": "closing"})))
}

#[derive(Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
#[cfg_attr(feature = "web-types", ts(export, export_to = "web/types/"))]
pub(crate) struct ValidateRequest {
    #[cfg_attr(feature = "web-types", ts(type = "Record<string, unknown>"))]
    pub data: Value,
}

#[derive(Serialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
#[cfg_attr(feature = "web-types", ts(export, export_to = "web/types/"))]
pub(crate) struct ValidationResponse {
    pub ok: bool,
    pub errors: Vec<FieldError>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
#[cfg_attr(feature = "web-types", ts(export, export_to = "web/types/"))]
pub(crate) struct FieldError {
    pub pointer: String,
    pub message: String,
}

async fn post_validate(
    State(state): State<SharedState>,
    Json(req): Json<ValidateRequest>,
) -> impl IntoResponse {
    let mut errors = Vec::new();
    for error in state.validator.iter_errors(&req.data) {
        errors.push(FieldError {
            pointer: error.instance_path().to_string(),
            message: error.to_string(),
        });
    }
    Json(ValidationResponse {
        ok: errors.is_empty(),
        errors,
    })
}

#[derive(Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
#[cfg_attr(feature = "web-types", ts(export, export_to = "web/types/"))]
pub(crate) struct PreviewRequest {
    #[cfg_attr(feature = "web-types", ts(type = "Record<string, unknown>"))]
    pub data: Value,
    pub format: String,
    #[serde(default = "crate::web::session::default_true")]
    pub pretty: bool,
}

#[derive(Serialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
#[cfg_attr(feature = "web-types", ts(export, export_to = "web/types/"))]
pub(crate) struct PreviewResponse {
    pub payload: String,
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
    OutputOptions::new(format).with_pretty(pretty).render(value)
}

async fn static_assets(State(state): State<SharedState>, uri: OriginalUri) -> impl IntoResponse {
    if let Some(asset) = state.asset_provider.load(uri.path()) {
        let body = Body::from(asset.contents.into_owned());
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CACHE_CONTROL, "no-store")
            .header(header::CONTENT_TYPE, asset.mime)
            .body(body)
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
