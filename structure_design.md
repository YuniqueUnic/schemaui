# schemaui 设计说明

本文记录 schemaui 如何把 JSON Schema 映射为 TUI、各模块的职责以及键盘、校验与测试策略。

## 1. 设计原则

- **Schema 保真**：支持 draft-07 中常见结构（对象、数组、`$ref`、`patternProperties`、`oneOf`/`anyOf` 等），并提供确定性的控件映射。
- **Form 优先**：所有 Schema 信息都会先转换为 `FormSchema`/`FormState`，渲染层永远不直接解析原始 JSON。
- **模块内聚**：loader/resolver/layout、form state、runtime、presentation 彼此解耦，公共 API 仅暴露 `SchemaUI`、`UiOptions` 等必要入口。
- **全链路校验**：任何一次编辑（包括 overlay）都会触发 `jsonschema::Validator`；错误即时反馈到字段和状态栏。
- **键盘一致性**：按键先被 `InputRouter` 分类为语义化 `KeyAction`，再由 `KeyBindingMap` 映射到 `FormCommand`/`AppCommand`，主界面和 overlay 共用同一套行为。

## 2. 管线总览

```
JSON Schema (serde_json::Value)
   │
   ▼
schema::loader        # 读取/构建 RootSchema
   │
   ▼
schema::resolver      # 解析 $ref / JSON Pointer
   │
   ▼
schema::layout::build_form_schema
   │   - 生成 RootSection / FormSection
   │   - 展平嵌套 Section、记录 depth
   │   - 映射字段类型为 FieldKind（string/enum/composite/...）
   ▼
FormSchema            # 纯描述性的表单结构
   │
   ▼
FormState             # 运行态值、聚焦索引、错误/脏状态
   │
   ├─ FormEngine      # 分发 FormCommand，驱动 jsonschema 验证
   └─ FieldState      # 负责输入、列表/overlay 操作、值序列化
   ▼
App runtime           # 事件循环 / overlay / 状态栏 / 持久化
   │
   └─ presentation    # Ratatui 组件：tabs、field 列表、overlay、footer
```

### 映射要点

| Schema 结构 | FieldKind / UI 控件 |
| --- | --- |
| `type: string/integer/number` | 文本输入（整数/浮点支持 `←/→` 调整） |
| `type: boolean` | Toggle/checkbox |
| `enum` | Popup 选择器 |
| `oneOf` / `anyOf` | 变体选择 + overlay 表单 |
| 标量数组 | 文本列表 + overlay（需时） |
| Composite/KeyValue 列表 | 列表面板 + overlay（含增删改移动） |
| `patternProperties` / `additionalProperties:false` | Key/Value 编辑器或限制性对象 |
| `$ref` 链 | 由 resolver 展开后，与 inline 字段一致 |

## 3. Form 层

- `domain::schema`：定义 `FormSchema`、`RootSection`、`FormSection`、`FieldSchema`。
- `form::state::FormState`：
  - 维护 `roots`（包含扁平 Section 与深度信息）。
  - 维护 `root_index`/`section_index`/`field_index`；`advance_section` 保证 Tab/箭头可在 root 之间循环。
  - 提供 `focused_field`、`field_by_pointer` 等查询方法。
- `form::field`（拆分为 `value.rs`、`convert.rs`、`state/*`）：
  - `builder.rs` 根据 `FieldSchema` 初始化值。
  - `input.rs` 处理普通输入控件。
  - `lists.rs` 负责 composite/list/kv/array 的增删改、overlay 交互及 `close_*` 操作。
  - `value_ops.rs` 提供 `display_value`、`current_value`、`seed_value`、错误状态管理。
- `form::key_value`、`form::composite`、`form::array`：分别封装特定编辑器的状态同步与 overlay 会话。

## 4. Runtime 与展示层

- `app::runtime::App`：
  - 封装 terminal 生命周期（`terminal::TerminalGuard`）与 panic hook（崩溃时退出备用屏，配合 `color-eyre` 输出栈）。
  - 保存 `StatusLine`、全局错误、保存/退出状态、当前 popup/overlay。
  - `handle_key` → `InputRouter::classify` → `KeyBindingMap::resolve` → 调用 FormCommand/AppCommand。
- `app::runtime::overlay`：包含 `OverlaySession`、`CompositeEditorOverlay`、overlay 验证/保存/关闭逻辑。
- `app::runtime::list_ops`：将列表增删改、选择抽成通用方法，主表单与 overlay 共享。
- `presentation::components`：
  - `sections.rs` 渲染 root/section tabs。
  - `fields.rs` 渲染字段列表、光标、meta 行、错误信息。
  - `overlay.rs` 绘制 overlay 主体 + 侧栏。
  - `footer.rs` 绘制帮助/状态栏。

## 5. 输入与快捷键

- `app::input::InputRouter`：
  - 识别 `Ctrl+J/L`（或 `Ctrl+[ / ]`）为 Root 步进；`Ctrl+Tab`/`Ctrl+Shift+Tab` 为 Section 步进；`Ctrl+N/D/←/→/↑/↓` 对应列表操作；`Ctrl+E/S/Q` 等全局命令；其他键默认下沉给字段。
- `KeyBindingMap`：内建默认映射，并允许用户在 `UiOptions` 覆盖特定 `KeyAction`。
- 统一的 `CommandDispatch`（Form/App/Input）确保主界面与 overlay 共享同一套输入语义。

## 6. 校验与错误体验

- `FormEngine::dispatch` 在 `FieldEdited` 时：
  1. 调用 `FormState::try_build_value` 组装整体 JSON；
  2. 使用 `jsonschema::Validator` 校验；
  3. 将错误信息写回匹配的字段，并在状态栏提示。
- overlay 内部使用 `validator_for` 针对当前子 Schema 构建临时 validator，实现即时校验。
- `status::StatusLine` 提供 `ready`/`value_updated`/`issues_remaining` 等语义信息；popup/overlay 会覆盖状态文案给出操作提示。

## 7. 测试组织

- 测试文件全部位于 `tests/`：`tests/form/*`、`tests/schema/*`、`tests/app/*`、`tests/presentation/*`。
- 每个源码模块通过 `include!(concat!(env!("CARGO_MANIFEST_DIR"), ...))` 引入对应测试文件，既能访问私有 API，又保持结构清晰。
- 当前测试覆盖：
  - form：字段导航（跨 root 循环）、KeyValue Unicode 处理。
  - schema：layout 映射、resolver `$ref`/Pointer 解析。
  - app：输入路由（root/section 快捷键）。
  - presentation：字段 meta 行配色。
- 新增功能时，请按照模块新增相应测试文件。

## 8. 研发准则

- 单个文件建议控制在 600 行以内；对于体量较大的逻辑（如 `runtime`、`field state`）必须拆分子模块。
- 优先使用成熟库（jsonschema、ratatui、crossterm、color-eyre），避免重复造轮子。
- 保持 KISS、SOLID：函数专注单一职责，模块之间通过明确的数据结构交互。
- 文档与代码同步演进：README 面向使用者，`structure_design.md` 记录实现细节，方便后来者快速接手。
