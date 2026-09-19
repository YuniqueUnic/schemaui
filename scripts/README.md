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

#### feature-matrix.sh

运行 Rust feature matrix 编译测试的脚本。

```bash
./scripts/feature-matrix.sh smoke
./scripts/feature-matrix.sh exhaustive
```

- `smoke`：跑默认 smoke matrix
- `exhaustive`：设置 `SCHEMAUI_EXHAUSTIVE_FEATURE_MATRIX=1` 跑全量合法组合
- 优先使用 `uv run --with pytest`；若本机已安装 `python3 + pytest` 也可直接运行
- 默认开启 live progress 输出，避免多轮 `cargo check` 时看起来像“卡住”

#### deps-duplicates.sh

查看 workspace 全量 feature 图中的重复依赖：

```bash
./scripts/deps-duplicates.sh
```

- 等价于 `cargo tree --duplicates --workspace --all-features`
- 用于配合 compile-time 优化与依赖审计

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

#### sync-release-to-gitee.py

把已发布的 `schemaui-cli` release（说明 + 全部 asset）镜像到 Gitee 仓库
`Credhat/schemaui`。github.com 在国内不稳定，`a2ui-ask` 的安装脚本会在 GitHub
不可达时回退到该镜像，因此镜像必须持有同名 tag 下的同名 asset。

```bash
python3 scripts/sync-release-to-gitee.py --tag schemaui-cli-v0.8.0
python3 scripts/sync-release-to-gitee.py --tag schemaui-cli-v0.8.0 --dry-run
python3 scripts/sync-release-to-gitee.py --tag schemaui-cli-v0.8.0 --check

# 回填多个历史 release（单个 --tag 内用空格或逗号分隔）
python3 scripts/sync-release-to-gitee.py --tag "schemaui-cli-v0.7.0 schemaui-cli-v0.7.1"
```

- **所有模式（含 `--check` / `--dry-run`）都需要 `GITEE_TOKEN`**（Gitee
  私人令牌，需 `projects` 权限）；CI 中已配置为同名 repository secret。Gitee
  对匿名调用限流很严，一个回填跑到中途就会 403，所以直接要求令牌而不是半路失败
- 使用 `GITHUB_TOKEN` 可提升 GitHub API 额度（匿名仅 60 次/小时，回填会撞到）
- 幂等：已存在的 release 会复用，已上传的 asset 会跳过，中断后可安全重跑
- 上传中途连接被 Gitee 断开时，会先回查该 release 实际持有的 asset 再决定是否
  重传——断连时文件往往已经存进去了，盲目重传会产生重复附件
- **多个 tag 会按版本从旧到新处理**，不要依赖传入顺序。Gitee 的 release 列表按
  镜像记录的创建时间倒序排列，若先创建最新版，它就会被压到列表最底部（翻页才
  看得到）
- 同名附件若出现多份（例如两个回填进程并行），会自动只保留一份：优先保留大小与
  GitHub 一致的副本，其余删除
- `--check` 只校验镜像是否与 GitHub 一致，发现任何差异（缺失 / 重复 / 大小不符）
  时以非零码退出，供 CI 断言
- `--gitee-repo` 可覆盖镜像仓库；release 关联的提交取自 tag 自身指向的 commit
  （会解引用附注标签），不用分支名，否则 Gitee 上若缺该 tag 会被建到错误位置
- **前提：Gitee 必须已持有该 tag 指向的 commit**。创建 release 会让 Gitee 去建
  tag，而它建不了自己仓库里没有的 commit——此时返回 `400 创建标签失败`，只提
  tag、不提真正原因。Gitee 自身的 git 同步跟不上 release-plz 推的提交，所以
  `cd.yml` 在调用本脚本前会先把该 tag 推过去；手工回填若撞到这个 400，先执行
  `git push https://gitee.com/Credhat/schemaui.git refs/tags/<tag>` 再重跑
- 由 `cd.yml` 的 `mirror-to-gitee` 作业在 `upload-assets` 之后自动调用；历史
  release 用 `workflow_dispatch` 的 `gitee_tags` 输入触发
  `mirror-to-gitee-manual` 回填

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
  - `CD` 里 `upload-assets` 上传完预编译产物后，`mirror-to-gitee` 会把整个
    release 镜像到 Gitee，无需手动操作

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
- `just feature-matrix` / `just feature-matrix-exhaustive`
- `just deps-duplicates`
