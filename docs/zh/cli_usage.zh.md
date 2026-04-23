# schemaui-cli 使用指南

`schemaui-cli` 是 `schemaui` 库的官方命令行包装器。它接受 JSON Schema +
配置快照；当未显式指定模式子命令时默认启动交互式 TUI，同时也暴露显式的
`completion`、`tui`、`web`、`tui-snapshot`、`web-snapshot` 子命令。本指南与
`schemaui-cli/src/main.rs` 中的实际代码保持一致，以确保行为可预测。

## 1. 安装与运行

### 从源码运行

```bash
cargo run -p schemaui-cli -- tui --schema ./schema.json --config ./config.yaml
```

### 安装为二进制文件

```bash
cargo install schemaui-cli
schemaui --help
```

如果省略模式子命令，`schemaui` 会回退到默认的 TUI 流程，因此下面两种写法等价：

```bash
schemaui --schema ./schema.json
schemaui tui --schema ./schema.json
```

### Shell completion

`schemaui-cli` 现在内置了基于 `argh_complete` 的补全脚本生成能力：

```bash
schemaui completion bash > ~/.local/share/bash-completion/completions/schemaui
schemaui completion zsh > ~/.zfunc/_schemaui
schemaui completion fish > ~/.config/fish/completions/schemaui.fish
schemaui completion nushell > ~/.config/nushell/completions/schemaui.nu
```

当前官方支持的 shell 是 `bash`、`zsh`、`fish`、`nushell`。PowerShell
还没有接入，因为上游 `argh_complete` 目前没有提供 PowerShell generator。

## 2. 执行流程

```
┌────────┐ args┌───────────────┐ schema/config ┌──────────────┐ result ┌────────────┐
│  argh  ├────▶│ InputSource   ├──────────────▶│ SchemaUI     ├──────▶│ io::output │
└────┬───┘     └─────────┬─────┘               │ (library)    │        └────┬───────┘
     │ diagnostics       │ format hint         └─────┬────────┘             │  writes
┌────▼─────────┐         │ DocumentFormat            │ validator            ▼  files/stdout
│Diagnostic    │◀────────┘ (extension or default)    │
│Collector     │                                     ▼
└──────────────┘                               Interactive UI
```

关键组件：

- **`schema_source`**：统一处理显式 `--schema`、配置文件内声明、远程/本地 schema
  加载，以及最终的兜底推断。
- **`FormatHint`**：根据扩展名检查格式，并在真正解析前拦截被 feature
  关闭的格式。
- **`DiagnosticCollector`**：聚合每个输入/输出问题，出错时统一中止。
- **`completion`**：基于同一套命令树渲染 shell completion 脚本。
- **`SchemaUI`**：CLI 复用的仍然是库中的同一套运行时能力。

## 3. 输入模式

| 标志                  | 行为                                                                        | 注意事项                                                         |
| --------------------- | --------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| `-s, --schema <SPEC>` | 本地路径、`file://`、`http(s)://`、内联 JSON/YAML/TOML，或 `-` 表示 stdin。 | 显式 schema 的优先级高于 `--config` 中发现的任何 schema 声明。   |
| `-c, --config <SPEC>` | 与 `--schema` 相同的加载语义。                                              | 可选；省略 `--schema` 时，CLI 会先尝试配置内声明，再做兜底推断。 |

代码层强制执行的约束：

- `stdin` 只能使用一次，因此 `--schema -` 和 `--config -` 不能同时使用。
- 优先级为：`--schema` > 配置声明 > 推断 schema。
- 相对本地声明路径会相对 `--config` 所在目录解析。
- 若配置来自内联文本或 stdin，相对路径会相对当前工作目录解析。
- HTTP(S) schema 加载由 `schemaui-cli` 的 `remote-schema` feature 控制。CLI
  默认启用； `schemaui` 库默认关闭远程 schema 加载。
- `schemaui` 库默认 feature 为 `tui + json`；CLI 默认 feature 为
  `full + remote-schema`。
- `json`、`yaml`、`toml` 都是真实的 feature
  gate；三者全关时，编译会直接报清晰错误。

### 配置 schema 自动检测

当只传 `--config` 时，`schemaui-cli` 会先扫描配置中的 schema 声明，再回退到
`schema_from_data_value`。

支持的声明格式：

- **JSON**：根级 `$schema`
- **TOML**：`#:schema https://example.com/schema.json`
- **YAML**：`# yaml-language-server: $schema=...`
- **YAML 兜底**：`# @schema ...`

对于 JSON，根级 `$schema`
被视为元数据，因此在内存中的默认值会先移除该字段，再参与 校验与输出。

## 4. 输出与持久化

- `-o, --output <DEST>` 可重复传递；`-` 表示写到 stdout。
- 输出扩展名（`.json`、`.yaml`、`.toml`）决定 `DocumentFormat`。
- 当未指定任何目标时，CLI 默认写到 stdout；如果你明确想走回退文件，再显式传
  `--temp-file <PATH>`。
- `--no-pretty` 切换为紧凑序列化；默认是 pretty 输出。
- `--force` / `--yes` 允许覆盖已有文件；否则遇到已存在目标会直接拒绝执行。

这些行为都由 `io::output::OutputOptions`
驱动，因此嵌入方可以复用完全相同的输出逻辑。

## 5. 参数参考

| 标志                  | 描述                                    | 代码钩子                            |
| --------------------- | --------------------------------------- | ----------------------------------- |
| `-o, --output <DEST>` | 追加输出目标（`-` 写到 stdout）。       | `build_output_options`              |
| `--title <TEXT>`      | 覆盖 TUI 标题栏。                       | `SchemaUI::with_title`              |
| `--temp-file <PATH>`  | 未设置 `--output` 时，显式写到该文件。  | `build_output_options`              |
| `--no-temp-file`      | 兼容性 no-op；默认行为本来就是 stdout。 | `build_output_options`              |
| `--no-pretty`         | 输出紧凑 JSON/TOML/YAML。               | `OutputOptions::with_pretty(false)` |
| `--force`, `--yes`    | 允许覆盖现有文件。                      | `ensure_output_paths_available`     |

## 6. 使用示例

### schema + config + 双输出

```bash
schemaui tui \
  --schema ./schema.json \
  --config ./config.yaml \
  -o - \
  -o ./edited.toml
```

### 仅 config（推断 schema）

```bash
cat defaults.yaml | schemaui --config - --output ./edited.json
```

### 仅 config，但使用文件头中的 schema 声明

```bash
schemaui web --config ./config.yaml
```

```yaml
# yaml-language-server: $schema=./schema.json
name: api
port: 8080
```

### 显式 schema 覆盖文件头声明

```bash
schemaui \
  --schema https://example.com/runtime.schema.json \
  --config ./config.toml
```

### 用内联 schema 避免双 stdin

```bash
schemaui tui \
  --schema '{"type":"object","properties":{"host":{"type":"string"}}}' \
  --config ./config.json -o -
```

### 生成 completion 脚本

```bash
schemaui completion bash
```

## 7. 诊断与错误

- **聚合报告**：`DiagnosticCollector` 会把冲突
  stdin、格式被禁用、输出文件已存在等问题 聚合成编号列表，再以非零状态退出。
- **格式推断**：当扩展名要求的格式 feature 没开时，CLI 会在解析前直接停止。
- **运行时错误**：其余错误经由 `color-eyre` 输出上下文，例如
  `failed to parse config as yaml` 或 `failed to compile JSON schema`。

## 8. 库互操作

CLI 只是 `SchemaUI` 的一层薄包装：

```rust
let mut ui = if let Some(defaults) = config_value {
    SchemaUI::new(defaults).with_schema(schema)
} else {
    SchemaUI::from_schema(schema)
};
if let Some(title) = cli.title.as_ref() {
    ui = ui.with_title(title.clone());
}
let value = ui.run_tui()?;
if let Some(options) = output_settings.as_ref() {
    options.write(&value)?;
}
```

这意味着嵌入项目既可以原样复刻 CLI 行为，也可以替换前端，只复用同一套 I/O
与校验链路。

## 9. Feature Flags

| Feature                          | 作用                                           |
| -------------------------------- | ---------------------------------------------- |
| `json`                           | 启用 JSON 解析/序列化以及 JSON 格式探测。      |
| `yaml`                           | 通过 `serde_yaml` 启用 YAML 解析/序列化。      |
| `toml`                           | 通过 `toml` 启用 TOML 解析/序列化。            |
| `web`                            | 启用浏览器 UI 子命令以及内嵌 HTTP 运行时。     |
| `full`                           | 便利组合：启用 `json`、`yaml`、`toml`、`web`。 |
| `remote-schema`                  | 通过 `reqwest` 启用 HTTP(S) schema 加载。      |
| 默认值（`full + remote-schema`） | 对最终 CLI 用户开放的开箱即用构建。            |

`DocumentFormat::available_formats()` 会自动反映同一套 feature 矩阵。至少保留
`json`、`yaml`、`toml` 中的一个。

## 10. 操作提示

- 如果 schema 和 config 都想走
  stdin，就把其中一个改为内联文本；可执行文件只会消费一次 stdin。
- 输出最好总是带扩展名；格式推断使用第一个文件的后缀，跨文件冲突会被直接拒绝。
- 可以把 `-o -` 和文件输出同时使用，这样既能进 CI 日志，也能真正落盘。

## 11. Web 模式

启用 `web` feature（CLI 默认启用）后，`schemaui-cli` 会暴露 `web` 子命令：

```bash
schemaui web \
  --schema ./schema.json \
  --config ./defaults.json \
  --host 127.0.0.1 --port 0 \
  -o -
```

该命令与 TUI 共用同一套 schema/config 加载链路，然后调用
`schemaui::web::session::bind_session` 启动临时 HTTP 服务并挂载静态资源与 API。
终端会打印绑定地址；如果端口传 `0`，则自动选择随机空闲端口。

- 点击 **Save** 可以在不退出页面的情况下持久化当前编辑内容。
- 点击 **Save & Exit** 会关闭临时服务，并把最终结果写到配置好的输出目标。

`web` 子命令额外参数：

| 标志            | 描述                                 |
| --------------- | ------------------------------------ |
| `--host <IP>`   | 临时 HTTP 服务绑定地址。             |
| `--port <PORT>` | 绑定端口；`0` 表示请求随机空闲端口。 |

其他参数（`--schema`、`--config`、`--output` 等）与 TUI 模式完全一致。
