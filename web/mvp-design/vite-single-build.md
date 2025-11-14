# // ============================================================================ // web-ui/vite.config.js - Vite 配置（完全离线单文件构建） //

import { defineConfig } from 'vite'; import react from '@vitejs/plugin-react';
import { viteSingleFile } from 'vite-plugin-singlefile';

export default defineConfig({ plugins: [ react(), viteSingleFile({ //
使用推荐的构建配置 useRecommendedBuildConfig: true, // 移除 Vite
模块加载器（因为所有东西都内联了）removeViteModuleLoader: true, //
内联所有动态导入 inlinePattern: [], // 删除内联后的文件 deleteInlinedFiles:
true, }), ],

build: { // 目标浏览器（现代浏览器）target: 'esnext',

    // 输出目录
    outDir: 'dist',

    // 资源内联限制（设置为非常大的值以内联所有资源）
    assetsInlineLimit: 100000000, // 100MB - 确保所有资源都内联

    // 禁用 CSS 代码拆分（所有 CSS 打包到一个文件）
    cssCodeSplit: false,

    // 生成 source map（可选，用于调试）
    sourcemap: false,

    // 清空输出目录
    emptyOutDir: true,

    // Rollup 选项
    rollupOptions: {
      // 输出配置
      output: {
        // 内联动态导入
        inlineDynamicImports: true,

        // 手动分块 - 所有代码打包到同一个 chunk
        manualChunks: undefined,

        // 资产文件命名
        assetFileNames: 'assets/[name].[hash][extname]',

        // chunk 文件命名
        chunkFileNames: 'assets/[name].[hash].js',

        // 入口文件命名
        entryFileNames: 'assets/[name].[hash].js',
      },

      // 外部依赖（留空表示打包所有依赖）
      external: [],
    },

    // 最小化配置
    minify: 'esbuild',

    // 报告压缩后的大小
    reportCompressedSize: true,

    // chunk 大小警告限制（单文件会很大，所以提高限制）
    chunkSizeWarningLimit: 5000, // 5MB

},

// CSS 配置 css: { // PostCSS 配置（Tailwind）postcss: './postcss.config.js', },

// 解析配置 resolve: { alias: { '@': '/src', }, },

// 开发服务器配置 server: { port: 5173, strictPort: false, open: true, },

// 预览服务器配置 preview: { port: 4173, }, });

# // ============================================================================ // web-ui/package.json - 依赖配置 //

const packageJson = { "name": "schemaui-web", "version": "0.1.0", "type":
"module", "scripts": { "dev": "vite", "build": "vite build", "preview": "vite
preview", "lint": "eslint src --ext js,jsx", "format": "prettier --write src" },
"dependencies": { "react": "^18.3.1", "react-dom": "^18.3.1", "lucide-react":
"^0.263.1" }, "devDependencies": { "@vitejs/plugin-react": "^4.3.4", "vite":
"^5.4.11", "vite-plugin-singlefile": "^2.0.2", "autoprefixer": "^10.4.20",
"postcss": "^8.4.49", "tailwindcss": "^3.4.17", "eslint": "^8.57.0", "prettier":
"^3.3.3" } };

# // ============================================================================ // web-ui/postcss.config.js - PostCSS 配置 //

export default { plugins: { tailwindcss: {}, autoprefixer: {}, }, };

# // ============================================================================ // web-ui/tailwind.config.js - Tailwind CSS 配置 //

export default { content: [ "./index.html", "./src/**/*.{js,jsx,ts,tsx}", ],
theme: { extend: { colors: { // 自定义颜色方案 primary: { 50: '#f0f9ff', 100:
'#e0f2fe', 200: '#bae6fd', 300: '#7dd3fc', 400: '#38bdf8', 500: '#0ea5e9', 600:
'#0284c7', 700: '#0369a1', 800: '#075985', 900: '#0c4a6e', }, }, backdropBlur: {
xs: '2px', }, }, }, plugins: [], };

# // ============================================================================ // web-ui/.prettierrc - Prettier 配置 //

const prettierConfig = { "semi": true, "singleQuote": true, "tabWidth": 2,
"trailingComma": "es5", "printWidth": 100, "arrowParens": "avoid" };

# // ============================================================================ // web-ui/index.html - HTML 模板 //

const indexHtml = `

<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <meta name="description" content="SchemaUI Web - Interactive JSON Schema Editor" />
  <title>SchemaUI Web</title>

<!-- 所有样式会被内联到这里 -->
</head>
<body>
  <div id="root"></div>

<!-- 所有 JavaScript 会被内联到这里 -->
<script type="module" src="/src/main.jsx"></script>
</body>
</html>
`;

# // ============================================================================ // web-ui/src/main.jsx - 应用入口 //

const mainJsx = ` import React from 'react'; import ReactDOM from
'react-dom/client'; import App from './App'; import './index.css';

// 渲染应用 ReactDOM.createRoot(document.getElementById('root')).render(
<React.StrictMode>
<App /> </React.StrictMode> ); `;

# // ============================================================================ // web-ui/src/index.css - 全局样式 //

const indexCss = ` /* Tailwind 指令 */ @tailwind base; @tailwind components;
@tailwind utilities;

/* 全局样式 */

- { margin: 0; padding: 0; box-sizing: border-box; }

html, body, #root { height: 100%; overflow: hidden; }

body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto',
'Oxygen', 'Ubuntu', 'Cantarell', 'Fira Sans', 'Droid Sans', 'Helvetica Neue',
sans-serif; -webkit-font-smoothing: antialiased; -moz-osx-font-smoothing:
grayscale; }

code { font-family: 'Fira Code', 'Monaco', 'Courier New', monospace; }

/* 自定义滚动条 */ ::-webkit-scrollbar { width: 8px; height: 8px; }

::-webkit-scrollbar-track { background: rgba(30, 41, 59, 0.3); border-radius:
4px; }

::-webkit-scrollbar-thumb { background: rgba(148, 163, 184, 0.4); border-radius:
4px; transition: background 0.2s; }

::-webkit-scrollbar-thumb:hover { background: rgba(148, 163, 184, 0.6); }

/* 选中文本样式 */ ::selection { background: rgba(14, 165, 233, 0.3); color:
inherit; }

/* 禁用文本选择（某些 UI 元素） */ .no-select { -webkit-user-select: none;
-moz-user-select: none; -ms-user-select: none; user-select: none; }

/* 动画 */ @keyframes fadeIn { from { opacity: 0; transform: translateY(-10px);
} to { opacity: 1; transform: translateY(0); } }

.animate-fade-in { animation: fadeIn 0.3s ease-out; }

@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.5; } }

.animate-pulse { animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite; } `;

# // ============================================================================ // 构建脚本 - build.sh //

const buildScript = `#!/bin/bash

# web-ui/build.sh - 构建完全离线的单页应用

set -e

echo "🔨 Building SchemaUI Web Interface..." echo
"━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 1. 检查依赖

if [ ! -d "node_modules" ]; then echo "📦 Installing dependencies..." npm
install fi

# 2. 清理旧构建

echo "🧹 Cleaning old build..." rm -rf dist

# 3. 构建应用

echo "⚡ Building application..." npm run build

# 4. 验证输出

if [ -f "dist/index.html" ]; then FILE_SIZE=$(du -h "dist/index.html" | cut -f1)
  echo "✅ Build successful!"
  echo "📦 Output: dist/index.html ($FILE_SIZE)"

# 检查是否完全内联

if grep -q "src=\"" dist/index.html || grep -q "href=\"" dist/index.html; then
echo "⚠️ Warning: Found external references in HTML" grep -n "src=\"\\|href=\""
dist/index.html || true else echo "✅ All assets inlined successfully!" fi

# 统计

echo "" echo "📊 Build Statistics:" echo
"━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" echo "Total size: $FILE_SIZE"
echo "Files in dist: $(ls -1 dist | wc -l)"

else echo "❌ Build failed!" exit 1 fi

echo "" echo "🎉 Done! Ready to embed in Rust binary." `;

# // ============================================================================ // 测试脚本 - test-offline.sh //

const testOfflineScript = `#!/bin/bash

# web-ui/test-offline.sh - 测试离线功能

set -e

echo "🧪 Testing offline functionality..." echo
"━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 1. 构建应用

./build.sh

# 2. 检查外部依赖

echo "🔍 Checking for external dependencies..."

EXTERNAL_LINKS=$(grep -o 'https\\?://[^"\\x27]*' dist/index.html | wc -l)

if [ "$EXTERNAL_LINKS" -eq 0 ]; then echo "✅ No external links found - fully
offline!" else echo "❌ Found $EXTERNAL_LINKS external links:" grep -o
'https\\?://[^"\\x27]*' dist/index.html exit 1 fi

# 3. 启动本地服务器（模拟离线环境）

echo "" echo "🌐 Starting local server..." echo "Press Ctrl+C to stop" echo ""

cd dist python3 -m http.server 8888 --bind 127.0.0.1 `;

# // ============================================================================ // GitHub Actions CI - .github/workflows/build.yml //

const githubActions = ` name: Build Web UI

on: push: branches: [ main ] paths: - 'web-ui/**' pull_request: branches: [ main
] paths: - 'web-ui/**'

jobs: build: runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v4

    - name: Setup Node.js
      uses: actions/setup-node@v4
      with:
        node-version: '20'
        cache: 'npm'
        cache-dependency-path: web-ui/package-lock.json

    - name: Install dependencies
      working-directory: web-ui
      run: npm ci

    - name: Build
      working-directory: web-ui
      run: npm run build

    - name: Verify single file output
      working-directory: web-ui
      run: |
        if [ ! -f "dist/index.html" ]; then
          echo "❌ Build output not found!"
          exit 1
        fi

        # Check for external references
        if grep -q 'src="http' dist/index.html || grep -q 'href="http' dist/index.html; then
          echo "❌ Found external references!"
          exit 1
        fi

        echo "✅ Build verified!"

    - name: Upload artifact
      uses: actions/upload-artifact@v4
      with:
        name: web-ui-build
        path: web-ui/dist/index.html
        retention-days: 30

`;

# // ============================================================================ // 使用说明 //

console.log(`

# SchemaUI Web - 完整构建配置

## 快速开始

1. 安装依赖：cd web-ui npm install

2. 开发模式：npm run dev

3. 构建生产版本：npm run build
   # 或者使用 shell 脚本
   ./build.sh

4. 测试离线功能： ./test-offline.sh

5. 预览构建结果：npm run preview

## 验证完全离线

构建完成后，检查 dist/index.html：

\`\`\`bash

# 1. 检查文件大小

ls -lh dist/index.html

# 2. 检查是否有外部引用

grep -n 'src="http\\|href="http' dist/index.html

# 3. 如果没有输出，说明完全离线！

\`\`\`

## 集成到 Rust

\`\`\`rust // schemaui/src/web/assets.rs use rust_embed::RustEmbed;

#[derive(RustEmbed)] #[folder = "../web-ui/dist"] pub struct WebAssets;

impl WebAssets { pub fn index_html() -> String { String::from_utf8(
Self::get("index.html") .expect("index.html must exist") .data .into_owned()
).expect("Valid UTF-8") } } \`\`\`

## 预期输出

- 单个 HTML 文件（约 300-800KB）
- 包含所有 JavaScript（React + 应用代码）
- 包含所有 CSS（Tailwind + 自定义样式）
- 包含所有图标（Lucide React 内联）
- 无任何外部依赖或 CDN 引用

## 优化建议

1. 如果文件太大（>1MB），考虑：
   - 移除未使用的 Tailwind 类
   - 使用更少的 Lucide 图标
   - 启用更激进的 minify

2. 如果需要更小的体积：
   - 使用 Preact 替代 React
   - 使用原生 CSS 替代 Tailwind
   - 手写高亮器而非使用库

3. 如果需要更快的加载：
   - 延迟加载非关键组件
   - 使用 code splitting（但会生成多个文件）
   - 使用 Web Workers（如果支持）`);

export { packageJson, prettierConfig, indexHtml, mainJsx, indexCss, buildScript,
testOfflineScript, githubActions };
