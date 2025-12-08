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
