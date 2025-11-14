# SchemaUI Web 完整实现指南 🚀

## 📋 目录

1. [架构概览](#架构概览)
2. [完全离线实现方案](#完全离线实现方案)
3. [解决现有问题](#解决现有问题)
4. [项目结构](#项目结构)
5. [构建流程](#构建流程)
6. [集成步骤](#集成步骤)

---

## 🏗️ 架构概览

### 技术栈选择

**前端（完全离线）**

- React 18 - UI 框架
- Vite - 构建工具（支持单文件输出）
- Tailwind CSS - 样式系统（内联到 HTML）
- Prism.js - 语法高亮（离线版本）
- 无 CDN 依赖

**后端（Rust）**

- axum - Web 框架
- tokio - 异步运行时
- jsonschema - Schema 验证
- rust-embed - 资源嵌入
- tower-http - CORS 支持

### 数据流

```
用户输入 → React 前端 → WebSocket/HTTP API → Rust 后端
                                                    ↓
                                              JSON Schema 验证
                                                    ↓
                                              返回验证结果
                                                    ↓
                                            实时更新 UI（inline 显示）
```

---

## 🔒 完全离线实现方案

### 1. 构建单文件 HTML（关键！）

使用 `vite-plugin-singlefile` 插件：

```javascript
// web-ui/vite.config.js
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { viteSingleFile } from "vite-plugin-singlefile";

export default defineConfig({
  plugins: [
    react(),
    viteSingleFile({
      removeViteModuleLoader: true,
      useRecommendedBuildConfig: true,
    }),
  ],
  build: {
    target: "esnext",
    assetsInlineLimit: 100000000, // 内联所有资源
    cssCodeSplit: false,
    rollupOptions: {
      output: {
        inlineDynamicImports: true,
      },
    },
  },
});
```

### 2. 内联语法高亮库

两种方案：

**方案 A：使用轻量级库（推荐）**

```javascript
// 自实现的极简语法高亮（已在 React 组件中实现）
// 优点：完全可控，无依赖，体积小
// 缺点：功能有限，但足够用
```

**方案 B：内联 Prism.js**

```html
<!-- 在构建时将 Prism.js 核心 + 语言包内联 -->
<script>
  // Prism.js 核心代码（复制到这里）
  // JSON、YAML、TOML 语言定义
</script>
```

### 3. 资源嵌入到 Rust 二进制

```rust
// 使用 rust-embed 将 HTML 打包进二进制
#[derive(RustEmbed)]
#[folder = "web-ui/dist"]
pub struct WebAssets;

// 构建时自动包含所有文件
// 运行时无需外部文件系统访问
```

### 4. 验证完全离线

```bash
# 断网测试
sudo ifconfig en0 down  # macOS
# 或
sudo ip link set eth0 down  # Linux

# 运行服务器
cargo run -p schemaui-cli -- web --schema test.json

# 应该完全正常工作！
```

---

## 🔧 解决现有问题

### 问题 1：输入时失去 focus

**原因**：实时验证触发 React 重新渲染，导致输入框重新创建

**解决方案**：

```javascript
// ❌ 错误做法：每次渲染都创建新组件
const FieldEditor = ({ field, value, onChange }) => {
  return <input value={value} onChange={onChange} />;
};

// ✅ 正确做法：使用 useRef 保持引用稳定
const FieldEditor = ({ field, value, onChange, error }) => {
  const inputRef = useRef(null);

  // 使用防抖避免过于频繁的验证
  const debouncedOnChange = useMemo(
    () => debounce(onChange, 300),
    [onChange],
  );

  return (
    <input
      ref={inputRef}
      value={value}
      onChange={(e) => {
        // 立即更新显示
        inputRef.current.value = e.target.value;
        // 延迟验证
        debouncedOnChange(e.target.value);
      }}
    />
  );
};
```

**更好的方案：受控组件 + 去抖验证**

```javascript
const [formData, setFormData] = useState({});
const [validationErrors, setValidationErrors] = useState([]);

// 验证使用 useEffect + 防抖
useEffect(() => {
  const timer = setTimeout(async () => {
    const result = await validateData(formData);
    setValidationErrors(result.errors);
  }, 300); // 300ms 防抖

  return () => clearTimeout(timer);
}, [formData]);

// onChange 只更新数据，不触发验证
const handleChange = (field, value) => {
  setFormData((prev) => ({ ...prev, [field]: value }));
};
```

### 问题 2：顶栏设计问题

**解决方案：固定高度 + Flexbox 布局**

```css
/* ✅ 正确的顶栏设计 */
.header {
  height: 56px; /* 固定高度 */
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
  background: rgba(30, 41, 59, 0.8);
  backdrop-filter: blur(12px);
  border-bottom: 1px solid rgba(148, 163, 184, 0.1);
}

.header-actions {
  display: flex;
  gap: 12px;
  align-items: center;
}

button {
  height: 36px; /* 固定按钮高度 */
  padding: 0 16px;
  white-space: nowrap;
}
```

### 问题 3：单页滚动问题

**解决方案：正确的布局结构**

```jsx
// ✅ 正确的单页布局（已实现）
<div className="h-screen flex flex-col">
  {/* 占满视口 */}
  {/* 顶栏：固定高度 */}
  <header className="h-14 flex-shrink-0">
    ...
  </header>

  {/* 主内容：flex-1 占据剩余空间 */}
  <main className="flex-1 flex overflow-hidden">
    {/* overflow-hidden 关键！ */}
    {/* 左侧导航：独立滚动 */}
    <aside className="w-72 overflow-y-auto">
      ...
    </aside>

    {/* 中间编辑器：独立滚动 */}
    <section className="flex-1 overflow-y-auto">
      ...
    </section>

    {/* 右侧预览：独立滚动 */}
    <aside className="w-96 overflow-y-auto">
      ...
    </aside>
  </main>

  {/* 状态栏（可选）：固定在底部 */}
  <footer className="h-8 flex-shrink-0">
    ...
  </footer>
</div>;
```

**关键点**：

- 使用 `h-screen` 确保容器占满视口
- 使用 `overflow-hidden` 在主容器上阻止页面滚动
- 每个子区域独立使用 `overflow-y-auto` 实现内部滚动

### 问题 4：预览区语法高亮和 TOML 支持

**完整实现（已包含在 React 组件中）**：

```javascript
const SyntaxHighlight = ({ code, language }) => {
  const highlightJSON = (text) => {
    return text
      .replace(/("[\w\d_-]+")\s*:/g, '<span class="text-cyan-400">$1</span>:')
      .replace(/:\s*(".*?")/g, ': <span class="text-green-400">$1</span>')
      .replace(/:\s*(\d+\.?\d*)/g, ': <span class="text-orange-400">$1</span>')
      .replace(
        /:\s*(true|false|null)/g,
        ': <span class="text-purple-400">$1</span>',
      );
  };

  const highlightTOML = (text) => {
    return text
      // 高亮 section headers [section]
      .replace(
        /^\[.*\]/gm,
        (match) => `<span class="text-purple-400">${match}</span>`,
      )
      // 高亮 keys
      .replace(/^(\w+)\s*=/gm, '<span class="text-cyan-400">$1</span> =')
      // 高亮 string values
      .replace(
        /=\s*".*?"/g,
        (match) => `= <span class="text-green-400">${match.slice(2)}</span>`,
      )
      // 高亮 numbers
      .replace(
        /=\s*\d+\.?\d*/g,
        (match) => `= <span class="text-orange-400">${match.slice(2)}</span>`,
      )
      // 高亮 booleans
      .replace(
        /=\s*(true|false)/g,
        (match) => `= <span class="text-purple-400">${match.slice(2)}</span>`,
      );
  };

  const highlightYAML = (text) => {
    return text
      .replace(/^(\s*[\w\d_-]+):/gm, '<span class="text-cyan-400">$1</span>:')
      .replace(/:\s*(".*?")/g, ': <span class="text-green-400">$1</span>')
      .replace(/:\s*(\d+\.?\d*)/g, ': <span class="text-orange-400">$1</span>')
      .replace(
        /:\s*(true|false|null)/g,
        ': <span class="text-purple-400">$1</span>',
      );
  };

  return (
    <pre className="text-sm leading-relaxed overflow-auto h-full p-4">
      <code dangerouslySetInnerHTML={{
        __html: language === 'json' ? highlightJSON(code)
              : language === 'yaml' ? highlightYAML(code)
              : language === 'toml' ? highlightTOML(code)
              : code
      }} />
    </pre>
  );
};

// 格式转换
const formatOutput = (data, format) => {
  switch (format) {
    case "json":
      return JSON.stringify(data, null, 2);

    case "yaml":
      // 简化的 YAML 输出
      const toYAML = (obj, indent = 0) => {
        const spaces = "  ".repeat(indent);
        return Object.entries(obj).map(([k, v]) => {
          if (typeof v === "object" && v !== null) {
            return `${spaces}${k}:\n${toYAML(v, indent + 1)}`;
          }
          return `${spaces}${k}: ${JSON.stringify(v)}`;
        }).join("\n");
      };
      return toYAML(data);

    case "toml":
      // 简化的 TOML 输出
      const toTOML = (obj, section = "") => {
        let result = [];
        for (const [k, v] of Object.entries(obj)) {
          if (typeof v === "object" && v !== null) {
            const newSection = section ? `${section}.${k}` : k;
            result.push(`\n[${newSection}]`);
            result.push(toTOML(v, newSection));
          } else {
            result.push(`${k} = ${JSON.stringify(v)}`);
          }
        }
        return result.join("\n");
      };
      return toTOML(data);
  }
};
```

### 问题 5：树状结构设计

**改进后的树状导航（已实现）**：

```javascript
// 递归树节点组件
const TreeNode = ({
  node,
  level = 0,
  selectedPath,
  onSelect,
  expandedNodes,
  onToggle,
}) => {
  const isExpanded = expandedNodes.has(node.path);
  const isSelected = selectedPath === node.path;
  const hasChildren = node.children?.length > 0;

  return (
    <div>
      {/* 节点本身 */}
      <div
        className={`
          flex items-center gap-2 py-2 px-3 cursor-pointer
          hover:bg-slate-700/50 rounded-lg transition-all
          ${isSelected ? "bg-cyan-500/20 border-l-2 border-cyan-400" : ""}
        `}
        style={{ paddingLeft: `${level * 16 + 12}px` }}
        onClick={() => onSelect(node.path)}
      >
        {/* 展开/折叠按钮 */}
        {hasChildren && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              onToggle(node.path);
            }}
          >
            {isExpanded ? <ChevronDown /> : <ChevronRight />}
          </button>
        )}

        {/* 节点标题 */}
        <span>{node.title}</span>

        {/* 必填标记 */}
        {node.required && <span className="text-red-400">*</span>}
      </div>

      {/* 子节点（递归） */}
      {isExpanded && hasChildren && (
        <div>
          {node.children.map((child) => (
            <TreeNode
              key={child.path}
              node={child}
              level={level + 1}
              selectedPath={selectedPath}
              onSelect={onSelect}
              expandedNodes={expandedNodes}
              onToggle={onToggle}
            />
          ))}
        </div>
      )}
    </div>
  );
};

// 从 Schema 构建树结构
const buildTreeFromSchema = (schema, path = "") => {
  if (!schema.properties) return [];

  return Object.entries(schema.properties).map(([key, prop]) => {
    const nodePath = path ? `${path}/${key}` : `/${key}`;
    return {
      path: nodePath,
      title: prop.title || key,
      type: prop.type,
      required: schema.required?.includes(key),
      children: prop.type === "object" && prop.properties
        ? buildTreeFromSchema(prop, nodePath)
        : [],
    };
  });
};
```

---

## 📁 项目结构

```
schemaui/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── tui/           # 现有 TUI 代码
│   └── web/           # 新增 Web 模块
│       ├── mod.rs
│       ├── server.rs  # Web 服务器
│       ├── assets.rs  # 资源嵌入
│       └── api.rs     # API 处理
├── web-ui/            # 前端源码
│   ├── package.json
│   ├── vite.config.js
│   ├── index.html
│   ├── src/
│   │   ├── App.jsx
│   │   ├── components/
│   │   │   ├── TreeNavigator.jsx
│   │   │   ├── FieldEditor.jsx
│   │   │   └── PreviewPane.jsx
│   │   └── utils/
│   │       ├── highlight.js
│   │       └── format.js
│   └── dist/          # 构建输出（单个 HTML）
│       └── index.html
└── schemaui-cli/
    └── src/
        └── commands/
            └── web.rs
```

---

## 🔨 构建流程

### 1. 前端构建

```bash
# web-ui/package.json
{
  "name": "schemaui-web",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "lucide-react": "^0.263.0"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "^4.0.0",
    "autoprefixer": "^10.4.14",
    "postcss": "^8.4.24",
    "tailwindcss": "^3.3.2",
    "vite": "^5.0.0",
    "vite-plugin-singlefile": "^0.13.5"
  }
}
```

```bash
cd web-ui
npm install
npm run build

# 检查输出
ls -lh dist/index.html
# 应该看到一个 ~200-500KB 的单文件（包含所有资源）
```

### 2. Rust 构建

```bash
# 构建带 web 功能的版本
cargo build --release --features web

# 验证资源已嵌入
cargo tree --features web | grep rust-embed
```

### 3. 测试完全离线

```bash
# 断网
sudo ifconfig en0 down  # 或其他网络接口

# 运行
./target/release/schemaui-cli web --schema test.json

# 应该完全正常工作，包括：
# - 页面加载
# - 语法高亮
# - 实时验证
# - 所有交互功能
```

---

## 🚀 集成步骤

### Step 1: 添加 Web 功能到库

```toml
# schemaui/Cargo.toml
[features]
default = ["tui"]
tui = ["ratatui", "crossterm"]
web = ["axum", "tokio", "tower-http", "rust-embed", "open"]

[dependencies]
# Web 依赖
axum = { version = "0.7", optional = true }
tokio = { version = "1", features = ["full"], optional = true }
tower-http = { version = "0.5", features = ["cors"], optional = true }
rust-embed = { version = "8.0", optional = true }
open = { version = "5.0", optional = true }
```

### Step 2: 实现 Web 模块

复制 Rust 实现代码到对应文件。

### Step 3: 更新 CLI

```rust
// schemaui-cli/src/main.rs
#[derive(Subcommand)]
enum Commands {
    Tui(TuiCommand),
    #[cfg(feature = "web")]
    Web(WebCommand),
}

#[tokio::main]
async fn main() -> Result<()> {
    match cli.command {
        #[cfg(feature = "web")]
        Commands::Web(cmd) => cmd.execute().await?,
        Commands::Tui(cmd) => cmd.execute()?,
    }
}
```

### Step 4: 构建前端

```bash
cd web-ui
npm install
npm run build
```

### Step 5: 测试端到端

```bash
# 编译
cargo build --release --features web

# 运行
./target/release/schemaui-cli web \
  --schema examples/service.schema.json \
  --config examples/service.config.json \
  --port 3000

# 在浏览器中编辑配置
# 点击 Save
# 点击 Exit

# 查看 terminal 输出的最终 JSON
```

---

## 🎨 设计系统

### 颜色方案

```javascript
const colors = {
  // 背景层次
  bg: {
    primary: "#0f172a", // slate-900
    secondary: "#1e293b", // slate-800
    tertiary: "#334155", // slate-700
  },

  // 强调色
  accent: {
    primary: "#0ea5e9", // cyan-500
    hover: "#06b6d4", // cyan-600
    light: "#22d3ee", // cyan-400
  },

  // 语义色
  semantic: {
    error: "#ef4444", // red-500
    warning: "#f59e0b", // amber-500
    success: "#10b981", // green-500
    info: "#3b82f6", // blue-500
  },

  // 文本
  text: {
    primary: "#f1f5f9", // slate-100
    secondary: "#cbd5e1", // slate-300
    tertiary: "#94a3b8", // slate-400
    disabled: "#64748b", // slate-500
  },
};
```

### 间距系统

```javascript
const spacing = {
  xs: "4px",
  sm: "8px",
  md: "16px",
  lg: "24px",
  xl: "32px",
  "2xl": "48px",
};
```

### 圆角系统

```javascript
const borderRadius = {
  sm: "6px",
  md: "8px",
  lg: "12px",
  xl: "16px",
};
```

---

## ✅ 验证清单

- [ ] 前端构建产生单个 HTML 文件
- [ ] HTML 文件不包含任何外部链接（CDN）
- [ ] 断网测试通过
- [ ] 语法高亮工作（JSON/YAML/TOML）
- [ ] 实时验证不导致输入框失去焦点
- [ ] 树状导航展开/折叠正常
- [ ] 单页滚动正确（内部滚动，非页面滚动）
- [ ] Save 按钮保存数据
- [ ] Exit 按钮关闭服务器并输出 JSON
- [ ] 所有交互都有微动画反馈
- [ ] 响应式设计（至少 1280px 宽度）

---

## 🎯 性能优化建议

1. **虚拟滚动**：如果树节点超过 1000 个，使用 react-window
2. **Web Worker**：将 JSON Schema 验证移到 Worker 线程
3. **增量验证**：只验证修改的字段，而非整个 schema
4. **缓存编译的 Schema**：JSONSchema 编译结果缓存在 Rust 后端

---

## 📚 参考资料

- [Vite Single File Plugin](https://github.com/richardtallent/vite-plugin-singlefile)
- [rust-embed](https://github.com/pyrossh/rust-embed)
- [axum Web Framework](https://github.com/tokio-rs/axum)
- [JSON Schema Specification](https://json-schema.org/)

---

## 🎉 完成！

现在你拥有了一个完全离线、功能完整、现代化的 Web 配置界面！
