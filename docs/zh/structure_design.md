# schemaui 架构指南

本文档解释了 schemaui 如何接收 schemas/configs、将它们映射到表单结构、渲染 TUI
以及验证数据。使用它来导航代码库，并作为更改管道时的检查清单。

## 1. 设计原则

1. **克制与精确** – 保持模块小型化（优先 <600
   行代码），当行为增长时拆分文件，并依赖成熟的
   crate（`serde_*`、`jsonschema`、`ratatui`、`crossterm`、`clap`）。
2. **Schema 保真度** – 运行时绝不能丢弃影响验证或 UI 的关键字（draft-07
   覆盖范围包括 `$ref`、`definitions`、`patternProperties`、`oneOf/anyOf`）。
3. **表单优先渲染** – TUI 仅消费 `FormState`；原始 JSON 由 IO/schema
   层解析一次，在表示代码中不再重复解析。
4. **完整诊断** – 输入和输出错误在（CLI +
   运行时）中累积，然后才显示给用户，以便用户在一次性看到所有需要修复的内容。
5. **键盘为中心的 UX** – 每个操作都映射到 `KeyAction` → `CommandDispatch`
   链，保持导航、覆盖层和弹出窗口的一致性。

## 2. 数据接收与 I/O 层

模块：`src/io`（库和 CLI 共享）。

```
┌────────────┐ text/file/stdin ┌───────────────────┐ serde_* per feature ┌──────────────┐
│ Document   ├────────────────▶│ io::input::parse  ├────────────────────▶│ serde_json:: │
│ source     │                 │ (DocumentFormat)  │                     │ Value        │
└────────────┘                 └───────────────────┘                     └──────────────┘
```

- **输入源** – `io::input::parse_document_str` 使用 `serde_json`、`serde_yaml`
  和 `toml`（功能门控）从文件、stdin 或内联字符串接收 JSON/TOML/YAML。CLI
  镜像此逻辑并首先检查路径是否存在；如果不存在，则解析字面字符串。
- **Schema + config 关系** – 用户可以传递规范 schema
  加配置快照。`schema_with_defaults`（由 `DefaultApplier` 提供支持）将快照值作为
  `default` 关键字注入到
  `properties`、`patternProperties`、`additionalProperties`、`$ref`
  目标、数组和依赖 schema
  中，而不改变结构。当仅提供数据时，`schema_from_data_value/str` 推断 schema
  并使用默认值对其进行注释。
- **格式提示与功能** – `DocumentFormat::available_formats()` 反映编译时功能。CLI
  的 `FormatHint`/`InputSource` 组合检查扩展名，拒绝已禁用格式的请求，并控制
  stdin 使用。
- **输出** – `io::output::OutputOptions` 将格式选择、美化/紧凑切换和
  `OutputDestination::{Stdout, File}` 向量分组。CLI 调用者可以提供多个目标（混合
  stdout 和文件），库通过 `SchemaUI::with_output` 重用相同的类型。
- **诊断** – CLI 的 `DiagnosticCollector` 保留每个问题（无效的 schema/config
  规范、混合输出格式、缺失功能、现有文件），并在 UI 启动前一起报告它们。

## 3. Schema 解释管道

```
io::input (serde_json::Value)
  → schema::loader::load_root_schema            // 反序列化 RootSchema
  → schema::resolver::SchemaResolver            // 解析 $ref / JSON Pointer
  → schema::layout::build_form_schema           // 构建 FormSchema 树
  → form::state::FormState::from_schema         // 具化 FieldState
  → app::runtime::App                           // 驱动 TUI + 验证
  → io::output::emit (optional)                 // 写入最终 Value
```

关键职责：

1. **加载器** – 带上下文的 `serde_json::from_value`，以便在 schema
   格式错误时用户获得可操作的错误。
2. **解析器** – 在 `properties`、`definitions` 或任意 JSON Pointer 片段内展开
   `$ref`，确保下游逻辑使用完全具化的 `SchemaObject`。
3. **布局** – 通过遍历每个对象将解析后的 schema 转换为 `FormSchema`：
   - 顶级属性成为 `RootSection`（每个属性一个）加上用于松散字段的合成 "General"
     根。
   - 嵌套对象成为 `FormSection`。`SectionInfo`
     使用元数据（`title`、`description` 或自定义 `x-group*` 扩展）命名部分。
   - `detect_kind` 将 `SchemaObject` 映射到 `FieldKind`（基元、枚举、数组、复合
     `oneOf`/`anyOf`、键/值映射），同时防范不支持的形状。
4. **表单状态** – `FormState::from_schema` 展平每个部分，跟踪
   `root_index`/`section_index`/`field_index`，并公开导航、值组装（`try_build_value`）、播种默认值和错误记录的辅助函数。
5. **运行时** – `app::runtime::App`
   将输入处理、覆盖层、状态消息、验证和可选输出序列化联系在一起。

## 4. 支持的 Schema 结构

当前的布局 + 表单堆栈处理：

- 任意根部分（标签页）和带有面包屑标题的嵌套部分。
- 深度嵌套的对象和数组（复合 + 枚举数组打开覆盖层；标量数组保持内联）。
- `$ref` 链和共享的 `definitions`。
- `oneOf` / `anyOf` 复合（单选或多选取决于
  schema）。用户通过弹出窗口选择变体，然后在覆盖层内编辑展开的内容。
- `patternProperties`、`propertyNames` 和 `additionalProperties`，用于构建基于
  schema 的键/值编辑器。
- `dependentSchemas`、`dependencies` 以及通过 `schema_with_defaults`
  插入的默认值，以便派生值立即显示。

不支持的形状（例如，嵌套数组的数组）在布局期间记录面向用户的错误，以便他们可以调整
schema 而不是遇到未定义的行为。

## 5. 验证与错误显示

文件：`src/app/validation.rs` + `form::reducers`。

1. `SchemaUI::run` 预先编译 `jsonschema::Validator`（panic 成为带上下文的
   `color-eyre` 报告）。
2. 每次编辑发出 `FormCommand::FieldEdited { pointer }`。`FormEngine` 通过
   `FormState::try_build_value` 重建 JSON 值并将其馈送到验证器。
3. `ValidationOutcome::Invalid` 清除旧错误，通过 `FormState::set_error`
   将新错误分发到匹配字段（通过 JSON
   指针），并将其余错误复制到全局错误列表（在页脚下方渲染）。
4. 构建失败（例如，无效的数字字面量）导致
   `ValidationOutcome::BuildError`，保持验证器完整但突出显示有问题的字段。
5. 覆盖层在对应于当前复合/键值/列表条目的子 schema 上使用
   `validator_for`，确保嵌套编辑在提交前也得到验证。

## 6. 运行时与表示分层

- **输入处理** – `app::input::InputRouter` 将 `KeyEvent` 分类为语义
  `KeyAction`（字段步进、部分步进、根步进、弹出窗口切换、列表操作、覆盖层编辑、保存/退出）。`KeyBindingMap`
  将操作转换为 `CommandDispatch::{Form, App, Input}`。可以通过 `UiOptions`
  注入自定义绑定。
- **键映射管道** – 引入 `keymap/default.keymap.json`，由 `app::keymap` 通过
  `once_cell::sync::Lazy` 解析一次。每个条目包含：
  - `id` + `description` 用于文档/帮助文本。
  - `contexts`：`default`、`collection`、`overlay` 中的任意一个。这些映射到
    `KeymapContext`，以便 `keymap::help_text` 可以渲染正确的页脚片段。
  - `action`：标记对象（`Save`、`FieldStep { delta }`、`ListMove { delta }`
    等），直接反序列化为 `KeyAction`。
  - `combos`：文本快捷方式（例如 `"Ctrl+Shift+Tab"`）。令牌被解析为
    `KeyPattern`（必需的修饰符 + 代码匹配器）。字母组合隐式容忍
    `Shift`，除非模式已经需要它。`InputRouter::classify` 现在完全委托给
    `keymap::classify_key`，覆盖层/状态模块从同一数据集中提取帮助文本，保证 DRY
    文档 + UI。因此，添加快捷方式只需要编辑 JSON 文件（如果引入全新的
    `KeyAction`，可选地编辑 `KeyBindingMap`）。
- **App 运行时** – `app::runtime::App` 维护：
  - `FormState` 和编译的验证器
  - `StatusLine`（脏标志、帮助文本、诊断计数）
  - 用于枚举/变体选择的 `PopupState`
  - `CompositeEditorOverlay` 会话（参见
    `runtime/overlay.rs`），用于使用撤销/重做语义和每条目验证器编辑嵌套数据
  - 列表辅助函数（`runtime/list_ops.rs`）用于主视图和覆盖层共享的插入/删除/重新排序操作
  - 使用 `TerminalGuard` 的中央绘制循环（在 panic 时恢复终端状态）
- **表示** – `presentation::view` 将屏幕分为主体 + 页脚，然后调用
  `components::*` 来渲染：
  - 带焦点标记的根和部分标签页
  - 字段行（标签、值预览、元数据徽章、内联错误消息）
  - 弹出窗口（枚举/变体选择器）和覆盖层（带可选列表面板的全屏编辑器）
  - 页脚 / 帮助覆盖层（脏状态、验证计数、上下文感知提示）

### 6.1 事件循环时间线

```
KeyEvent (crossterm)
    │
    ▼
InputRouter::classify ─▶ KeyAction ─▶ KeyBindingMap ─▶ CommandDispatch
                                                          │
                                                          ▼
                                                  app::runtime::App
                                                          │
                                                          ▼
                                                FormEngine + Validator
                                                          │
                                                          ▼
                                                  presentation::draw
```

- `InputRouter` 委托给 `keymap::classify_key`，将键矩阵保留在 JSON 中。
- `KeyBindingMap` 将语义操作转换为 `FormCommand`（变更 `FormState`）或
  `AppCommand`（弹出窗口、覆盖层、状态、退出/保存）。
- `App::handle_key` 在安排下一次 `presentation::draw` 调用之前路由所有副作用。

### 6.2 覆盖层生命周期

```
Form focus ──Ctrl+E──▶ try_open_composite_editor
                          │
                          ▼
                 CompositeEditorOverlay::new
                          │ setup_overlay_validator
                          ▼
                   overlay FormState + StatusLine
                          │
        (InputRouter + KeyBindingMap reused inside overlay)
                          │
                          ▼
                      save/quit ──▶ close_composite_editor(commit)
```

- 覆盖层生成自己的 `FormState` 和可选的列表面板元数据，同时通过
  `jsonschema::validator_for` 重用全局验证器，该验证器作用于嵌套 schema。
- 帮助文本通过 `keymap::help_text(KeymapContext::Overlay)`
  获取，因此页脚消息保持同步。

## 7. 快捷键参考

| 作用域   | 快捷键                                     | 命令                           |
| -------- | ------------------------------------------ | ------------------------------ |
| 字段     | `Tab` / `Shift+Tab`、`Down` / `Up`         | 在部分内循环                   |
| 部分     | `Ctrl+Tab` / `Ctrl+Shift+Tab`              | 在当前根中的部分之间跳转       |
| 根       | `Ctrl+J` / `Ctrl+L`                        | 在根标签页之间跳转             |
| 弹出窗口 | `Enter`（打开/应用）、`Esc`（关闭/重置）   | 管理枚举/复合                  |
| 列表     | `Ctrl+N`、`Ctrl+D`、`Ctrl+←/→`、`Ctrl+↑/↓` | 添加/删除/选择/重新排序条目    |
| 覆盖层   | `Ctrl+E`                                   | 为复合/键值/列表条目启动编辑器 |
| 持久化   | `Ctrl+S`                                   | 保存 + 验证                    |
| 退出     | `Ctrl+Q`、`Ctrl+C`                         | 准备退出 / 确认退出            |

每个快捷键都通过 `InputRouter`
运行，因此覆盖层和主视图的行为相同，除非明确覆盖。

## 8. CLI 流程（schemaui-cli）

1. 解析 `--schema SPEC` 和
   `--config SPEC`。每个规范可以是文件路径、原始有效负载或
   `-`（stdin）。两个流不能同时使用 stdin；提示用户改为发送一个内联。
2. 从扩展名确定格式提示，检查功能可用性，并尝试加载/解析。失败将消息推送到
   `DiagnosticCollector` 而不是提前返回。
3. 从重复的 `-o/--output` 参数构建输出目标（每次出现接受多个值）。目标可以混合
   stdout（`-`）和文件路径；记录任何冲突的扩展名、缺失的功能或现有文件。
4. 在报告所有诊断（如果有）后，CLI 实例化
   `SchemaUI`，播种默认值（如果提供了配置），运行 TUI，最后使用请求的格式 +
   目标发出结果。

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

## 9. 公共 API 与输出钩子

- **`SchemaUI`**（`src/app/schema_ui.rs`）– 库消费者的入口点。公开原始 schema
  值、schema+data 对和推断 schema 的构造函数。在调用 `.run()` 之前链接
  `.with_title`、`.with_options`、`.with_output` 或 `.with_default_data`。
- **`UiOptions`** – 切换 UI 行为（滴答率、自动验证、帮助可见性、通过
  `KeyBindingMap` 的自定义键绑定）。
- **`OutputOptions` + `OutputDestination`** – 配置格式、美化和目标。由 CLI
  和任意宿主应用程序共享。
- **`DocumentFormat::available_formats()`** –
  揭示编译了哪些解析/序列化功能，以便宿主可以定制 UX。

## 10. 测试与维护

- 模块特定的测试位于 `tests/` 下，并通过 `include!` 包含到各自的模块中以覆盖私有
  API。添加新的测试文件而不是将现有文件扩展到超过 ~200 行。
- 在提交前运行 `cargo check` 或 `cargo test -p schemaui-cli`；大型重构应涵盖库和
  CLI crate。
- 保持文档双语？否——公共文档（README/设计）必须保持英文，以便下游用户无需翻译即可使用它们。

每当添加 schema 关键字、引入新覆盖层或修改 CLI 语义时，请回顾本指南。
