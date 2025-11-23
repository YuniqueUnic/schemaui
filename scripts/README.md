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
- 运行 pnpm build
- 生成生产环境的静态文件

### 更新脚本

#### update-cli-dependency.sh

更新 CLI 依赖的脚本。

```bash
./scripts/update-cli-dependency.sh
```

- 更新 Cargo.toml 中的依赖版本
- 确保依赖兼容性

#### update-readme-version.sh

更新 README 文件中版本号的脚本。

```bash
./scripts/update-readme-version.sh <new-version>
```

- 自动更新所有 README 中的版本号
- 保持文档版本一致性

### 发布流程

1. **更新版本号**

```bash
./scripts/update-readme-version.sh 0.3.4
```

2. **构建生产版本**

```bash
just build
```

### 自定义配置

如需自定义脚本行为，可以创建本地配置文件：

```bash
# scripts/local.conf (git-ignored)
export PORT=3000
export SCHEMA_PATH=../my-schemas/custom.json
```

然后在脚本中引入：

```bash
source ./scripts/local.conf 2>/dev/null || true
```

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

## 📝 添加新脚本

创建新脚本时，请遵循以下规范：

1. **文件命名**
   - 使用小写字母和连字符
   - 以 `.sh` 结尾
   - 名称描述功能

2. **脚本头部**

```bash
#!/bin/bash
set -e  # 遇到错误立即退出

# 脚本描述
# 用法: ./scripts/script-name.sh [参数]
#
# 参数:
#   arg1 - 描述
#   arg2 - 描述
```

3. **错误处理**

```bash
error_exit() {
    echo "错误: $1" >&2
    exit 1
}

# 使用
command || error_exit "命令执行失败"
```

4. **日志输出**

```bash
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
}

log "开始执行..."
```

## 📚 相关文档

- [构建文档](../docs/en/structure_design.md)
- [测试文档](../tests/README.md)
- [justfile](../justfile) - Make 替代工具配置

## 🏷️ 版本历史

- v1.0.0 - 初始脚本集
  - 基础构建和测试脚本
  - 版本更新自动化

---

_提示：所有脚本都可以通过 `just` 命令调用，查看 justfile 了解更多。_
