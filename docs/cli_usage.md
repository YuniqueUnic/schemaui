# schemaui-cli 使用指南

`shemaui-cli` 是对 `schemaui` 库的官方命令行封装，可在无需编写 Rust
代码的情况下，把 JSON Schema + 配置样本渲染为交互式
TUI，并导出编辑结果。本指南详述 CLI
的安装、输入输出模式、参数说明以及底层逻辑映射，便于在
CI、脚本或运维场景中快速集成。

## 1. 安装与运行

### 从源码运行

```bash
# 在本仓库根目录
cargo run -p schemaui-cli -- --schema ./schema.json --config ./config.yaml
```

### 安装为可执行文件

```bash
cargo install --path schemaui-cli --locked
schemaui-cli --help
```

> 默认二进制名称为 `schemaui-cli`，若希望覆盖系统现有 `schemaui` 命令，可使用
> `cargo install --path schemaui-cli --bin schemaui`。

## 2. 输入流程

CLI 通过 `schemaui::io::input` 提供的工具加载 Schema 和配置样本：

1. **Schema 来源**
   - `--schema <path>`：从文件或 `-`(stdin) 读取 JSON/TOML/YAML Schema。
   - `--schema-inline <json>`：直接传入内联文本；常见于 shell heredoc 或脚本。
2. **配置样本**
   - `--config <path>`：读取用户当前配置（JSON/TOML/YAML）。
   - `--config-inline <json>`：内联配置。

> 限制：`--schema -` 与 `--config -` 不可同时出现（stdin
> 仅能读取一次）。如需同时传输，可让其中之一使用 `--*-inline`。

解析完成后：

- 若仅提供配置（无 Schema），CLI 会调用 `schema_from_data_value` 推断
  Schema，并把所有值写入 `default`；
- 若同时提供 Schema 与配置，`schema_with_defaults` 会在
  `$ref`、`properties`、`patternProperties` 等节点递归注入默认值，确保 TUI
  载入时即显示真实数据。

## 3. 输出与持久化

`schemaui-cli` 使用 `OutputOptions` 将 TUI 的最终结果写回：

- `--stdout`：将 JSON/TOML/YAML 输出到标准输出。
- `--output <path>`：可重复使用，写入多个文件；会自动在末尾追加换行并 flush。
- `--output-format <json|yaml|toml>`：控制序列化格式，默认 JSON。
- `--no-pretty`：关闭美化输出，生成单行紧凑文本。
- 当未配置任何目的地时，CLI 会写入 `/tmp/schemaui.yaml`（或通过 `--temp-file`
  指定的路径）；可用 `--no-temp-file` 禁用该回退。

底层实现：

```rust
use schemaui::io::output::{OutputOptions, OutputDestination};

let options = OutputOptions::new(DocumentFormat::Toml)
    .with_pretty(true)
    .with_destinations(vec![OutputDestination::Stdout]);
ui = ui.with_output(options);
```

## 4. 完整参数速查

| 参数                     | 说明                                                      |
| ------------------------ | --------------------------------------------------------- |
| `-s, --schema <PATH>`    | Schema 文件或 `-` 代表 stdin                              |
| `--schema-inline <TEXT>` | 直接传入 Schema 字符串，与 `--schema` 互斥                |
| `--schema-format <json   | yaml                                                      |
| `-c, --config <PATH>`    | 配置样本文件或 stdin                                      |
| `--config-inline <TEXT>` | 内联配置，与 `--config` 互斥                              |
| `--config-format <json   | yaml                                                      |
| `--title <TEXT>`         | 自定义 TUI 标题                                           |
| `--output-format <json   | yaml                                                      |
| `--stdout`               | 输出到标准输出                                            |
| `--output <PATH>`        | 追加输出文件，可多次使用                                  |
| `--temp-file <PATH>`     | 当未指定输出时的回退文件路径（默认 `/tmp/schemaui.yaml`） |
| `--no-temp-file`         | 禁用回退文件                                              |
| `--no-pretty`            | 关闭美化输出                                              |

## 5. 运行示例

### 同时传入 Schema 与配置

```bash
schemaui-cli \
  --schema ./schema.json \
  --config ./config.yaml \
  --output-format toml \
  --stdout \
  --output ./config.out.toml
```

### 仅有配置（自动推断 Schema）

```bash
cat ./config.yaml | schemaui-cli --config - --output ./edited.json
```

### 使用 inline 文本避免多次 stdin

```bash
schemaui-cli --schema-inline '{"type":"object","properties":{"host":{"type":"string"}}}' \
             --config ./config.json --stdout
```

## 6. 错误与退出码

- 当参数冲突（例如同时指定 `--schema` 与 `--schema-inline`）或 STDIN
  被重复请求时，CLI 会输出错误并以非零退出码结束。
- `schema_with_defaults`、`parse_document_str`
  在解析失败时，也会连同具体格式提示信息一起返回，例如
  `failed to parse config as yaml`。
- 用户在 TUI 内按 `Ctrl+Q`、`Ctrl+S`
  等快捷键时的行为与库一致：未保存退出会要求确认，保存后才会返回 CLI
  并触发输出。

## 7. 与库的配合

CLI 与库完全解耦：

- 任何使用 `schemaui` 的项目都可以直接调用 `SchemaUI` API 构建自定义 CLI/GUI；
- CLI 只是一个薄封装，不会引入额外非必要依赖，方便在脚本化场景中快速部署。

如需扩展 CLI（例如添加配置文件、热加载、脚本化钩子），建议在
`schemaui-cli/src/main.rs` 基础上继续扩展，或在其他仓库中复用 `schemaui`
提供的入口/出口层。
