## schemaui

**schemaui** 是一个将 JSON Schema 动态映射为终端用户界面（TUI）的 Rust
库。它能够解析复杂的 Schema（包含 `$ref`、`oneOf`/`anyOf`、多层级
Section、数组、Key/Value
结构等），构建可聚焦的表单树，并在用户每一次编辑后立即通过
`jsonschema::Validator` 进行校验，确保配置数据始终有效。

### 快速上手

```toml
[dependencies]
schemaui = "0.1.1"
serde_json = "1"
```

```rust
use schemaui::SchemaUI;
use serde_json::json;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Service Runtime",
        "type": "object",
        "properties": {
            "metadata": {
                "type": "object",
                "properties": {
                    "serviceName": {"type": "string"},
                    "environment": {"type": "string", "enum": ["dev", "staging", "prod"]}
                },
                "required": ["serviceName"]
            },
            "runtime": {
                "type": "object",
                "properties": {
                    "http": {
                        "type": "object",
                        "properties": {
                            "host": {"type": "string", "default": "0.0.0.0"},
                            "port": {"type": "integer", "minimum": 1024, "maximum": 65535}
                        }
                    }
                }
            }
        },
        "required": ["metadata", "runtime"]
    });

    let value = SchemaUI::new(schema).with_title("SchemaUI Demo").run()?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
```

### 工程结构

- **schemaui（库 crate）**：暴露 `SchemaUI`、`UiOptions` 以及入口/出口层
  API，负责 JSON Schema → Form → TUI 的全流程。源码位于
  `src/`，单元/集成测试集中在 `tests/` 目录。
- **schemaui-cli（命令行工具）**：独立 crate（`schemaui-cli/`），仅依赖
  `schemaui` 库。CLI 在解析参数后调用库 API，支持 schema/配置文件、inline
  文本、STDIN 以及多种输出通道。
- **examples/**：示例演示如何从 Rust 代码直接构造 Schema 并启动 TUI。

### 入口 / 出口层

- **统一入口 (`io::input`)**：`parse_document_str` 支持 JSON/TOML/YAML →
  `serde_json::Value`；`schema_from_data_value/str` 会根据样本值自动推断 JSON
  Schema，并为每个节点写入 `default`。
- **样本合并 (`schema_with_defaults`)**：如果用户已经有正式 JSON Schema，可用
  `schema_with_defaults`、`SchemaUI::from_schema_and_data` 或
  `SchemaUI::with_default_data` 将 JSON/TOML/YAML 快照的值注入
  Schema。`DefaultApplier` 会递归穿透
  `properties`、`patternProperties`、`additionalProperties`、`dependencies` 与
  `$ref`，保证真实默认值与校验关键字共存。
- **特性开关**：`json` 始终启用；`yaml`/`toml` 通过 feature 选择，或直接启用
  `all_formats` 一次获得全部序列化器。
- **统一出口 (`io::output`)**：`OutputOptions` 控制 `DocumentFormat`、是否
  pretty，以及 `OutputDestination`（`Stdout`
  或文件列表）。`schemaUI.with_output(...)` 会在 `App::run`
  结束后自动将结果写入配置的目标。

```rust
use schemaui::{DocumentFormat, OutputDestination, OutputOptions, SchemaUI};

fn run_from_file(raw: &str) -> color_eyre::Result<()> {
    let ui = SchemaUI::from_data_str(raw, DocumentFormat::Yaml)?
        .with_title("Runtime Config")
        .with_output(
            OutputOptions::new(DocumentFormat::Toml)
                .with_pretty(true)
                .with_destinations(vec![
                    OutputDestination::Stdout,
                    OutputDestination::file("./config.out.toml"),
                ]),
        );
    ui.run()?;
    Ok(())
}
```

### 支持的 JSON Schema 结构

- **多 Root / Section**：顶层属性映射为 Root Tab，嵌套对象递归展开为 Section
  树，自动生成层级标题。
- **嵌套字段与列表**：对象/数组可无限嵌套；标量数组以逗号列表呈现，可重复的
  composite/kv 列表拥有独立的条目面板与 overlay。
- **`$ref` 链与 JSON Pointer**：`schema::resolver`
  会在布局之前解析所有引用，因此引用字段与 inline
  定义具有完全一致的渲染与验证行为。
- **`oneOf` / `anyOf`**：渲染为 Variant 选择器，选择后会打开
  overlay，根据所选模式实时验证并同步回主表单。
- **`patternProperties` / `additionalProperties:false`**：映射为 Key/Value
  编辑器或受限对象；所有关键字（类型、枚举、范围、长度等）在 UI 中即时反馈。

### 映射与校验管线

1. **schema::loader** 读取 JSON Schema 并生成 `RootSchema`。
2. **schema::resolver** 提前解析 `$ref` 与指针引用，确保后续阶段拿到完全展开的
   `SchemaObject`。
3. **schema::layout::build_form_schema** 根据元信息构造 `FormSchema`：
   - 把顶层属性转换为 RootSection。
   - 将嵌套对象展开为扁平 Section 列表，记录 `depth` 以渲染层级缩进。
   - 根据实例类型映射到 `FieldKind`（文本、枚举、复合、数组、KV 等）。
4. **form::state::FormState** 基于 `FormSchema` 初始化 `FieldState`，维护
   root/section/field 的焦点索引与 dirty/错误状态。
5. **form::reducers::FormEngine** 接收 `FormCommand`，在 `FieldEdited` 时使用
   `jsonschema::Validator` 校验完整 JSON，错误信息映射回对应字段。
6. **app::runtime::App** 驱动事件循环、状态栏与 overlay；`InputRouter`
   将按键映射为 `KeyAction` → `CommandDispatch`。
7. **presentation::components** 使用 Ratatui 渲染根标签、Section、Field
   列表、弹窗、状态栏与 overlay。

### TUI 组成与模块化

- **Root Tabs**：展示 Schema 的根节点，`Ctrl+J / Ctrl+L` 或 `Alt+Shift+[ / ]`
  切换。Root/Section 的焦点索引用统一算法循环前进/后退。
- **Section Tabs**：针对当前 Root 的所有 Section
  展示面包屑式标签，`Ctrl+Tab`/`Ctrl+Shift+Tab` 在 Section 之间跳转。
- **Field 列表**：`fields.rs`
  负责渲染标签、值摘要、类型信息与错误提示；`FieldState` 中的
  `display_value`/`meta_line` 控制具体样式。
- **Overlay**：`app::runtime::overlay` 将 composite、list、kv、array
  编辑封装为独立表单，包含自有 validator、指引文字与列表侧边栏，只有 `Ctrl+S` 或
  `Esc`（二次确认）才会退出。
- **状态/帮助栏**：`status::StatusLine` 根据当前上下文展示 dirty
  状态、校验结果、快捷键提示。

### 快捷键设计

| 上下文    | 快捷键                                   | 功能                                           |
| --------- | ---------------------------------------- | ---------------------------------------------- |
| 全局导航  | `Tab` / `Shift+Tab`                      | 在字段之间移动（越界时在 root/section 间循环） |
|           | `Ctrl+Tab` / `Ctrl+Shift+Tab`            | 在 Section 间跳转                              |
|           | `Ctrl+J` / `Ctrl+L` 或 `Alt+Shift+[ / ]` | 切换 Root Tab                                  |
| 字段交互  | `Enter`                                  | 打开枚举/变体弹窗或 overlay                    |
|           | `Esc`                                    | 关闭弹窗或 overlay（脏数据需二次确认）         |
| 持久化    | `Ctrl+S`                                 | 保存并重新校验整个表单                         |
|           | `Ctrl+Q`                                 | 退出应用（脏数据时提示）                       |
| 列表/映射 | `Ctrl+N` / `Ctrl+D`                      | 添加 / 删除条目                                |
|           | `Ctrl+←` / `Ctrl+→`                      | 选择前一/后一条目                              |
|           | `Ctrl+↑` / `Ctrl+↓`                      | 调整条目顺序                                   |
| Overlay   | `Ctrl+E`                                 | 从主界面进入 overlay 编辑                      |
|           | `Ctrl+S` / `Esc` / `Tab`                 | Overlay 内保存 / 关闭 / 导航                   |

### 测试

项目测试集中于 `tests/`
目录，按照模块划分：`tests/form`、`tests/schema`、`tests/app`、`tests/presentation`。每个源码模块通过
`include!(...)` 引入对应测试文件，即便内部函数是 `pub(super)` 也能被验证：

```bash
cargo test
```

### CLI Usage

The built-in `schemaui` binary wraps the library in a configurable workflow:

```bash
schemaui \
  --schema ./schema.json \
  --config ./config.yaml \
  --output-format json \
  --stdout \
  --output ./config.out.json
```

- `--schema` / `--config` accept file paths or `-` (stdin) and support
  JSON/TOML/YAML. When both are supplied, the schema stays authoritative while
  the config snapshot seeds defaults. Passing literal payloads is also possible
  through `--schema-inline` / `--config-inline`, which keeps stdin free for the
  other stream.
- To avoid double reads, only one of schema/config may use `-` simultaneously
  (use inline flags when both need piped content).
- Formats can be inferred from file extensions or forced via `--schema-format` /
  `--config-format`.
- Outputs are routed through the exit layer: mix `--stdout`, repeated
  `--output <path>`, or rely on the default temp file `/tmp/schemaui.yaml`.
  Disable the fallback with `--no-temp-file` or relocate it via `--temp-file`.
- Titles and other options chain directly through `SchemaUI`, so the CLI mirrors
  the library flow: load → merge defaults → render TUI → emit the edited
  configuration in the requested format(s).
- CLI 的详细使用说明见
  [`docs/cli_usage.md`](docs/cli_usage.md)：该文档覆盖参数列表、输入/输出策略、错误处理与示例命令。一般情况下，可在
  workspace 中运行 `cargo run -p schemaui-cli -- ...`，或执行
  `cargo install --path schemaui-cli` 安装独立二进制。

### 许可证

- Apache License 2.0（[LICENSE-APACHE](LICENSE-APACHE) /
  <http://www.apache.org/licenses/LICENSE-2.0>）
- MIT License（[LICENSE](LICENSE) / <http://opensource.org/licenses/MIT>）

### 参与贡献

1. 先 `cargo fmt && cargo check` 确保风格与编译通过。
2. 在 `tests/<module>/` 下补充或更新对应测试。
3. 提交 PR 时说明设计动机，并保持 KISS / SOLID 的模块化原则。
