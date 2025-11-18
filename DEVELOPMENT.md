# SchemaUI 开发指南

## 🚀 快速开始

### 环境要求

- Rust 1.70+
- Node.js 18+
- pnpm 8+

### 安装依赖

```bash
# Rust 依赖
cargo build

# Web UI 依赖
cd web/ui && pnpm install

# 测试依赖
cd tests && npm install
```

### 构建项目

```bash
# 使用 just（推荐）
just build

# 或手动构建
cargo build -p schemaui-cli -F full
cd web/ui && pnpm build
```

## 📁 项目结构

```
schemaui/
├── src/                    # Rust 核心库源码
│   ├── ui_ast/            # UI AST 生成器
│   ├── app/               # 应用逻辑
│   ├── form/              # 表单处理
│   └── domain/            # 领域模型
├── schemaui-cli/          # CLI 应用
│   ├── src/
│   │   ├── tui/          # 终端 UI
│   │   └── web/          # Web 服务器
│   └── tests/
├── web/                   # Web UI
│   ├── ui/               # React 应用
│   │   ├── src/
│   │   │   ├── components/   # React 组件
│   │   │   ├── utils/       # 工具函数
│   │   │   └── ui-ast.ts   # UI AST 类型定义
│   │   └── dist/           # 构建输出
│   └── types/             # TypeScript 类型
├── tests/                 # 测试套件
│   ├── e2e/              # 端到端测试
│   ├── web-ui/           # Web UI 测试
│   └── schemas/          # 测试 Schema
├── docs/                  # 文档
│   ├── en/               # 英文文档
│   ├── zh/               # 中文文档
│   └── fixes/            # 修复报告
├── scripts/              # 实用脚本
└── examples/             # 示例 Schema 文件
```

## 🔧 开发工作流

### 1. 本地开发

启动开发服务器：

```bash
# 方式一：使用 just
just dev

# 方式二：手动启动
./target/debug/schemaui web -s examples/complex.schema.json --port 5175
```

### 2. 代码修改

#### Rust 代码

```bash
# 修改后重新编译
cargo build -p schemaui-cli

# 运行测试
cargo test
```

#### Web UI

```bash
# 开发模式（热重载）
cd web/ui && pnpm dev

# 生产构建
cd web/ui && pnpm build
```

### 3. 测试

```bash
# 运行所有测试
just test

# 运行 E2E 测试
cd tests && npm run test:e2e

# 运行特定测试
cargo test test_name
```

## 🏗️ 架构说明

### UI AST (用户界面抽象语法树)

UI AST 是 SchemaUI 的核心概念，它将 JSON Schema 转换为可渲染的 UI 结构：

```
JSON Schema → UI AST → TUI/Web UI
```

关键文件：

- `src/ui_ast/mod.rs` - Rust 实现
- `web/ui/src/ui-ast.ts` - TypeScript 类型定义

### 组件架构

#### NodeRenderer

核心渲染组件，负责根据 UI AST 节点渲染相应的表单控件。

关键功能：

- 字段渲染
- 数组处理
- 复合类型（oneOf/anyOf）
- 变体匹配

#### 变体匹配系统

处理 oneOf/anyOf 的智能匹配：

- `variantMatch.ts` - 匹配逻辑
- `variantDefault` - 默认值生成

## 🐛 调试技巧

### 启用日志

```bash
# Rust 日志
RUST_LOG=debug cargo run

# JavaScript 控制台日志
# 在代码中添加 console.log
```

### 使用开发者工具

1. 打开 Chrome DevTools (F12)
2. 查看 Console 面板的日志
3. 使用 Network 面板监控 WebSocket 通信
4. React DevTools 检查组件状态

### 常见问题调试

#### 数字输入问题

检查点：

- `NodeRenderer.tsx` 的 onChange 处理器
- 指针路径处理逻辑
- 事件传播链

#### 变体切换问题

检查点：

- `variantMatch.ts` 的匹配逻辑
- 默认值唯一性
- React key 属性

## 🧪 测试策略

### 单元测试

- Rust: `cargo test`
- TypeScript: `npm test`

### 集成测试

- API 测试
- WebSocket 通信测试
- Schema 验证测试

### E2E 测试

- Puppeteer 自动化
- 用户流程验证
- 跨浏览器测试

## 📝 代码规范

### Rust

- 使用 `rustfmt` 格式化
- 遵循 Rust API Guidelines
- 添加文档注释

```rust
/// 处理 UI 节点渲染
///
/// # Arguments
/// * `node` - UI AST 节点
/// * `value` - 当前值
pub fn render_node(node: &UiNode, value: &Value) -> Result<()> {
    // 实现
}
```

### TypeScript/React

- 使用 ESLint + Prettier
- 函数组件 + Hooks
- TypeScript 严格模式

```typescript
interface Props {
  node: UiNode;
  value: JsonValue;
  onChange: (pointer: string, value: JsonValue) => void;
}

export const Component: React.FC<Props> = ({ node, value, onChange }) => {
  // 实现
};
```

## 🚢 发布流程

1. **更新版本号**

```bash
# Cargo.toml
version = "0.3.4"

# package.json
"version": "0.3.4"
```

2. **运行测试**

```bash
just test-all
```

3. **构建发布版本**

```bash
cargo build --release
cd web/ui && pnpm build
```

4. **创建标签**

```bash
git tag v0.3.4
git push origin v0.3.4
```

## 🤝 贡献指南

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

### 提交信息格式

```
type(scope): subject

body

footer
```

类型：

- `feat`: 新功能
- `fix`: Bug 修复
- `docs`: 文档更新
- `style`: 代码格式
- `refactor`: 重构
- `test`: 测试
- `chore`: 构建/工具

## 📚 资源链接

- [JSON Schema 规范](https://json-schema.org/)
- [Rust 文档](https://doc.rust-lang.org/)
- [React 文档](https://react.dev/)
- [Ratatui TUI 框架](https://ratatui.rs/)

## 📄 许可证

MIT / Apache-2.0 双许可

---

_更多信息请参考项目 README 和文档。_
