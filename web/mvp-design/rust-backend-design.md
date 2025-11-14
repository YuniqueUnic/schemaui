// ============================================================================
// schemaui/src/web/mod.rs - Web 功能模块入口
// ============================================================================

pub mod server;
pub mod assets;
pub mod api;

pub use server::WebServer;

// ============================================================================
// schemaui/src/web/assets.rs - 嵌入前端资源（完全离线）
// ============================================================================

use rust_embed::RustEmbed;

/// 嵌入所有前端资源到二进制中，实现完全离线
#[derive(RustEmbed)]
#[folder = "web-ui/dist"]
pub struct WebAssets;

impl WebAssets {
    /// 获取单页应用的 HTML
    pub fn index_html() -> String {
        // 这里会包含编译后的完整 HTML（包含所有 JS/CSS）
        String::from_utf8(
            Self::get("index.html")
                .expect("index.html must exist")
                .data
                .into_owned()
        ).expect("index.html must be valid UTF-8")
    }
}

// ============================================================================
// schemaui/src/web/server.rs - Web 服务器实现
// ============================================================================

use axum::{
    Router,
    routing::{get, post},
    extract::{State, Json},
    response::{Html, IntoResponse, Response},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use tower_http::cors::CorsLayer;

use crate::web::assets::WebAssets;
use crate::web::api::{ApiState, ValidationRequest, ValidationResponse, SaveRequest};

/// Web 服务器配置
#[derive(Debug, Clone)]
pub struct WebConfig {
    pub host: String,
    pub port: u16,
    pub auto_open: bool,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            auto_open: true,
        }
    }
}

/// Web 服务器实例
pub struct WebServer {
    config: WebConfig,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl WebServer {
    pub fn new(config: WebConfig) -> Self {
        Self {
            config,
            shutdown_tx: None,
        }
    }

    /// 启动 Web 服务器
    pub async fn run(
        &mut self,
        schema: serde_json::Value,
        initial_data: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        // 创建共享状态
        let state = Arc::new(ApiState {
            schema: Arc::new(schema),
            data: Mutex::new(initial_data.unwrap_or(serde_json::json!({}))),
            result: Mutex::new(None),
        });

        // 构建路由
        let app = Router::new()
            // 静态资源
            .route("/", get(serve_index))
            // API 端点
            .route("/api/schema", get(get_schema))
            .route("/api/validate", post(validate_data))
            .route("/api/save", post(save_data))
            .route("/api/exit", post(exit_server))
            // CORS 支持
            .layer(CorsLayer::permissive())
            .with_state(state.clone());

        // 绑定地址
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        
        println!("🚀 SchemaUI Web server running at http://{}", addr);
        
        if self.config.auto_open {
            let url = format!("http://{}", addr);
            let _ = open::that(url);
        }

        // 运行服务器，等待关闭信号
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
            })
            .await?;

        // 返回最终结果
        let result = state.result.lock().unwrap();
        Ok(result.clone().unwrap_or(serde_json::json!({})))
    }

    /// 关闭服务器
    pub fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

// ============================================================================
// 路由处理函数
// ============================================================================

/// 提供单页应用
async fn serve_index() -> Html<String> {
    Html(WebAssets::index_html())
}

/// 获取 Schema
async fn get_schema(State(state): State<Arc<ApiState>>) -> Json<serde_json::Value> {
    Json((*state.schema).clone())
}

/// 验证数据
async fn validate_data(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<ValidationRequest>,
) -> Json<ValidationResponse> {
    use jsonschema::JSONSchema;

    let compiled = match JSONSchema::compile(&state.schema) {
        Ok(s) => s,
        Err(e) => {
            return Json(ValidationResponse {
                valid: false,
                errors: vec![format!("Schema compilation error: {}", e)],
            });
        }
    };

    let result = compiled.validate(&req.data);
    
    match result {
        Ok(_) => Json(ValidationResponse {
            valid: true,
            errors: vec![],
        }),
        Err(errors) => {
            let error_messages: Vec<String> = errors
                .map(|e| format!("{}: {}", e.instance_path, e))
                .collect();
            
            Json(ValidationResponse {
                valid: false,
                errors: error_messages,
            })
        }
    }
}

/// 保存数据
async fn save_data(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<SaveRequest>,
) -> impl IntoResponse {
    let mut data = state.data.lock().unwrap();
    *data = req.data.clone();
    
    StatusCode::OK
}

/// 退出服务器
async fn exit_server(
    State(state): State<Arc<ApiState>>,
    Json(data): Json<serde_json::Value>,
) -> impl IntoResponse {
    // 保存最终结果
    let mut result = state.result.lock().unwrap();
    *result = Some(data);
    
    // 触发服务器关闭
    // 注意：实际实现中需要通过 channel 发送关闭信号
    
    StatusCode::OK
}

// ============================================================================
// schemaui/src/web/api.rs - API 数据结构
// ============================================================================

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// API 共享状态
pub struct ApiState {
    pub schema: Arc<serde_json::Value>,
    pub data: Mutex<serde_json::Value>,
    pub result: Mutex<Option<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct ValidationRequest {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ValidationResponse {
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveRequest {
    pub data: serde_json::Value,
}

// ============================================================================
// schemaui/Cargo.toml - 添加依赖
// ============================================================================

/*
[dependencies]
# 现有依赖...
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
jsonschema = "0.18"

# Web 功能依赖
axum = { version = "0.7", optional = true }
tokio = { version = "1", features = ["full"], optional = true }
tower-http = { version = "0.5", features = ["cors"], optional = true }
rust-embed = { version = "8.0", optional = true }
open = { version = "5.0", optional = true }

[features]
default = ["tui"]
tui = ["ratatui", "crossterm"]
web = ["axum", "tokio", "tower-http", "rust-embed", "open"]
*/

// ============================================================================
// schemaui-cli/src/commands/web.rs - CLI Web 命令实现
// ============================================================================

use clap::Args;
use schemaui::web::{WebServer, WebConfig};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct WebCommand {
    /// Path to JSON Schema file
    #[arg(short, long)]
    schema: PathBuf,

    /// Initial data file (optional)
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Output destination (- for stdout)
    #[arg(short, long, default_value = "-")]
    output: String,

    /// Server host
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Server port
    #[arg(long, default_value = "3000")]
    port: u16,

    /// Don't auto-open browser
    #[arg(long)]
    no_open: bool,
}

impl WebCommand {
    pub async fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 加载 Schema
        let schema_content = std::fs::read_to_string(&self.schema)?;
        let schema: serde_json::Value = serde_json::from_str(&schema_content)?;

        // 加载初始数据（如果有）
        let initial_data = if let Some(config_path) = &self.config {
            let content = std::fs::read_to_string(config_path)?;
            Some(serde_json::from_str(&content)?)
        } else {
            None
        };

        // 配置 Web 服务器
        let config = WebConfig {
            host: self.host.clone(),
            port: self.port,
            auto_open: !self.no_open,
        };

        // 运行服务器并等待结果
        let mut server = WebServer::new(config);
        let result = server.run(schema, initial_data).await?;

        // 输出结果
        let output_json = serde_json::to_string_pretty(&result)?;
        
        if self.output == "-" {
            println!("{}", output_json);
        } else {
            std::fs::write(&self.output, output_json)?;
            eprintln!("✅ Output written to: {}", self.output);
        }

        Ok(())
    }
}

// ============================================================================
// 前端构建脚本 - build.sh
// ============================================================================

/*
#!/bin/bash
# web-ui/build.sh - 构建完全离线的单页应用

set -e

echo "🔨 Building SchemaUI Web Interface..."

# 1. 使用 Vite 或类似工具构建 React 应用
cd web-ui
npm run build

# 2. 内联所有资源到单个 HTML 文件
# 使用 vite-plugin-singlefile 或手动内联

# 3. 输出到 dist 目录
echo "✅ Build complete! Output: web-ui/dist/index.html"

# 构建后的 HTML 应该包含：
# - 内联的所有 JavaScript
# - 内联的所有 CSS
# - 内联的语法高亮库（Prism.js 或 Highlight.js）
# - 无任何外部依赖或 CDN 引用
*/

// ============================================================================
// 使用示例
// ============================================================================

/*
# 1. 启动 Web 界面
cargo run -p schemaui-cli -- web --schema schema.json --port 3000

# 2. 带初始数据启动
cargo run -p schemaui-cli -- web --schema schema.json --config config.json

# 3. 输出到文件而非 stdout
cargo run -p schemaui-cli -- web --schema schema.json -o output.json

# 4. 自定义主机和端口
cargo run -p schemaui-cli -- web --schema schema.json --host 0.0.0.0 --port 8080

# 5. 不自动打开浏览器
cargo run -p schemaui-cli -- web --schema schema.json --no-open
*/