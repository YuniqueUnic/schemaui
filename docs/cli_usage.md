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

CLI 总是先依据文件扩展名决定解析格式：`config.yaml` → YAML、`schema.toml` → TOML。
若扩展名缺失或解析失败，则自动按照 **JSON → YAML → TOML**（取决于编译特性）依次回退，直到成功或耗尽格式，这样即便文件没有扩展名也能被识别。

## 3. 输出与持久化

- `--stdout`：将结果写入标准输出（若没有文件输出，则格式继承配置/Schema
  的扩展名，否则默认 JSON）。
- `-o, --output <path>`：可重复使用；`path` 的扩展名（`.json`/`.yaml`/`.toml`）
  用于决定序列化格式，多个输出必须共享相同的扩展名。
- `--temp-file` / `--no-temp-file`：当未提供输出且未指定 `--stdout` 时，默认写入
  `/tmp/schemaui.yaml`（可更改）。`--no-temp-file` 会彻底禁用该回退。
- `--no-pretty`：关闭美化输出。
- `--force`/`--yes` (`-f`/`-y`)：允许覆盖已经存在的输出文件。默认行为一旦发现
  目标文件存在就会报错并终止，避免误写入。

在至少存在一个文件输出时，CLI 按第一个路径的扩展名挑选格式；在纯 stdout
场景下则优先继承 `--config` / `--schema`
的文件扩展名（若无任何提示，则使用 JSON）。

底层实现：

```rust
use schemaui::io::output::{OutputOptions, OutputDestination};

let options = OutputOptions::new(DocumentFormat::Toml)
    .with_pretty(true)
    .with_destinations(vec![OutputDestination::Stdout]);
ui = ui.with_output(options);
```

## 4. 完整参数速查

| 参数                        | 说明                                                                 |
| --------------------------- | -------------------------------------------------------------------- |
| `-s, --schema <PATH>`       | Schema 文件或 `-` 代表 stdin                                         |
| `--schema-inline <TEXT>`    | 直接传入 Schema 字符串，与 `--schema` 互斥                           |
| `-c, --config <PATH>`       | 配置样本文件或 stdin                                                 |
| `--config-inline <TEXT>`    | 内联配置，与 `--config` 互斥                                         |
| `--title <TEXT>`            | 自定义 TUI 标题                                                      |
| `--stdout`                  | 输出到标准输出                                                       |
| `-o, --output <PATH>`       | 追加输出文件，可多次使用；扩展名决定序列化格式                       |
| `--temp-file <PATH>`        | 当未指定输出时的回退文件路径（默认 `/tmp/schemaui.yaml`）            |
| `--no-temp-file`            | 禁用回退文件                                                         |
| `--no-pretty`               | 关闭美化输出                                                         |
| `-f, --force` / `-y, --yes` | 允许覆盖已存在的输出文件，否则检测到冲突会直接报错并中止             |

## 5. 运行示例

### 同时传入 Schema 与配置

```bash
schemaui-cli \
  --schema ./schema.json \
  --config ./config.yaml \
  --stdout \
  -o ./config.out.toml
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
- 写入阶段若目标文件已存在且未提供 `--force`/`--yes`，会立即报错并退出，防止误覆盖。
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

## 8. 编译特性

`schemaui-cli` 将 `schemaui` 的格式特性透传为自身特性：

| CLI Feature | 说明 |
| --- | --- |
| `json`（默认） | 启用 JSON 解析/输出（始终可用） |
| `yaml`（默认） | 启用 YAML 解析/输出与自动检测 |
| `toml` | 启用 TOML 解析/输出与自动检测 |
| `all_formats` | 同时启用 `json`/`yaml`/`toml` |

例如：

```bash
cargo run -p schemaui-cli --no-default-features --features all_formats -- --schema schema.json --config config

cargo install --path schemaui-cli --no-default-features --features json
```

自动检测与输出能力会随所启用的格式特性一起裁剪：若禁用了 `yaml` 或 `toml`，CLI 的检测流程会自动跳过相应格式，只尝试剩余的类型。
