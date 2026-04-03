# SchemaUI 脚本目录

本目录包含用于构建、测试和部署 SchemaUI 的实用脚本。

## 📋 脚本列表

### 构建脚本

#### build-web.sh

构建 Web UI 界面的脚本。

```bash
./scripts/build-web.sh
```

- 清理旧的构建文件
- 运行 pnpm build:embedded
- 生成生产环境的静态文件

### 更新脚本

#### update-cli-dependency.sh

更新 CLI 依赖的脚本。

```bash
./scripts/update-cli-dependency.sh
```

- 更新 Cargo.toml 中的依赖版本
- 确保依赖兼容性
- 依赖 `python3`（可通过 `PYTHON_BIN` 环境变量覆盖）

#### update-readme-version.sh

更新 README 文件中版本号的脚本。

```bash
./scripts/update-readme-version.sh <new-version>
```

- 自动更新所有 README 中的版本号
- 保持文档版本一致性
- 依赖 `python3`（可通过 `PYTHON_BIN` 环境变量覆盖）

#### sync-package-manifests.py

从已发布的 `schemaui-cli` GitHub release 拉取真实 asset URL / SHA256，
并同步生成 Homebrew / Scoop / winget 分发文件。

```bash
python3 scripts/sync-package-manifests.py --tag schemaui-cli-v0.4.1
python3 scripts/sync-package-manifests.py --tag schemaui-cli-v0.4.1 --check
```

- 默认读取 `schemaui-cli/Cargo.toml` 的版本
- `--check` 只校验是否同步，不改文件
- 使用 `GITHUB_TOKEN` / `GH_TOKEN` 可提升 GitHub API 额度

#### sync-install-docs.py

从 `packaging/install/install-methods.json` 读取安装方式定义，并回写：

- `README.md`
- `README.ZH.md`
- `docs/en/cli_usage.md`

```bash
python3 scripts/sync-install-docs.py
python3 scripts/sync-install-docs.py --check
```

- 使用显式 marker block，只更新 CLI 快捷入口与 installation section
- `--check` 只校验文档是否已同步，不改文件
- 写回文档时会使用 `deno fmt` 归一化 Markdown 输出；`--check` 可在无 `deno`
  环境下运行

## 🚀 发布入口

如果要用 `cargo release` 发布 `schemaui-cli`，仓库已经把 tag 约定和 GitHub
workflow 串好了：

```bash
just release-cli-dry-run patch
just release-cli patch
```

- `just release-cli` 等价于
  `cargo release <level> --package schemaui-cli --execute`
- 发布时会运行 `release.toml` 里的 pre-release hook，更新 web bundle / CLI 依赖
  / README 版本
- 推送到 GitHub 后：
  - `push main` 会触发 `prek-checks`、`release-plz-pr`、`release-plz-release`
  - `push schemaui-cli-v* tag` 会自动创建 GitHub Release，并继续触发 `CD`

## 🐛 故障排除

### 常见问题

1. **权限错误**

```bash
chmod +x scripts/*.sh
```

2. **pnpm 未找到**

```bash
npm install -g pnpm
```

3. **端口已占用**

```bash
lsof -i :5175  # 查看占用端口的进程
kill -9 <PID>  # 终止进程
```

## 📚 相关文档

- [构建文档](../docs/en/structure_design.md)
- [测试文档](../tests/README.md)
- [justfile](../justfile) - Make 替代工具配置
