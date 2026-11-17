# TUI 手动体验示例

本例集中演示 textarea、多种控件、条件显示、嵌套草稿，以及 SVG/Mermaid
图片与源码查看。所有内容在本地，不会请求外部图片或服务。

## 启动

在仓库根目录的交互终端运行，使用**当前工作树**的代码，不要用尚未包含本次改动的已安装版本。根据当前
shell 选择命令，fish 不能直接使用 Bash/Zsh 的变量赋值和 `${VAR:-default}` 语法。

### fish

```fish
set -l demo_dir (mktemp -d)
and mbx run -p schemaui-cli -- tui \
  --schema examples/tui-showcase/schema.json \
  --config examples/tui-showcase/config.json \
  --draft "$demo_dir/session.draft.json" \
  --output "$demo_dir/result.json"
and printf '\n保存结果：%s/result.json\n' "$demo_dir"
```

`mktemp -d` 自动在系统临时目录中创建独立目录，无需 Bash
风格的环境变量默认值表达式；`and` 只在上一步成功后继续。

### Bash / Zsh

```bash
demo_dir="$(mktemp -d "${TMPDIR:-/tmp}/schemaui-tui-demo.XXXXXX")"
mbx run -p schemaui-cli -- tui \
  --schema examples/tui-showcase/schema.json \
  --config examples/tui-showcase/config.json \
  --draft "$demo_dir/session.draft.json" \
  --output "$demo_dir/result.json" && \
printf '\n保存结果：%s/result.json\n' "$demo_dir"
```

- CLI 默认 full，包含 `tui-images` 和 `mermaid`，无需手动加 feature。
- 建议至少 100 列 × 30 行；小窗口也可试试缩放、滚动与焦点跟随。
- 每次命令新建临时目录，不覆盖示例配置、之前的结果或其他会话的草稿。只有成功保存才会生成结果文件。
- TUI 绘制到 stderr，它必须保持连接终端；可重定向 stdout，但不要使用 `2>&1`
  或把整个命令接到管道中采集界面。
- 根表单 `Ctrl+S` 校验并写入会话草稿，`Ctrl+Q` 会校验、保存并退出；退出后才写出
  `result.json`。校验失败时按状态提示修正；强制退出不等于保存成功。
- `config.json` 含相对 `$schema` 声明，可省略启动时的 `--schema`。CLI
  会提取并移除这一输入声明，它不是业务字段，不出现在保存结果中；重新打开结果时请显式提供仓库内的
  `--schema` 路径。

## 先看图片，再试控件

| 标签                    | 建议操作                                                                                | 应观察到的结果                                                                                        |
| ----------------------- | --------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Images / 图片与多行     | 首个笔记字段按 `Ctrl+P`，按 `f`，再用 `i` 放大、方向键移动；按 `s` 查看源码，最后 `Esc` | 适应窗口时图片水平、垂直居中；放大后可沿两轴滚动，返回后焦点仍在笔记字段                              |
| Images / 图片与多行     | 笔记中按 `Enter`、输入中文或粘贴多行，再按 `Ctrl+Z` / `Ctrl+Y`                          | Enter 真正换行；粘贴作为一个可撤销操作；Tab 才离开字段                                                |
| Images / 图片与多行     | Tab 到 Mermaid，修改源码，按 `Ctrl+P`                                                   | 预览当前草稿，不是初始图；Esc 返回继续编辑，不是逐键实时预览                                          |
| Images / 图片与多行     | Tab 到 SVG 选项，Enter 打开，Down 高亮 Blue，Ctrl+P 预览                                | 预览不应用选择；Esc 回选择器，再 Enter 确认后保存整数 `20`，而非标签                                  |
| Images / 图片与多行     | radio 上 Left/Right 切换，再 Ctrl+P                                                     | 查看选中项的 Mermaid 图；查看器内 `n`/`p` 或 Tab/BackTab 切到描述源码，方向键只平移图片               |
| Images / 图片与多行     | 最后一个 Markdown 字段按 Ctrl+P                                                         | 明确显示 Markdown 源码，不把表格源码冒称为渲染结果                                                    |
| Controls / 控件         | slider 用 Left/Right；range 用 Up/Down 选端点，再 Left/Right                            | slider 按 `0.05` 步进；range 两端点可独立编辑，不会被方向键推成交叉范围                               |
| Controls / 控件         | 改 segmented、开关、复选框、CSS 颜色                                                    | 枚举保存整数，布尔保存 boolean；十六进制颜色显示色块，CSS 字符串保持原样                              |
| Controls / 控件         | nullable checkbox 按 Space，再 Backspace；nullable number 输入后清空                    | 保存结果恢复 JSON `null`，不是空字符串或 false                                                        |
| Visibility / 条件显示   | Space 打开 show advanced；编辑高级说明后再关闭                                          | 字段/分区隐藏，但保存结果保留已编辑值；用 Ctrl+Tab 可切换可见分区                                     |
| Visibility / 条件显示   | Features 中 Enter、Down、Space、Enter 勾选 advanced                                     | contains 条件字段出现；取消勾选不会删除其文本                                                         |
| Nested / 嵌套草稿       | 对象列表按 Ctrl+E，Enter 进入字段，改 name 后按两次 Esc                                 | 首次提示确认，第二次丢弃当前层未提交草稿；重新打开仍为原值，另一个条目和未知属性不变                  |
| Nested / 嵌套草稿       | 在覆盖层内编辑后 Ctrl+S，再 Esc                                                         | 校验并将整条已打开草稿栈提交到根表单内存；覆盖层保持打开，Esc 逐层关闭，最后回根表单保存结果文件      |
| Nested / 嵌套草稿       | anyOf 按 Ctrl+E、Enter 编辑 alpha；Ctrl+Right、Enter 编辑 beta；Ctrl+Left、Enter 返回   | alpha/beta 是同一 JSON 的两个视图，切回时先前输入还在；Ctrl+S 提交后保存两者及 extra，不用 Esc 切视图 |
| Nested / 嵌套草稿       | ports 按 Ctrl+E、Enter 到 key，Tab 到 value，再 Ctrl+E、Enter 编辑 port                 | 根 definitions 在子覆盖层中仍可解析；Ctrl+S 提交整栈，Esc 逐层关闭，保留条目的 extra                  |
| Nested / 嵌套草稿       | 空列表先 Ctrl+E，再 Ctrl+N、Ctrl+E、Enter                                               | 第一次不隐式新增；显式新增后才编辑，未动的 optional_note 不出现在结果里                               |
| Preservation / 无损保存 | 不改任何值，根表单 Ctrl+S                                                               | 保留 null、空数组、空 map、未知属性，缺失的 optional_default 仍缺失                                   |

切根标签用 **Ctrl+L / Ctrl+J**，切分区用 **Ctrl+Tab / Ctrl+Shift+Tab**，切字段用
**Tab / Shift+Tab**。textarea 的 Up/Down 是行间移动，range 的 Up/Down
是选择端点，不是离开字段。

覆盖层有条目标签与表单两个焦点区域：Ctrl+E 打开后先按 Enter
进入字段。干净草稿按一次 Esc 关闭当前层；有修改但未提交时，第一次 Esc
提示，紧接着第二次 Esc 确认丢弃。Ctrl+S
会提交整条已打开覆盖层栈到根内存，已提交内容不能再靠 Esc
撤销；回到根表单后再保存/退出。Ctrl+N
只新增，不自动打开。对嵌套集合执行快捷键前，先确认焦点在目标字段，避免把外层条目当作目标。

## 图片能力与源码降级

- `preview.svg` 是首个字段内联 SVG
  的可读副本，可用浏览器或图片查看器打开对照；schema 的 `x-content.source` 才是
  TUI 实际读取的内容，两者内容一致。其他选项的 SVG 也直接内联在 schema 中。
- 预览使用 `resvg` 按终端实际字符格像素栅格化，再由 `ratatui-image` 输出 **Kitty
  / iTerm2 / Sixel
  原生像素图片**。旧半块字符渲染路径已移除，不再用两个色块/字符格近似文字。Mermaid
  使用高对比度深色预览，原始 SVG 的颜色保持不变。
- 图片默认 **Fit**，完整适配当前可用区域并水平、垂直居中；`i` / `=` 可越过 Fit
  继续放大，`o` / `-` 缩小，`f` 才重新切回
  Fit。大图只渲染可见区域，显示水平/垂直滚动条。缩放从 SVG
  重新栅格化，不放大旧缩略图；缩放与调整窗口大小保留观察位置，并在最终有效区域约束滚动。
- 自动模式只读环境提示和终端报告的窗口像素，不查询
  stdin、不吞按键。Kitty/Ghostty 选 Kitty，iTerm2/WezTerm/Rio 选
  iTerm2，精确匹配的 foot/mlterm 选
  Sixel。环境提示不是协议握手保证；未知或冲突时显示源码，不猜测终端品牌。
- 字符格像素必须由同一次窗口快照准确计算，零值或不能整除时显示源码；不会编造
  `10x20` 默认值，也不会在 macOS 上额外乘 Retina
  倍数。若终端没报告尺寸，见下方显式设置说明。
- `TERM=dumb`、没有图片 feature 或未确认原生协议时 SVG 显示源码；Mermaid 还需要
  `mermaid` feature。Markdown
  始终显示源码。实际解析/编码/栅格化失败显示错误与源码。
- 本例使用自包含 SVG，不加载 PNG/JPEG 文件或远程 URL；SVG 的外部 `<image href>`
  不会被加载。
- SVG 准备、系统字体、栅格化与编码在会话拥有的单 worker
  中执行，只保留一个最新待处理请求，不积压连续缩放/平移的旧图片。按键仍顺序处理，首图尺寸就绪前的视口操作会有界排队并按序重放；系统字体通过库的
  memory mapping 加载并在会话内复用。
- 加载中显示
  `Rendering`；源码、帮助与关闭不等待后台图片完成，并隔离过期结果。未变化时不重复
  draw。已经进入的单次渲染/编码调用不能强制中断，终端 payload
  写入也可能耗时；这不是帧率保证。查看器按键如下，均不会修改表单草稿：

| 按键                     | 图片预览                                                      | 源码                      |
| ------------------------ | ------------------------------------------------------------- | ------------------------- |
| 方向键 / `hjkl`          | 左右、上下平移                                                | Up/Down 或 `j`/`k` 滚动行 |
| `i` / `=`，`o` / `-`     | 放大、缩小，保持观察中心                                      | 不修改源码                |
| `f`                      | 适应当前窗口并水平、垂直居中                                  | 不修改源码                |
| PageUp / PageDown        | 垂直翻页                                                      | 翻页                      |
| Home / End               | 图片左上角 / 右下角                                           | 源码开头 / 结尾           |
| `n` / `p`，Tab / BackTab | 下一项 / 上一项富内容                                         | 同左                      |
| `s`                      | 切换到源码，保留图片视口                                      | 返回图片预览              |
| `?` / Ctrl+H             | 打开完整帮助，Up/Down 或 PageUp/PageDown 滚动；Esc 回到查看器 | 同左                      |
| Esc / Ctrl+P             | 返回原编辑器或选择器                                          | 同左                      |

状态行应显示 **`Preview · native kitty/iterm2/sixel pixels`**，而不是旧的
`terminal half-blocks`。阅读比例仍是 SVG 单位到终端列的映射，不能当作 SVG
到物理像素的百分比；实际位图尺寸是占用列/行乘实际字符格像素。`f`
看全图，宽图在窄窗口里仍可能需要 `i` 放大后滚动阅读。小窗口帮助不全时用 `?`；Esc
先回查看器，不会关闭底层选择器或提交选项。带超时的会话在底边显示倒计时。

### 显式选择协议或字符格像素

通常先用上面的自动模式。若终端确实支持但提示“未自动识别”，在 `--output`
**之前**加：

```text
--image-protocol kitty
```

Kitty 需 0.28+，Ghostty 同样选 `kitty`；iTerm2、WezTerm、Rio 选
`iterm2`；确认支持 Sixel 的终端才选
`sixel`。不要因为有真彩色就选择图片协议，普通 `xterm-256color`
也不能证明支持。显式选择是用户配置，不是握手验证。

若协议已知但终端没报告字符格像素，另加
`--image-cell-size WIDTHxHEIGHT`，填当前终端**一个字符格的实际像素**，不是窗口行列数；例如
`10x20`
仅在实际测得就是该尺寸时适用。错误尺寸会拉伸或裁错图片，字体缩放后也要更新手动值。库调用者可通过
`UiOptions::with_image_protocol` 和 `with_image_cell_size`
配置。`--image-protocol source` 可强制源码。

自动模式在 tmux/screen 内显示源码，不修改任何 passthrough 设置。显式
Kitty/iTerm2 的 tmux 路径只包装现有透传，要求用户已经配置好；screen 与
Sixel-through-tmux 保守显示源码。未做这些 multiplexer 组合的实机视觉验收。

### 资源边界

栅格化前保守估算主图、SVG
隔离图层、路径/虚线扩展、原生编码及终端缓冲复制。渲染与编码分阶段取峰值；兄弟层顺序执行取最大值，嵌套活跃层才累加。128
MiB 是工程工作集估算，不是进程 RSS 硬上限，也不是压缩传输大小。

必要时先缩小可见区域并居中：手动 zoom 保留比例及真实字符格像素密度，只裁小
viewport；Fit
重新完整适配该区域。不提高预算、不省略图层，也不降采样后冒称高清。主画布另限制每轴
8192px、最多 4MP；Kitty 每轴最多 297
个字符格（当前库坐标限制）。放大后的其余区域可双轴滚动查看；`f`
恢复整图。连最小区域也无法满足预算，或有过密虚线/过复杂路径、filter/pattern/嵌入图片等不支持特性时，明确显示错误和源码。

Kitty 使用现有 zlib 无损压缩，iTerm2 使用 PNG；Sixel 的调色/量化不等于 RGBA
无损。压缩后字节数与解压后的 RGBA、渲染/编码工作集不同，不能用预计的几百 KiB
传输大小代替内存检查。

**依赖与发布限制：** 当前固定官方 resvg
`9e24e7b3ffa576ca07ed9be6a5ed16b6ecc69a7c`，包含
[#1238](https://github.com/linebender/resvg/pull/1238) 的嵌套局部坐标裁剪修复和
[#1239](https://github.com/linebender/resvg/pull/1239) 的可见区域图层优化。原
registry 0.50
在深层透明组放大裁剪时可能输出空白；不能把这种图当成功预览。修复尚未发布为
registry 版本，当前本地构建会获取固定官方 Git
源码；在上游发布包含这两项修复的新版本、切回版本依赖并完成像素验证前，不能发布本工作树到
crates.io。

可单独验证不带图片依赖的源码模式（同样在仓库根目录）：

**fish：**

```fish
set -l source_dir (mktemp -d)
and mbx run -p schemaui-cli --no-default-features --features tui,json -- tui \
  --schema examples/tui-showcase/schema.json \
  --config examples/tui-showcase/config.json \
  --draft "$source_dir/session.draft.json" \
  --output "$source_dir/result.json"
```

**Bash / Zsh：**

```bash
source_dir="$(mktemp -d "${TMPDIR:-/tmp}/schemaui-tui-source.XXXXXX")"
mbx run -p schemaui-cli --no-default-features --features tui,json -- tui \
  --schema examples/tui-showcase/schema.json \
  --config examples/tui-showcase/config.json \
  --draft "$source_dir/session.draft.json" \
  --output "$source_dir/result.json"
```

这是同一个 schema，无需为了轻量构建维护另一套示例。

## 已验证

原半块版本虽通过 17 条 PTY
与视口边界检查，用户实际反馈仍然模糊，因此没有通过清晰度验收，现已被原生像素实现替代。不能用旧检查结果证明画面清晰。

后台优化前的原生实现独立二进制在 macOS arm64 下通过 **37 条完整版 PTY 旅程，以及
1 条轻量源码模式旅程**；另补 2 条大窗口位置捕获和示例文案旅程，共 40
条。以下为该历史版本证据，不等于最新后台/放大修正已经完成同样验收：

- SVG/Mermaid、双轴居中与滚动、缩放、窗口
  resize、源码往返、选项预览与取消、完整帮助；多行编辑、控件、条件隐藏值保留、嵌套取消、map/anyOf
  提交及独立结果文件。
- Kitty/iTerm2 的帮助与源码期间隐藏、A→B→A resize
  后重新上传；字符格像素改变后重新栅格化；超预算窗口缩回后恢复预览，不丢失 SVG
  树或视口。
- 未知/冲突提示、`TERM=dumb`、零值或不精确像素、显式源码、tmux/screen
  自动模式与不支持的透传路径均显示源码；显式协议和字符格覆盖、根/子命令参数优先级、普通虚线与
  Mermaid 虚线可预览，过密或闭合的高复杂度虚线明确降级。
- 保存 JSON、退出码、raw 模式、alternate screen 和 bracketed paste 恢复；配置的
  `$schema` 按输入声明语义移除。超时退出码为 4、stdout 为空，并在离开 alternate
  screen 前清理本会话 Kitty 图片。Darwin 终端配置比较只忽略内核输入状态位
  `PENDIN`，完整前后状态仍保留。

另外解码实际 Kitty RGBA 和 iTerm2
PNG，核对位图与字符格占用一致、缩放往返像素相同、平移改变真实图像区域、Kitty ID
不累积且清理不涉及其他程序。Sixel 检查了真实编码及 raster
尺寸，未将它冒称为像素解码或终端视觉验收。已目视检查从原生 payload 提取的
SVG/Mermaid 位图，文字和线条不再按字符格近似；它们仍不是终端窗口截图。

该历史版本的 JSON 与 SVG 可解析，独立 SVG 与 schema 内联主图一致；严格
Clippy、library 324 条与 CLI 43 条已有测试、Wasm
检查和轻量构建通过，没有新增仓库内的正式测试用例。

### 2026-11-17 后台与放大修正

- 最新完整版通过六条 SVG/Mermaid/18 层透明 SVG 的双协议缩放旅程；解码 **54
  次真实 Kitty/iTerm2 图片传输**，核对完整 payload、实际字符格像素、普通
  SVG/Mermaid 同一比例往返像素一致，以及 `i`/`=` 可越过 Fit 放大、`o`/`-`
  能缩回。18 层 SVG 的中心标记宽约 120 → 170 →
  240px，确认不是只改百分比或输出空白。
- 预算 crop 后 End → 放大/缩小、帮助隐藏期间 resize
  的双协议旅程通过。候选扩域不会提前改变观察中心；同 region 下切 Fit
  的旧结果不会把模式改回 manual。
- 再通过 30 条完整业务流程，覆盖加载取消/帮助/源码、早期按键与 resize、高
  DPI、复杂 SVG 边界、超时、多行及控件编辑、条件隐藏值、嵌套取消、map/anyOf
  提交与管道输入。加上缩放和边缘旅程，本代共 **38 条完整版 + 1 条轻量源码模式
  PTY**，保存 JSON、退出码和终端恢复均通过；不与历史 40 条混算。
- 调试版同一高 DPI Mermaid 输入路线中，首图 Kitty 约 501ms、iTerm2 约
  915ms；连续 20 次 pan 后帮助分别约 95ms，关闭约 19/27ms。与早期同步版约 8059ms
  的帮助基线相比，输入阻塞显著减轻；默认比例、系统负载和工作量不同，不能当成严格性能基准或终端帧率。18
  层透明图在调试版仍可能需要数秒完成新图片，source/help/close 不等待它完成。
- 最新格式、严格 Clippy、完整版和轻量构建、Wasm 检查通过；已有 library 324
  条、CLI 43 条测试通过。library 的 1 条
  `generated_artifact_modules_compile_and_construct_artifacts`
  用例因内部直接调用 Cargo 并删除独立 target 而过滤；另以独立 `mbx` consumer
  验证六类生成模块，不将过滤项称为通过。没有新增正式测试。

桌面截图工具不可用，**真实终端窗口的清晰度、字体缩放和残影目视验收尚未完成**；Windows、真实
multiplexer 与跨终端组合也未验收。协议输出通过不等于这些目视项目已通过。

## 文件

- `schema.json`：控件、可见性、节点/选项 SVG 和 Mermaid、嵌套对象/集合的
  schema。
- `config.json`：有效初始值，刻意包含
  null、空集合、隐藏值、未知属性，并省略一个带 default 的可选字段。
- `preview.svg`：内联主图的独立副本，方便直接查看与编辑；更新主图时同步 schema
  中的 source。
- `README.md`：启动命令与手动体验路线。
