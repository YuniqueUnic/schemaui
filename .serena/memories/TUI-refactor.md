[need to fix] 当前的 main page 界面设计基本上是这样，但是还需要确认，以及确保

1. root sections，并且需要正确计算宽度，当 sections 过宽时，应该滚动显示
2. child sections，并且需要正确计算宽度，当 sections 过宽时，应该滚动显示
3. field 中字段的宽度应该正确计算，当字段过宽时，应该 wrap 显示。

- 比如：content, description, error_msg 等。

4. cusso r hint 宽度应该正确计算，并且始终应该位于 content
   字符的右侧，就像正常的一个 input 那般的 cursor hint 一样，

- 需要考虑到 wrap 的情况，content height 出现了变化。

5. 确保 tab , shift + tab 键能够正常工作，能够实现聚焦到下/上一个
   field/child-sections/sections.并且循环的逻辑.

然后就是 overlay 了

1. overlay 的目前的界面设计是不符合下面的描述的，应该重构。使其符合下面对于
   overlay 的设计和描述
2. 然后就是 overlay 的功能设计也应该按照下面对于 overlay
   的设计和描述来进行更改和重构。
3. 还有就是 overlay 处理 composite 时，如果 composite 的 overlay 中，还存在
   composite 的 fields, 也应该按照 CTRL+N, CTRL+E 的方式“弹出 overlay”.
   但是这里的“弹出 overlay”.实际是直接覆盖当前的 overlay.
   那么我们暂时约定一下，mainpage 上的第一个 overlay 记住 overlay1, 然后
   overlay1 上覆盖的 overlay 记住为 overlay2. 以此类推。overlayN. 使得 TUI
   界面看起来只有两个层级，mainpage 和 overlay. 但是这里需要给 overlay 的
   overlay 进行标记了。并且代码逻辑也应该嵌套处理这种情况。用户在 overlay 的
   overlay 中时，即 overlay2, CTRL+S 键，然后 ECS/CTRL+Q 应该能够关闭 overlay2,
   从而 回到 overlay 1. 然后用户再 CTRL+S, 然后 ECS/CTRL+Q 键应该能够关闭
   overlay1, 回到 mainpage.,
4. 需要修改 overlay 中 CTRL+S 的逻辑，CTRL+S 只应该保存当前的修改，不应该退出
   overlay.
   - 用户要退出 overlay，应该使用 ECS/CTRL+Q 键。
   - 这样能够让整个 TUI 保持一套操作逻辑。

再进一步：

1. 也许 composite
   或者相关的复杂组件，应该重构，使其更加易于使用，和符合当前新的架构。

## schemaui 的整体界面 TUI 设计：

## | [general] | dataPlane | website | docs | xxxx | xxxxxxxxxx | xxx<<| <- root sections ([xx] 表示当前被选中的 section，并且需要正确计算宽度，当 sections 过宽时，应该滚动显示)

## | [child section1] | child section2 | child section3 | child sect<< | <-- child sections ( [xx] 显示当前被选中的 child section，宽度应该正确计算，当 sections 过宽时，应该滚动显示) --------------------------------------------------------------------- 当然，child section 也可能还有其 child sections，所以需要递归处理。 | ---------------- | | name *: -| | | [ content ] | | root sections 是 json schema 的 $.object 类型中解析得到。也就是 json schema 中根节点中 nested 的第一级 object， | type | description | 这就应该算一个 field | 只要是在 root 中不能够被直接展示的基本类型，那么就应该算是 child section。 | error_msg -| | child section 的 child section，应该和 root section 一样处理。 | ---------------- | | name *: ·dirty | field 应该渲染至对应的 section(child section) 中 | [ content ] | | type | description | 用户可以使用 tab 键，将光标移动到下一个 field 中。也可以使用 shift + tab 键，将光标移动到上一个 field 中。 | error_msg | 当 child section 中的 fields 遍历完毕之后，那么 tab 键应该跳转到下一个 child section/root section 中。 | ---------------- | 当 child sections 遍历完毕之后，那么 tab 键应该跳转到下一个 root section 中。 | | shift + tab 键功能和 tab 键功能正好相反。 | | | | field -> fields -> child section -> child sections -> root section -> child section -> field | | | tab 循环遍历流程 | | | ----------------------------------------------------------------------------------------

## 快捷键说明栏： <- action bar(用于动态展示当前界面/控件的快捷键)

## 状态栏： <- status bar(用于动态展示当前系统/界面/控件的状态)

---

pop up overlay 设计：

## -------------------------------------------------------------------------------------- <-这是 main page | 顶栏 (用于增加/删除，调整 entries 的顺序) | | v v-这是 popup overlay | overlay 也应是包含一个顶栏 entries | |===========|==================================================| | 这个 entries 则是对应的是多个元素的意思 | | [entry 1] | entry 2 | entry 3 | entry 4 | entry 5 | entryx<< | | 通过快捷键 Ctrl + N 新增添加一个 entry，Ctrl + D 删除一个 entry | |-----------|--------------------------------------------------| | 然后使用 Ctrl + <-/-> 来调整当前选中的 entry 的顺序 | | [child section 1] | child sec 2 | child sec 3 | child sec<< | | 然后 依旧使用 tab，shift+tab 来实现 entry -> child section -> fields -> child sections -> fields -> entry 的逻辑切换 | |-----------|--------------------------------------------------| | 只是将 root sections 变成了 entries。 | | name *: | | 通过这样的渲染逻辑，就能实现 array 中也包含嵌套的 object 的逻辑 | | [ content ] | | | | type | description field 渲染逻辑和 | | | | error_msg mainpage 一致 | | | | ---------------- | | | | ... | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | |==============================================================| | | |

这样的设计就可以应对 嵌套的 json schema，通过将嵌套的 object 等嵌套类型映射为
child section，然后递归处理。最终的一般类型则会被渲染为 fields。

而类似数组这样的包含多个内容的类型，则是采用 overlay 模式进行展示和渲染。

- **快捷键单一来源** –
  `keymap/default.keymap.json`列出了每个快捷键（上下文、组合键、动作）。`app::keymap::keymap_source!()`宏将此文件拉入二进制文件中，`InputRouter`使用它对`KeyEvent`进行分类，运行时页脚从相同的数据中呈现帮助文本——保持文档和行为
  DRY。
- **根标签与部分** –
  焦点通过`Ctrl+J / Ctrl+L`（根）和`Ctrl+Tab / Ctrl+Shift+Tab`（部分）循环。普通`Tab`/`Shift+Tab`在各个字段之间移动。
- **字段** –
  渲染标签、描述和内联错误消息。枚举/复合字段显示当前选择；数组总结长度和选定条目。
- **弹出窗口与覆盖层** – 按下`Enter`键打开枚举/oneOf
  选择器的弹出窗口；`Ctrl+E`打开复合编辑器的全屏覆盖层。覆盖层暴露集合快捷键（`Ctrl+N`、`Ctrl+D`、`Ctrl+←/→`、`Ctrl+↑/↓`）以及`Ctrl+S`提交。
- **状态与帮助** –
  页脚突出显示脏状态、未解决的验证错误和上下文感知帮助文本。当自动验证启用时，每次编辑都会立即更新这些计数器。

| 上下文 | 快捷键                        | 动作                                                     |
| ------ | ----------------------------- | -------------------------------------------------------- |
| 导航   | `Tab` / `Shift+Tab`           | 在 (root sections/entries)/child sections/fields之间移动 |
|        | `Ctrl+Tab` / `Ctrl+Shift+Tab` | 切换 (root sections/entries)/child sections              |
|        | `Ctrl+J` / `Ctrl+L`           | 切换 root section/entries                                |
| 选择   | `Enter`                       | 打开弹出窗口/应用选择                                    |
| 编辑   | `Ctrl+E`                      | 启动复合编辑器 (overlay)                                 |
| 状态   | `Esc`                         | 清除状态或关闭弹出窗口                                   |
| 持久化 | `Ctrl+S`                      | 保存 + 验证 (验证通过才会被保存，但是这个行为应该可配置) |
| 退出   | `Ctrl+Q` / `Ctrl+C`           | 退出（如果脏则需要确认）                                 |
| 集合   | `Ctrl+N` / `Ctrl+D`           | 添加/删除条目                                            |
|        | `Ctrl+←/→`,                   | 重新排序条目                                             |

[need to fix]

1. 当前的 TUI 设计也存在问题：不够组件化。
   /Users/unic/dev/projs/rs/schemaui/src/presentation 这里是整体的界面 layout
   模块化设计。看起来还行， /Users/unic/dev/projs/rs/schemaui/src/form
   而这里的更细粒化的 form components，拆分还不够。我希望参考前端
   shadcn(https://ui.shadcn.com/docs/components) 的这样的组件化设计。
   将功能化的组件进行拆分和封装。比如我们这个 schemaui 项目。最重要的就是 form
   模块，form 则是 field 中的【content】的核心渲染模块。比如：JSON Schema → TUI
   映射

   `schema::layout::build_form_schema`遍历完全解析的模式，并将每个子树映射为`FormSection`/`FieldSchema`：

   | 模式功能                                                     | 结果控件                                                | 行为 |
   | ------------------------------------------------------------ | ------------------------------------------------------- | ---- |
   | `type: string`, `integer`, `number`                          | 带有数值保护的内联文本编辑器                            |      |
   | `type: boolean`                                              | 切换/复选框                                             |      |
   | `enum`                                                       | 弹出选择器（单选或多选用于数组枚举）                    |      |
   | 数组                                                         | 内联列表摘要 + 每个项目的覆盖层编辑器                   |      |
   | `patternProperties`, `propertyNames`, `additionalProperties` | 带有模式支持验证的键值编辑器                            |      |
   | `$ref`, `definitions`                                        | 在布局前解析；被视为内联模式                            |      |
   | `oneOf` / `anyOf`                                            | 变体选择器 + 覆盖层表单，将非活动变体排除在最终负载之外 |      |

   输入框就应该可以让用户能够输入，并且 cursor hint
   也应该正确的计算中文，英文的宽度，以及“ ”的宽度 (unicode-width)。对于
   integer/number 类型的控件，应该支持 <-/-> 键直接改变数值。 <- 为 -1， -> 为
   +1。(这个快捷键应该是可以配置的。步长也应该是可以配置的) 对于 boolean，enum
   类型，应该支持 <-/-> 键直接改变选项，依次遍历选项。(快捷键应该可以配置),
   当然像这样的多选一的类型，也可以使用 Enter 打开一个 overlay，然后使用 ↑/↓
   进行选择，然后 enter 进行提交。 - 如果是多选的话，也是使用
   overlay，弹出一个列表，然后用户使用 space 进行选择需要的多个选项，然后使用
   enter 进行确定提交。对于数组，则是采用内联列表摘要 +
   覆盖层编辑器，从而用户能在覆盖层中编辑数组元素。（增加，删除，调整顺序，选择编辑项目）
   然后就是每个这样的 component 也都应该具有 validator，用于验证用户输入。并且
   validate 函数应该返回错误信息，用于显示给用户。

---
