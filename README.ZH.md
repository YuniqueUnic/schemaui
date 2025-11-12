# schemaui

[![Crates.io](https://img.shields.io/crates/v/schemaui.svg)](https://crates.io/crates/schemaui)
[![Documentation](https://docs.rs/schemaui/badge.svg)](https://docs.rs/schemaui)
[![License](https://img.shields.io/crates/l/schemaui)](https://github.com/yuniqueunic/schemaui#license)
![Crates.io Total Downloads](https://img.shields.io/crates/d/schemaui)
![Deps.rs Crate Dependencies (latest)](https://img.shields.io/deps-rs/schemaui/latest)

<div align="center">
<img src="https://github.com/YuniqueUnic/schemaui/blob/master/docs/schemaui-demo.gif?raw=true" width="500">

[English](./README.md) | [中文文档](./README.ZH.md)

</div>

`schemaui` 将 JSON Schema
文档转换为由`ratatui`、`crossterm`和`jsonschema`驱动的完全交互式的终端用户界面。

该库解析丰富的模式（嵌套部分、`$ref`、数组、键值映射、模式属性等），将其转换为可导航的表单树，将其呈现为键盘优先的编辑器，并在每次编辑后验证结果，以便用户在保存之前始终可以看到完整的错误列表。

## 功能亮点

- **模式保真度** –
  `draft-07`，包括`$ref`、`definitions`、、`patternProperties`、枚举、数值范围以及嵌套的对象/数组。
- **部分和覆盖层** –
  顶层属性成为根标签，嵌套对象被展平为部分，复杂节点（复合体、键值集合、数组条目）打开具有自身验证器的专用覆盖层。
- **即时验证** –
  每次按键都可以触发`jsonschema::Validator`，所有错误（字段作用域 +
  全局）都被收集并一起显示。
- **可插拔的 I/O** – `io::input`可以处理
  JSON/YAML/TOML（通过功能标志），而`io::output`可以输出到标准输出和/或任何启用格式的多个文件。
- **内置 CLI** –
  `schemaui-cli`提供了与库相同的流程，包括多目标输出、stdin/内联规范和聚合诊断。

## 快速开始

```toml
[dependencies]
schemaui = "0.3.1"
serde_json = "1"
```

```rust
use schemaui::SchemaUI;
use serde_json::json;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema# ",
        "title": "服务运行时",
        "type": "object",
        "properties": {
            "metadata": {
                "type": "object",
                "properties": {
                    "serviceName": {"type": "string"},
                    "environment": {
                        "type": "string",
                        "enum": ["dev", "staging", "prod"]
                    }
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

    let value = SchemaUI::new(schema)
        .with_title("SchemaUI 演示")
        .run()?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
```

## 架构快照

```
┌─────────────┐   解析/合并     ┌──────────────┐   布局+输入  ┌─────────────┐
│ io::input   ├────────────────▶│ schema::*    ├─────────────▶│ form::*     │
└─────────────┘                 │ (加载器/     │              │ (状态,      │
                                │ 解析器/      │              │ 部分,       │
┌─────────────┐   输出值        │ 布局)        │   FormState  │ 状态机)     │
│ io::output  ◀─────────────────┴──────────────┘              └────────┬────┘
└─────────────┘                                             焦点/编辑  │
                                                                       │
                                                                ┌──────▼────────┐
                                                                │ app::runtime  │
                                                                │ (输入路由器,  │
                                                                │ 覆盖层, 状态) │
                                                                └──────┬────────┘
                                                                       │ 绘制
                                                                ┌──────▼────────┐
                                                                │ presentation::*│
                                                                │ (ratatui视图)  │
                                                                └────────────────┘
```

此布局反映了`src/`下的实际模块，便于将任何代码更改映射到其架构责任。

## 输入与输出设计

- `io::input::parse_document_str`将
  JSON/YAML/TOML（通过`serde_json`、`serde_yaml`、`toml`）转换为`serde_json::Value`。功能标志（`json`、`yaml`、`toml`、`all_formats`）保持依赖项精简。
- `schema_from_data_value/str`从活动配置中推断模式，注入草稿 -07
  元数据和默认值，以便 UI 加载现有值。
- `schema_with_defaults`将规范模式与用户数据合并，通过`properties`、`patternProperties`、`additionalProperties`、`dependencies`、`dependentSchemas`、数组和`$ref`目标传播默认值，而不修改原始树。
- `io::output::OutputOptions`封装了序列化格式、美观/紧凑切换以及`OutputDestination::{Stdout, File}`的向量。支持多个目标；冲突在输出前被捕获。
- `SchemaUI::with_output`将这些选项集成到运行时中，以便在会话结束后自动写入最终的`serde_json::Value`。

## JSON Schema → TUI 映射

`schema::layout::build_form_schema`遍历完全解析的模式，并将每个子树映射为`FormSection`/`FieldSchema`：

| 模式功能                                                     | 结果控件                                                |
| ------------------------------------------------------------ | ------------------------------------------------------- |
| `type: string`, `integer`, `number`                          | 带有数值保护的内联文本编辑器                            |
| `type: boolean`                                              | 切换/复选框                                             |
| `enum`                                                       | 弹出选择器（单选或多选用于数组枚举）                    |
| 数组                                                         | 内联列表摘要 + 每个项目的覆盖层编辑器                   |
| `patternProperties`, `propertyNames`, `additionalProperties` | 带有模式支持验证的键值编辑器                            |
| `$ref`, `definitions`                                        | 在布局前解析；被视为内联模式                            |
| `oneOf` / `anyOf`                                            | 变体选择器 + 覆盖层表单，将非活动变体排除在最终负载之外 |

根对象生成标签；嵌套对象成为带有面包屑标题的部分。每个字段记录其 JSON
指针（例如`/runtime/http/port`），以便焦点管理和验证可以精确映射错误。

## 验证生命周期

- `jsonschema::validator_for`在`SchemaUI::run`开始时编译完整模式一次。
- 每次编辑都会触发`FormCommand::FieldEdited`。`FormEngine`通过`FormState::try_build_value`重建当前文档，运行验证器，并将错误反馈到`FieldState`或全局状态行。
- 覆盖层（复合变体、键值映射、列表条目）会根据当前正在编辑的子模式启动自己的验证器，因此问题会在离开覆盖层之前浮出水面。

```
┌─────────────┐ 解析模式   ┌─────────────────┐ 膨胀状态        ┌────────────┐
│ SchemaUI::run├──────────▶│ domain::parse   ├────────────────▶│ FormState  │
└─────┬───────┘            │ (schema::layout)│                 └──────┬─────┘
      │ validator_for()    └─────────────────┘           编辑         │
      │                                                        ┌──────▼─────────┐
      └────────────────────────────────────────────────────── ▶│ app::runtime   │
                                                               │ (状态, 输入)   │
                                                               └──────┬─────────┘
                                                                      │ FormCommand
                                                               ┌──────▼──────────┐
                                                               │ FormEngine      │
                                                               │ + jsonschema    │
                                                               └─────────────────┘
```

`App`是`FormState`的唯一所有者；即使是覆盖层编辑也会通过`FormEngine`流动，以保持验证规则集中。

## TUI 构建块与快捷键

- **快捷键单一来源** –
  `keymap/default.keymap.json`列出了每个快捷键（上下文、组合键、动作）。`app::keymap::keymap_source!()`宏将此文件拉入二进制文件中，`InputRouter`使用它对`KeyEvent`进行分类，运行时页脚从相同的数据中呈现帮助文本——保持文档和行为
  DRY。
- **根标签与部分** –
  焦点通过`Ctrl+J / Ctrl+L`（根）和`Ctrl+Tab / Ctrl+Shift+Tab`（部分）循环。普通`Tab`/`Shift+Tab`在各个字段之间移动。
- **字段** –
  渲染标签、描述和内联错误消息。枚举/复合字段显示当前选择；数组总结长度和选定条目。
- **弹出窗口与覆盖层** – 按下`Enter`键打开枚举/oneOf
  选择器的弹出窗口；`Ctrl+E`打开复合编辑器的全屏覆盖层。覆盖层暴露集合快捷键（`Ctrl+N`、`Ctrl+D`、`Ctrl+←/→`、`Ctrl+↑/↓`）以及`Ctrl+S`提交。
- **状态与帮助** –
  页脚突出显示脏状态、未解决的验证错误和上下文感知帮助文本。当自动验证启用时，每次编辑都会立即更新这些计数器。

| 上下文 | 快捷键                              | 动作                     |
| ------ | ----------------------------------- | ------------------------ |
| 导航   | `Tab` / `Shift+Tab`                 | 在字段之间移动           |
|        | `Ctrl+Tab` / `Ctrl+Shift+Tab`       | 切换部分                 |
|        | `Ctrl+J` / `Ctrl+L`                 | 切换根标签               |
| 选择   | `Enter`                             | 打开弹出窗口/应用选择    |
| 编辑   | `Ctrl+E`                            | 启动复合编辑器           |
| 状态   | `Esc`                               | 清除状态或关闭弹出窗口   |
| 持久化 | `Ctrl+S`                            | 保存 + 验证              |
| 退出   | `Ctrl+Q` / `Ctrl+C`                 | 退出（如果脏则需要确认） |
| 集合   | `Ctrl+N` / `Ctrl+D`                 | 添加/删除条目            |
|        | `Ctrl+←/→`, `Ctrl+↑/↓`              | 选择/重新排序条目        |
| 覆盖层 | `Ctrl+S`, `Esc`, `Ctrl+N/D/←/→/↑/↓` | 保存、取消、管理复合列表 |

### 快捷键系统

将每个快捷键放入`keymap/default.keymap.json`中，以便运行时逻辑、帮助覆盖层和文档都使用单一信息源。

- **格式** – 每个 JSON
  对象声明一个`id`、人类可读的`description`、`contexts`（任何`"default"`、`"collection"`、`"overlay"`），一个`action`区分联合类型以及文本`combos`列表。例如：

  ```json
  {
    "id": "list.move.up",
    "description": "将条目上移",
    "contexts": ["collection", "overlay"],
    "action": { "kind": "ListMove", "delta": -1 },
    "combos": ["Ctrl+Up"]
  }
  ```

- **宏 + 解析器** – `app::keymap::keymap_source!()` `include_str!`s
  JSON，`once_cell::sync::Lazy`在启动时一次解析，并将每个组合键编译为`KeyPattern`（键码、所需修饰符、美观显示字符串）。
- **集成** –
  `InputRouter::classify`委托给`keymap::classify_key`，该函数返回嵌入在 JSON
  中的`KeyAction`。`keymap::help_text`根据`KeymapContext`过滤绑定，连接用于`StatusLine`和覆盖层说明的片段。
- **扩展** – 要添加快捷键，编辑
  JSON，选择暴露帮助文本的上下文，并在引入新的语义命令时在`KeyBindingMap`中连接结果`KeyAction`。

## 运行时层

| 层           | 模块 (s)                                                                   | 责任                                                           |
| ------------ | -------------------------------------------------------------------------- | -------------------------------------------------------------- |
| 摄取         | `io::input`, `schema::loader`, `schema::resolver`                          | 解析 JSON/TOML/YAML，解析`$ref`，并规范化元数据。              |
| 布局类型     | `schema::layout`                                                           | 从解析后的模式生成`FormSchema`（根/部分/字段）。               |
| 表单状态     | `form::state`, `form::section`, `form::field`                              | 跟踪焦点、指针、脏标志、强制转换和错误。                       |
| 命令与简化器 | `form::actions`, `form::reducers`, `app::validation`                       | 定义`FormCommand`，突变状态，并路由验证结果。                  |
| 运行时控制器 | `app::runtime`, `app::overlay`, `app::popup`, `app::status`, `app::keymap` | 事件循环，输入路由器分发，覆盖层生命周期，帮助文本，状态更新。 |
| 呈现         | `presentation::view`, `presentation::components::*`                        | 通过`ratatui`呈现标签、字段列表、弹出窗口、覆盖层和页脚。      |

每个模块保持在约 600 行代码以下（硬上限 800），以尊重 KISS
原则并使重构易于管理。

## CLI (`schemaui-cli`)

### Install

```bash
cargo install schemaui-cli
# It will be installed to `~/.cargo/bin` and renamed to `schemaui`
# 它将安装到 `~/.cargo/bin` 并重命名为 `schemaui`
# 所以你应该这样使用： `schemaui -c xxx`
```

```bash
schemaui \
  --schema ./schema.json \
  --config ./defaults.yaml \
  -o - \
  -o ./config.toml ./config.json
```

```
┌────────┐  clap args   ┌──────────────┐ read stdin/files ┌─────────────┐
│  CLI   ├─────────────▶│ InputSource  ├─────────────────▶│ io::input   │
└────┬───┘              └──────┬───────┘                  └────┬────────┘
     │ diagnostics             │ schema/default Value          │
┌────▼─────────┐        ┌──────▼──────┐                        |
│Diagnostic    │◀───────┤ FormatHint  │                        │
│Collector     │        └──────┬──────┘                        │
└────┬─────────┘               │ pass if clean                 │
     │                         │                               │
┌────▼────────┐  build options └────────────┐                  │
│Output logic ├────────────────────────────▶│ OutputOptions    │
└────┬────────┘                             └────────────┬─────┘
     │ SchemaUI::new / with_*                        ┌───▼────────┐
     └──────────────────────────────────────────────▶│ SchemaUI   │
                                                     │ (library)  │
                                                     └────────────┘
```

- 输入 – `--schema` /
  `--config`接受文件路径、内联有效载荷或`-`用于标准输入（但不能同时使用两者）。如果只提供配置，CLI
  通过`schema_from_data_value`推断模式。
- 诊断 –
  `DiagnosticCollector`累积格式问题、功能标志不匹配、标准输入冲突和现有输出文件，以执行前的诊断。
- 输出 –
  `-o/--output`可重复使用，并且可以混合文件路径与`-`用于标准输出。当未设置目标时，工具写入`/tmp/schemaui.json`，除非传递了`--no-temp-file`。扩展名决定格式；拒绝冲突的扩展名。
- 标志 –
  `--no-pretty`切换紧凑输出，`--force/--yes`允许覆盖文件，`--title`传递到`SchemaUI::with_title`。

## 关键依赖项

| 库                                          | 用途                                        |
| ------------------------------------------- | ------------------------------------------- |
| `serde`, `serde_json`, `serde_yaml`, `toml` | 解析和序列化模式/配置数据。                 |
| `schemars`                                  | 草稿 -07 模式表示，由`schema::layout`使用。 |
| `jsonschema`                                | 表单和覆盖层的运行时验证。                  |
| `ratatui`                                   | 渲染小部件、布局、覆盖层和页脚。            |
| `crossterm`                                 | 输入路由器消耗的终端事件。                  |
| `indexmap`                                  | 模式遍历的顺序保持映射。                    |
| `once_cell`                                 | 懒解析快捷键 JSON。                         |
| `clap`, `color-eyre` (CLI)                  | 参数解析和用户友好的诊断。                  |

## 文档映射

- `README.md` – 概述 + 架构快照。
- `docs/structure_design.md` – 详细的模式/布局/运行时设计，带有流程图。
- `docs/cli_usage.md` – CLI 特定手册（输入、输出、管道、示例）。

## 开发

- 定期运行`cargo fmt && cargo test`；大多数模块通过`include!`嵌入`tests/`中的文件，以覆盖私有
  API。
- 将模块保持在约 600 行代码以下（硬上限
  800）。一旦行为增长，就拆分帮助程序，以保持 KISS 完整。
- 优先使用成熟的库（`serde_*`、`schemars`、`jsonschema`、`ratatui`、`crossterm`、`once_cell`），除非更改微不足道，否则不要编写定制代码。
- 每当管道、快捷键或 CLI 语义演变时，更新`docs/*`，以确保面向用户文档的真实性。

## 参考项目

1. https://github.com/rjsf-team/react-jsonschema-form
2. https://ui-schema.bemit.codes/examples

## 路线图

- [x] 在运行时解析 JSON Schema 并生成 TUI
- [ ] 在运行时解析 JSON Schema 并生成 Web UI
- [ ] 在编译时解析 JSON Schema，然后生成 TUI 代码，为运行时暴露必要的 API
- [ ] 在编译时解析 JSON Schema，然后生成 Web UI 代码，为运行时暴露必要的 API
- [ ] 在运行时解析 JSON Schema 并生成交互式 CLI
- [ ] 在编译时解析 JSON Schema，然后生成交互式 CLI 代码，为运行时暴露必要的 API

## 许可证

根据您的选择，本项目可在以下许可证下授权：

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) 或
  http://www.apache.org/licenses/LICENSE-2.0 )
- MIT license ([LICENSE-MIT](LICENSE-MIT) 或 http://opensource.org/licenses/MIT
  )

### 贡献

欢迎贡献！请随时提交拉取请求。

祝您编程愉快！

## Star 历史

<a href="https://www.star-history.com/#YuniqueUnic/schemaui&type=date&legend=top-left">
<picture>
  <source
    media="(prefers-color-scheme: dark)"
    srcset="
      https://api.star-history.com/svg?repos=YuniqueUnic/schemaui&type=date&legend=top-left&theme=dark
    "
  />
  <source
    media="(prefers-color-scheme: light)"
    srcset="
      https://api.star-history.com/svg?repos=YuniqueUnic/schemaui&type=date&legend=top-left
    "
  />
  <img
    alt="Star History Chart"
    src="https://api.star-history.com/svg?repos=YuniqueUnic/schemaui&type=date&legend=top-left"
  />
</picture>
</a>
