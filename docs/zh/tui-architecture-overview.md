# TUI 架构与键位映射概览

本文档是关于**TUI 运行时**、其**UI
概念**（主页面、章节、子章节、字段、条目、浮层、弹出窗口、帮助浮层）以及驱动键盘导航的**键位映射**的简明指南。

本文档面向需要以下工作的工程师：

- 理解当前实现的真实工作原理。
- 安全地扩展/重构 TUI、浮层或键绑定。
- 向其他团队成员解释这些概念。

内容反映了`src/tui`和`keymap/default.keymap.json`下的**当前代码库**。

---

## 1. TUI 在整个流程中的位置

从高层来看，TUI 是一个前端，它消费来自核心管道准备好的`FrontendContext`。

**关键代码：**

- `src/core/pipeline.rs` – `SchemaPipeline`
- `src/core/frontend.rs` – `Frontend`, `FrontendContext`
- `src/tui/session.rs` – `TuiFrontend`
- `src/tui/app/schema_ui.rs` – `SchemaUI`构建器

### 1.1 数据与控制流

```text
JSON 模式 + 默认值 (serde_json::Value)
    │
    ▼
core::SchemaPipeline
    - schema_with_defaults
    - validator_for
    - build_ui_ast
    │
    ▼
FrontendContext { ui_ast, validator, initial_data, schema }
    │
    ▼
TuiFrontend::run(ctx)
    - form_schema_from_ui_ast → FormSchema
    - FormState::from_schema_with_palette
    │
    ▼
App::run()
    - 输入循环
    - 浮层、弹出窗口、帮助浮层
    │
    ▼
View (tui::view::draw)
    - 主页面、浮层、弹出窗口、帮助浮层
```

简化版的类 Rust 伪代码（来自`SchemaUI`）：

```rust
fn run_tui(schema: Value, defaults: Option<Value>) -> Result<Value> {
    let pipeline = SchemaPipeline::new(schema)
        .with_title(Some("Title".into()))
        .with_defaults(defaults);

    let frontend = TuiFrontend { options: UiOptions::default() };
    pipeline.run_with_frontend(frontend)
}
```

从现在开始，我们专注于**TuiFrontend**、**FormState**、`App`运行时以及键位映射。

---

## 2. 核心 UI 概念

本节为你在代码和文档中看到的 TUI 术语提供精确的含义。

### 2.1 主页面（根表单）

**主页面**是显示以下内容的根表单视图：

- 根标签页（每个顶层模式组一个）
- 活动根目录内的章节和子章节
- 活动章节内的字段
- 内联错误和页脚状态栏

**关键代码：**

- `src/tui/state/form_state.rs` – `FormState`
- `src/tui/state/section.rs` – `SectionState`
- `src/tui/model/layout` – `FormSchema`, `FormSection`
- `src/tui/view/components/body.rs` – `render_body`
- `src/tui/view/frame.rs` – `UiContext`, `draw`

运行时表现为`FormState`：

```rust
pub struct FormState {
    pub roots: Vec<RootSectionState>,
    ui: UiStores,
}

pub struct RootSectionState {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub sections: Vec<SectionState>,
}

pub struct SectionState {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub path: Vec<String>,       // 类似面包屑的路径
    pub depth: usize,            // 0 = 顶级，>0 = 子章节
    pub fields: Vec<FieldState>,
    pub scroll_offset: usize,
}
```

`UiStores`跟踪主页面的焦点：

```rust
pub struct UiStores {
    pub root: RootTabsStore,
    pub sections: SectionTabsStore,
    pub fields: FieldListStore,
}
```

**从模式映射：**

- 顶级对象属性 → `RootSectionState`（根标签页）。
- 嵌套对象 → `SectionState`（扁平化为每个根目录的列表，使用`depth`表示嵌套）。
- 模式中的 JSON
  指针成为`FieldSchema`和`FieldState`中的`pointer`字段，用于验证和错误映射。

**渲染方式：**
`render_body`，它使用活动根、章节和字段焦点来绘制主页面的可见部分。

### 2.2 章节与子章节

**章节**是根内的逻辑字段组（例如"元数据"、"HTTP"、"TLS"）。**子章节**是章节内的嵌套章节。

在布局时（`tui::model::layout`），会生成`FormSection`的树结构。在运行时，这棵树被扁平化为`SectionState`向量，同时保留`depth`和`path`：

```rust
impl SectionState {
    pub fn collect(
        section: &FormSection,
        depth: usize,
        palette: &Arc<ComponentPalette>,
        acc: &mut Vec<SectionState>,
    ) {
        // 为此章节创建 SectionState
        let fields = section
            .fields
            .iter()
            .cloned()
            .map(|schema| FieldState::from_schema_with_palette(schema, Arc::clone(palette)))
            .collect();
        acc.push(SectionState {
            id: section.id.clone(),
            title: section.title.clone(),
            description: section.description.clone(),
            path: section.path.clone(),
            depth,
            fields,
            scroll_offset: 0,
        });

        // 递归添加子章节
        for child in &section.children {
            SectionState::collect(child, depth + 1, palette, acc);
        }
    }
}
```

- 父子关系由`depth`和`path`捕获。
- 视图层使用这些为子章节渲染缩进和面包屑。

### 2.3 字段

**字段**是实际的可编辑控件——字符串、数字、枚举、复合类型、列表、键/值映射等。

**关键代码：**

- `src/tui/state/form_state.rs` – `FieldState`
- `src/tui/state/field/components` – `FieldComponent`实现
- `src/tui/view/components/body.rs` – 字段行渲染

运行时表现：

```rust
pub struct FieldState {
    pub schema: FieldSchema,
    pub(crate) component: Box<dyn FieldComponent>,
    pub dirty: bool,
    pub error: Option<String>,
}
```

具体的`component`编码不同模式形状的行为：

- 基本类型 – 文本/数字编辑器。
- 枚举 – 基于弹出窗口的选择。
- 复合类型 – `oneOf`/`anyOf`单变体。
- 复合列表 – 复合类型的数组（"条目"）。
- 键/值 – 类似映射的编辑器。
- 标量数组 – 简单类型的数组。

**模式映射：** `FieldSchema`引用已解析的`SchemaObject`及其 JSON
指针；该指针用于：

- 从`FormState`构建值（`try_build_value`）。
- 将验证器错误附加到特定字段。
- 在帮助浮层错误列表中显示指针。

### 2.4 条目（集合条目）

**条目**是可重复字段中的单个元素：

- 复合列表（`CompositeListState`）的项，例如对象数组或`anyOf`包装项。
- 映射中的键/值条目。
- 标量数组中的项。

**关键代码：**

- `src/tui/state/composite/composite_list.rs` – `CompositeListState`
- `src/tui/state/field/components/composite_list.rs` – 列表字段组件
- `src/tui/app/runtime/list_ops.rs` – 列表操作
- `src/tui/app/popup.rs` – 条目的变体选择器弹出窗口

`CompositeListState`维护以下内容：

- `entries` – 每个条目的状态（变体、摘要、嵌套值）。
- `selected_index` – 哪个条目获得焦点。
- 操作：添加/移除/移动/选择条目。

这些操作通过`FormCommand`暴露给运行时，并最终挂钩到`Ctrl+N`、`Ctrl+D`、`Ctrl+Left/Right`、`Ctrl+Up/Down`等键绑定（参见键位映射
JSON 和下面的列表操作部分）。

### 2.5 弹出窗口

**弹出窗口**是用于选择值的小型居中列表：

- 枚举选择
- 布尔切换
- 复合变体选择
- 复合列表条目变体选择

**关键代码：**

- `src/tui/app/popup.rs` – `PopupState`, `PopupOwner`
- `src/tui/app/runtime/mod.rs` – `popup: Option<AppPopup>`, `handle_popup_key`
- `src/tui/view/components/popup.rs` – `render_popup`

运行时持有：

```rust
struct AppPopup {
    owner: PopupOwner,
    state: PopupState,
}

pub struct App {
    // ...
    popup: Option<AppPopup>,
}
```

`PopupOwner`告诉运行时如何应用选择：

- `Root` – 直接修改`FormState`。
- `Composite` – 修改活动浮层。
- `VariantSelector { .. }` – 为复合列表条目选择变体。

弹出窗口是**模态的**：当弹出窗口打开时，`App::handle_key`会先将事件路由到`handle_popup_key`，与键位映射上下文无关。

### 2.6 浮层

**浮层**是用于嵌套结构的全屏编辑器：

- 编辑复合字段（oneOf/anyOf 对象、多态联合）。
- 编辑复合列表中的单个条目。
- 当键/值条目或标量数组需要自己的表单时进行编辑。

**关键代码：**

- `src/tui/app/runtime/overlay` – 浮层运行时逻辑
- `src/tui/app/runtime/mod.rs` – `overlay_stack`
- `src/tui/view/components/overlay.rs` – `render_composite_overlay`
- `src/tui/view/frame.rs` – `CompositeOverlay`, `draw`中的分层

运行时维护一个浮层栈：

```rust
pub struct App {
    // ...
    overlay_stack: Vec<CompositeEditorOverlay>,
}
```

每个浮层都包装了自己的`FormState`以及元数据：

```rust
pub struct OverlayState {
    pub field_pointer: String,
    pub field_label: String,
    pub host: OverlayHost,
    pub level: usize,
    pub target: CompositeOverlayTarget,
    pub session: OverlaySession,
    // ...
}
```

- `OverlayHost::RootForm`表示浮层正在编辑主页面的字段。
- `OverlayHost::Overlay { parent_level }`处理从浮层打开的浮层。
- `CompositeOverlayTarget`区分字段级编辑与每个条目编辑。

**生命周期（简化）：**

```text
表单焦点 ──Ctrl+E──▶ try_open_composite_editor
                          │
                          ▼
                 CompositeEditorOverlay::new
                          │ setup_overlay_validator
                          ▼
                   浮层 FormState + StatusLine
                          │
        （InputRouter + Keymap 在浮层内复用）
                          │
                          ▼
                     Ctrl+S ──▶ save_active_overlay（保持打开）
                          │
                          ▼
                     Esc/Q ──▶ close_active_overlay（提交）
```

浮层复用全局键位映射和验证器基础设施，但将其应用于嵌套的`FormState`实例。

### 2.7 帮助浮层

**帮助浮层**是最顶层，显示：

- 键盘快捷键的分页列表（按键位映射上下文分组）。
- 当前字段错误的摘要（JSON 指针 + 截断消息）。

**关键代码：**

- `src/tui/app/runtime/mod.rs` – `HelpOverlayState`, `toggle_help_overlay`,
  `handle_help_overlay_key`
- `src/tui/view/frame.rs` – `HelpOverlayRender`, `UiContext`
- `src/tui/view/components/help.rs` – `render_help_overlay`
- `src/tui/state/form_state.rs` – `error_entries()`

运行时状态：

```rust
pub struct App {
    // ...
    help_overlay: Option<HelpOverlayState>,
}

struct HelpOverlayState {
    pages: Vec<Vec<String>>, // 每页是一系列行
    page: usize,             // 当前页索引
}
```

按键处理：

```rust
fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
    if key.kind != KeyEventKind::Press { return Ok(()); }

    if self.handle_help_overlay_key(&key) { return Ok(()); }
    if self.handle_popup_key(key)? { return Ok(()); }
    // 然后是浮层/主页面处理
}

fn handle_help_overlay_key(&mut self, key: &KeyEvent) -> bool {
    let Some(state) = self.help_overlay.as_mut() else { return false; };
    match key.code {
        KeyCode::Esc => { self.help_overlay = None; true }
        KeyCode::Tab => { /* 下一页 */ true }
        KeyCode::BackTab => { /* 上一页 */ true }
        _ => false,
    }
}
```

在视图层，`UiContext`包含一个可选的`HelpOverlayRender`，`frame::draw`最后渲染它，因此它位于主页面、浮层和弹出窗口之上。

---

## 3. 键位映射模型与上下文

键位映射在 JSON 中定义并在启动时解析。

**关键代码：**

- `keymap/default.keymap.json` – 键位映射数据
- `src/tui/app/keymap.rs` – 解析，`KeymapContext`, `KeymapStore`
- `src/tui/app/options.rs` – 接入`UiOptions`
- `src/tui/app/input.rs` – `KeyAction`, `KeyBindingMap`, `InputRouter`

### 3.1 JSON 结构

`default.keymap.json`中的每个条目如下所示：

```json
{
  "id": "list.move.up",
  "description": "Move entry up",
  "contexts": ["collection", "overlay"],
  "action": { "kind": "listMove", "delta": -1 },
  "combos": ["Ctrl+Up"]
}
```

字段：

- `id` – 绑定的稳定标识符（用于文档/日志）。
- `description` – 帮助消息中使用的人类可读文本。
- `contexts` – 语义范围（`"default"`、`"collection"`、`"overlay"`）。
- `action` – 标记联合，反序列化为`RawAction` → `KeyAction`。
- `combos` – 文本组合列表（例如`"Ctrl+Shift+Tab"`）。

`KeymapStore`解析这些条目，将其转换为带有解析后`KeyPattern`的`KeyBinding`实例，并公开两个主要
API：

```rust
pub fn classify(&self, key: &KeyEvent) -> Option<KeyAction>;
pub fn help_text(&self, context: KeymapContext) -> Option<String>;
```

### 3.2 键位映射上下文

`KeymapContext`表示帮助文本的语义组：

```rust
pub enum KeymapContext {
    Default,
    Collection,
    Overlay,
}
```

它们**不**用于过滤哪些键触发；相反，它们：

- 决定哪些贡献到页脚帮助字符串。
- 驱动哪些绑定出现在帮助浮层的每一页中。

运行时根据焦点选择上下文：

```rust
fn current_help_text(&self) -> Option<String> {
    if !self.options.show_help { return None; }
    let context = if self.overlay_depth() > 0 {
        KeymapContext::Overlay
    } else if let Some(field) = self.form_state.focused_field()
        && field.is_composite_list()
    {
        KeymapContext::Collection
    } else {
        KeymapContext::Default
    };
    self.keymap_store.help_text(context)
}
```

语义含义：

- **Default** – 不在特殊列表上下文中且不在浮层内部时的应用级导航和编辑。
- **Collection** –
  在主页面聚焦集合类字段时的列表操作（添加/移除/选择/移动条目）。
- **Overlay** – 在任何浮层内部时的操作（包括浮层内集合的列表操作）。

### 3.3 从 KeyEvent 到 AppCommand/FormCommand

事件流（简化）：

```text
KeyEvent (crossterm)
    │
    ▼
InputRouter::classify
    │  使用 KeymapStore::classify（KeyPattern 匹配）
    ▼
KeyAction
    │
    ▼
KeyBindingMap::resolve
    │
    ▼
CommandDispatch::{Form(FormCommand), App(AppCommand)}
    │
    ├─ FormCommand  → FormEngine & FormState
    └─ AppCommand   → App::handle_app_command / handle_overlay_app_command
```

区别：

- `FormCommand` – 修改`FormState`（焦点变化、值编辑、列表操作）。
- `AppCommand` – 控制浮层、弹出窗口、保存/退出、帮助浮层等。

由于上下文**仅**影响帮助，分类管道保持简单且可预测。

---

## 4. 概念 → 代码 → 键位映射映射

下表总结了主要 UI 概念如何映射到代码和键位映射上下文：

| 概念     | 模式/布局                             | 运行时结构                                                    | 视图组件                                  | 典型上下文         |
| -------- | ------------------------------------- | ------------------------------------------------------------- | ----------------------------------------- | ------------------ |
| 主页面   | 根对象属性 → 根和章节                 | `FormState`, `RootSectionState`, `SectionState`, `FieldState` | `components::body`, `frame::draw`         | `Default`          |
| 章节     | 嵌套对象/组                           | `SectionState { depth, path }`                                | `body`中的章节标题                        | `Default`          |
| 子章节   | `depth > 0`的嵌套章节                 | 与章节相同，但`depth`更深                                     | 缩进/面包屑章节标题                       | `Default`          |
| 字段     | 叶子模式节点（字符串、枚举、复合...） | `FieldState` + `FieldComponent`                               | `body`中的字段行                          | `Default`          |
| 条目     | 数组项/映射条目/复合列表项            | `CompositeListState`、键/值状态、标量数组状态                 | 条目标题/条带 + 内联摘要                  | `Collection`       |
| 弹出窗口 | 枚举/变体选择器                       | `PopupState`, `AppPopup`, `PopupOwner`                        | `components::popup`                       | （模态，无上下文） |
| 浮层     | 复合编辑器/嵌套表单                   | `CompositeEditorOverlay`, `overlay_stack`, 嵌套`FormState`    | `components::overlay`, `CompositeOverlay` | `Overlay`          |
| 帮助浮层 | 不适用                                | `App`中的`HelpOverlayState`                                   | `components::help`, `HelpOverlayRender`   | 所有（按上下文）   |

此表是入职新工程师或审查重构时的好起点。

---

## 5. 重构与扩展指南

本节提供了典型 TUI 更改的具体、非详尽检查清单。

### 5.1 添加或更改键绑定

1. **添加或更新** `keymap/default.keymap.json`中的条目：

   ```json
   {
     "id": "help.toggle",
     "description": "Toggle help overlay",
     "contexts": ["default", "collection", "overlay"],
     "action": { "kind": "showHelp" },
     "combos": ["Ctrl+?"]
   }
   ```

2. **确保** `RawAction` → `KeyAction`映射支持该操作（`src/tui/app/keymap.rs`）：

   ```rust
   #[serde(tag = "kind", rename_all = "camelCase")]
   enum RawAction {
       Save,
       Quit,
       ResetStatus,
       TogglePopup,
       EditComposite,
       ShowHelp,
       // ...
   }

   impl RawAction {
       fn into_action(self) -> KeyAction {
           match self {
               RawAction::ShowHelp => KeyAction::ShowHelp,
               // ...
           }
       }
   }
   ```

3. **映射**
   `KeyAction`到`CommandDispatch`于`KeyBindingMap::resolve`（`src/tui/app/input.rs`）：

   ```rust
   KeyAction::ShowHelp => self
       .bindings
       .get(&KeyActionDiscriminant::ShowHelp)
       .cloned()
       .unwrap_or(CommandDispatch::App(AppCommand::ShowHelp)),
   ```

4. **处理**
   `AppCommand`于`App::handle_app_command`或`handle_overlay_app_command`（`src/tui/app/runtime/mod.rs`）。

### 5.2 添加新的基于浮层的编辑器

假设你想要一个用于复杂字段的新浮层类型。

高层步骤：

1. **在布局中建模字段**：
   - 如有必要，在`tui::model::layout`中扩展`FieldKind` / `FieldSchema`。
   - 确保`detect_kind`识别新类型。

2. **添加`FieldComponent`**实现于`src/tui/state/field/components`并将其接入`FieldState::from_schema`。

3. **定义浮层状态**：
   - 在`src/tui/app/runtime/overlay`中扩展`CompositeOverlayTarget` /
     `OverlayHost`。
   - 如有需要，向浮层会话添加字段。

4. **打开浮层**：
   - 如无，更新`try_open_composite_editor`或在`overlay/app/open.rs`中添加新助手以从聚焦字段实例化新浮层。

5. **渲染它**：
   - 如果布局不同，在`src/tui/view/components/overlay.rs`中扩展`CompositeOverlay`
     / `render_composite_overlay`。

6. **接入命令**：
   - 尽可能复用现有`AppCommand`/`FormCommand`（KISS 原则）。
   - 仅当行为无法通过列表/字段编辑表达时才引入新命令。

### 5.3 保持文档与代码同步

当你更改 TUI 或键位映射时，请更新：

- `README.md`和`README.ZH.md`（高层快照）。
- `docs/en/structure_design.md` / `docs/zh/structure_design.md`（架构/深入）。
- `docs/en/cli_usage.md` / `docs/zh/cli_usage.zh.md`（CLI 行为）。
- 如果概念或上下文更改，本文档。

避免在许多文档中重复低级细节；将**流程图和重构说明**集中在此和`structure_design.md`中。

---

## 6. 相关文档

- `README.md` – 项目概览和架构快照。
- `docs/en/structure_design.md` – 完整架构和管道设计。
- `docs/en/cli_usage.md` – CLI 使用、标志和示例。

此概览旨在成为新贡献者和现有贡献者处理 TUI
及其键盘交互时的**简短、准确的入口**。
