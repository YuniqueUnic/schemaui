# schemaui-cli 使用指南

`schemaui-cli` 是 `schemaui` 库的官方命令行包装器。它接受 JSON Schema +
配置快照，启动交互式 TUI，并以任何启用的格式输出编辑后的文档。本指南与
`schemaui-cli/src/main.rs` 中的实际代码保持一致，以确保行为的可预测性。

## 1. 安装与运行

### 从源码运行

```bash
cargo run -p schemaui-cli -- --schema ./schema.json --config ./config.yaml
```

### 安装为二进制文件

```bash
cargo install --path schemaui-cli --locked
schemaui --help             # 二进制文件通过 clap 元数据命名为 `schemaui`
```

## 2. 执行流程

```
┌────────┐ args┌───────────────┐ schema/config ┌──────────────┐ result ┌────────────┐
│  clap  ├────▶│ InputSource   ├──────────────▶│ SchemaUI     ├──────▶│ io::output │
└────┬───┘     └─────────┬─────┘               │ (library)    │        └────┬───────┘
     │ diagnostics       │ format hint         └─────┬────────┘             │  writes
┌────▼─────────┐         │ DocumentFormat            │ validator            ▼  files/stdout
│Diagnostic    │◀────────┘ (extension or default)    │                      
│Collector     │                                     ▼                      
└──────────────┘                               Interactive UI
```

关键组件：

- **`InputSource`** – 解析文件、stdin 或内联规范。
- **`FormatHint`** – 检查扩展名并确保在解析前拒绝已禁用的格式。
- **`DiagnosticCollector`** –
  汇总每个输入/输出问题，如果出现任何错误则提前中止。
- **`SchemaUI`** – 与库消费者使用的相同运行时；CLI 仅负责连接参数。

## 3. 输入模式

| 标志                  | 行为                                              | 注意事项                                         |
| --------------------- | ------------------------------------------------- | ------------------------------------------------ |
| `-s, --schema <SPEC>` | 文件路径、字面 JSON/YAML/TOML 或 `-` 表示 stdin。 | 如果路径不存在，CLI 会将参数视为内联文本。       |
| `-c, --config <SPEC>` | 与 `--schema` 相同的语义。                        | 可选；省略时，如果存在配置值，则从中推断默认值。 |

代码强制执行的约束：

- `stdin` 只能使用一次，因此 `--schema -` 和 `--config -` 不能同时使用。
- 如果仅提供 `--config`，CLI 会调用 `schema_from_data_value` 来构建带有默认值的
  schema。

## 4. 输出与持久化

- `-o, --output <DEST>` 可重复；传递 `-` 可在文件之外包含
  stdout。扩展名（`.json`、`.yaml`、`.toml`）驱动 `DocumentFormat`。
- 当未设置目的地时，CLI 会写入 `/tmp/schemaui.json`，除非传递了 `--no-temp-file`
  或 `--temp-file <PATH>` 覆盖回退路径。
- `--no-pretty` 切换紧凑序列化；默认为美化输出。
- `--force`/`--yes` 允许覆盖现有文件。如果没有此标志，当目标文件已存在时，CLI
  会拒绝运行。

内部由 `io::output::OutputOptions`
提供支持，因此嵌入项目可以重用完全相同的序列化逻辑。

## 5. 参数参考

| 标志                  | 描述                           | 代码钩子                            |
| --------------------- | ------------------------------ | ----------------------------------- |
| `-o, --output <DEST>` | 追加目标（`-` 写入 stdout）。  | `build_output_options`              |
| `--title <TEXT>`      | 覆盖 TUI 标题栏。              | `SchemaUI::with_title`              |
| `--temp-file <PATH>`  | 未设置目标时的自定义回退文件。 | `build_output_options`              |
| `--no-temp-file`      | 完全禁用回退文件行为。         | `build_output_options`              |
| `--no-pretty`         | 输出紧凑的 JSON/TOML/YAML。    | `OutputOptions::with_pretty(false)` |
| `--force`, `--yes`    | 允许覆盖文件。                 | `ensure_output_paths_available`     |

## 6. 使用示例

### Schema + config + 双输出

```bash
schemaui \
  --schema ./schema.json \
  --config ./config.yaml \
  -o - \
  -o ./edited.toml
```

### 仅 config（推断 schema）

```bash
cat defaults.yaml | schemaui --config - --output ./edited.json
```

### 内联 schema 以避免双 stdin

```bash
schemaui --schema '{"type":"object","properties":{"host":{"type":"string"}}}' \
    --config ./config.json -o -
```

## 7. 诊断与错误

- **汇总报告** – `DiagnosticCollector` 存储每个输入/输出问题（冲突的
  stdin、禁用的格式、现有文件），并在以非零代码退出之前将它们打印为编号列表。
- **格式推断** – `resolve_format_hint`
  在扩展名需要已禁用的功能时发出警告（例如，没有 `yaml` 功能时使用
  `.yaml`）。CLI 立即停止，而不是稍后在序列化期间失败。
- **运行时错误** – 其他所有内容都通过 `color-eyre`
  冒泡，因此堆栈跟踪包含上下文，如 `failed to parse config as yaml` 或
  `failed to compile JSON schema`。

## 8. 库互操作

CLI 是 `SchemaUI` 的薄包装器：

```rust
let mut ui = SchemaUI::new(schema);
if let Some(title) = cli.title.as_ref() {
    ui = ui.with_title(title.clone());
}
if let Some(defaults) = config_value.as_ref() {
    ui = ui.with_default_data(defaults);
}
if let Some(options) = output_settings {
    ui = ui.with_output(options);
}
ui.run()?;
```

这意味着嵌入项目可以逐字重现 CLI 流程，或完全替换前端（例如，构建自定义 CLI 或
GUI），同时重用相同的 I/O 和验证管道。

## 9. 功能标志

| 功能           | 效果                                      |
| -------------- | ----------------------------------------- |
| `json`（默认） | 启用 JSON 解析/序列化。始终开启。         |
| `yaml`（默认） | 通过 `serde_yaml` 添加 YAML 解析/序列化。 |
| `toml`（可选） | 通过 `toml` 添加 TOML 解析/序列化。       |
| `all_formats`  | 便利功能：启用 `json`、`yaml` 和 `toml`。 |

`DocumentFormat::available_formats()` 遵循相同的功能矩阵，因此 CLI
和宿主应用程序都会自动反映构建时的能力。

## 10. 操作提示

- 当 schema 和 config 都需要 stdin
  时，将一个流作为字面文本传递（不存在的路径被视为内联）；可执行文件只读取 stdin
  一次。
- 为输出提供明确的扩展名——格式推断使用第一个文件的后缀，跨文件的不匹配将被拒绝。
- 将 `-o -` 与文件输出结合使用，可以在写入磁盘的同时将结果传输到 CI 日志中。

使用这些模式，您可以在 CI/CD 管道或开发者工具中自信地编写 `schemaui-cli` 脚本。
