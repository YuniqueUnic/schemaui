/* eslint-disable react-refresh/only-export-components */

import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";

/**
 * UI locale. `"en"` is the source language: every key *is* its English text,
 * so the English rendering needs no dictionary and a missing translation
 * degrades to English rather than to a key name.
 */
export type Locale = "en" | "zh";

/**
 * Chinese renderings, keyed by the English source string. `{name}` slots in
 * a template are filled by `t`'s second argument. Exported so tests can
 * audit the dictionary itself.
 */
export const zhDictionary: Record<string, string> = {
  // Header / session chrome
  "Back to schema": "返回 Schema",
  "Back": "返回",
  "Configuration session": "配置会话",
  "SchemaUI on GitHub": "GitHub 上的 SchemaUI",
  "Save (Ctrl+S)": "保存（Ctrl+S）",
  "Save": "保存",
  "Switch to light mode": "切换到浅色模式",
  "Switch to dark mode": "切换到深色模式",
  "Light": "浅色",
  "Dark": "深色",
  "Language": "语言",
  "Exit": "退出",
  "Export": "导出",
  "Exiting…": "正在退出…",
  "Exporting…": "正在导出…",
  "Loading session…": "正在加载会话…",
  "Expand navigation": "展开导航",
  "Collapse navigation": "收起导航",
  "Session Ended": "会话已结束",
  "You can close this browser tab.": "可以关闭此浏览器标签页了。",
  "Session Timed Out": "会话已超时",
  "This session closed itself when its deadline passed, so nothing was saved. Ask whoever started it to run it again.":
    "会话已到达截止时间并自动关闭，未保存任何内容。请联系启动者重新运行。",
  "Session Unavailable": "会话不可用",
  "This session is no longer being served. It may have timed out or already been closed — ask whoever started it to run it again.":
    "该会话已不再可用，可能已超时或被关闭——请联系启动者重新运行。",
  "General": "常规",
  "(root)": "（根）",

  // Navigation / panels
  "Schema": "Schema",
  "Layout": "布局",
  "Nav": "导航",
  "Editor": "编辑器",
  "Preview": "预览",
  "No layout available": "暂无布局",

  // Status bar
  "Exiting": "正在退出",
  "Ending session…": "正在结束会话…",
  "Saving": "正在保存",
  "Persisting changes…": "正在保存更改…",
  "Validating": "正在校验",
  "Checking the current document…": "正在检查当前文档…",
  "Errors {count}": "{count} 个错误",
  "Fix validation errors before saving.": "保存前需先修复校验错误。",
  "Unsaved": "未保存",
  "Changes are staged locally.": "更改暂存在本地。",
  "Ready": "就绪",
  "Everything is synced.": "所有内容已同步。",
  "Save changes": "保存更改",
  "Hide shortcuts": "隐藏快捷键",
  "Show shortcuts": "显示快捷键",

  // Preview pane
  "Copied": "已复制",
  "Copy": "复制",
  "Collapse preview": "收起预览",
  "Expand preview": "展开预览",
  "Pretty": "美化",
  "Preview failed": "预览失败",

  // Field controls
  "Required": "必填",
  "Select an option": "请选择一个选项",
  "On": "开",
  "Off": "关",
  "Enabled": "已启用",
  "Disabled": "已禁用",
  "+ Add entry": "+ 添加条目",
  "Click below to add your first item": "点击下方添加第一个条目",
  "No items yet": "暂无条目",
  "Edit": "编辑",
  "Remove": "删除",
  "Entry {count}": "条目 {count}",
  "{title} entry {count}": "{title} 条目 {count}",
  "{name} · Item {count}": "{name} · 条目 {count}",
  "Hex colour": "十六进制颜色",
  "Colour {value}": "颜色 {value}",
  "unset": "未设置",

  // Variants
  "No variants configured.": "未配置任何变体。",
  "Selected Variant": "所选变体",
  "{name} content:": "{name} 的内容：",
  "Variant {count}": "变体 {count}",
  "Entry": "条目",
  "{name} · {variant} {count}": "{name} · {variant} {count}",
  "Select variant type for {name}": "为 {name} 选择变体类型",
  "Choose which type of item to add:": "选择要添加的条目类型：",
  "No entries yet": "暂无条目",
  "Click below to add your first entry": "点击下方添加第一个条目",
  "Click the button below to add your first entry": "点击下方按钮添加第一个条目",
  "+ Add variant entry": "+ 添加变体条目",
  "Edit Selected Variant": "编辑所选变体",
  "Edit Active Variant": "编辑当前变体",
  "Select a variant type. The corresponding editor will appear below.":
    "选择一个变体类型，对应的编辑器将显示在下方。",
  "This field can match multiple schemas": "该字段可匹配多个 Schema",
  "One of variants": "one_of 变体",
  "Any of variants": "any_of 变体",

  // Overlay / dialogs
  "Details": "详情",
  "Layer {count}": "第 {count} 层",
  "Nested editor overlay": "嵌套编辑器浮层",
  "Close": "关闭",
  "Validation Errors ({count})": "校验错误（{count}）",
  "The following fields have validation errors. Click on an error to navigate to the field.":
    "以下字段存在校验错误。点击一条错误即可跳转到对应字段。",
  "Go to field": "前往字段",
  "Exit blocked": "退出被阻止",
  "Resolve the remaining schema errors": "解决剩余的 Schema 错误",
  "Fix the issues below or force exit to emit only the last saved configuration.":
    "修复下方的问题，或强制退出以仅输出最后保存的配置。",
  "+{count} more issue(s) hidden. Continue editing to see full details.":
    "还有 {count} 个问题未显示。继续编辑可查看完整详情。",
  "Back to form": "返回表单",
  "Force exit with last saved data": "强制退出并使用最后保存的数据",

  // Countdown
  "{time} until this session closes": "{time} 后本会话关闭",
  "This session closes when the countdown reaches zero. Anything unsaved is discarded.":
    "倒计时归零时本会话将关闭，未保存的内容会被丢弃。",

  // Mermaid editor
  "Mermaid source": "Mermaid 源码",
  "Mermaid preview": "Mermaid 预览",
  "Side by side": "左右",
  "Stacked": "上下",
  "Swap sides": "对换",
  "Live": "实时",
  "Render preview": "渲染预览",
  "Editor left, preview right": "编辑器在左，预览在右",
  "Editor above, preview below": "编辑器在上，预览在下",
  "Preview left, editor right": "预览在左，编辑器在右",
  "Preview follows typing": "预览跟随输入",
  "Rendering…": "正在渲染…",
  "Type Mermaid source to see it rendered": "输入 Mermaid 源码即可看到渲染结果",
  "This diagram does not parse:": "该图无法解析：",

  // Slider
  "Decrease {name}": "减小 {name}",
  "Increase {name}": "增大 {name}",
  "Edit {name}": "编辑 {name}",
  "Minimum": "最小值",
  "Maximum": "最大值",
  "Cannot read “{value}” as a number": "无法将“{value}”识别为数字",
  "Clamped": "已收敛到",
  "Rounded": "已取整到",
  "{action} to {value}": "{action} {value}",
  "Kept {value}": "保留 {value}",

  // Rich figures
  "This figure could not be rendered: {message}": "无法渲染该图形：{message}",
  "Mermaid / SVG source": "Mermaid / SVG 源码",
  "Copy source": "复制源码",
  "Download SVG": "下载 SVG",
  "Figure": "图形",

  // Playground landing screen
  "SchemaUI Playground": "SchemaUI 演练场",
  "Paste or drop a JSON Schema below (JSON, YAML or TOML). Everything after this runs in your browser via WebAssembly — no data leaves this tab.":
    "在下方粘贴或拖入一个 JSON Schema（支持 JSON、YAML 或 TOML）。此后的一切都会在你的浏览器里通过 WebAssembly 运行 —— 没有任何数据离开这个标签页。",
  "Everything stays on your computer. Your schema and data are parsed and rendered entirely in this browser tab via WebAssembly — nothing is uploaded, and there is no server on the other end to send it to. See the":
    "一切都保留在你的电脑上。你的 Schema 和数据完全在这个浏览器标签页里通过 WebAssembly 解析与渲染 —— 不会上传任何内容，另一端也没有服务器可以接收它。可以查看",
  "source on GitHub": "GitHub 上的源代码",
  "JSON Schema": "JSON Schema",
  "Initial data (optional)": "初始数据（可选）",
  "Upload file": "上传文件",
  "Upload a .json, .yaml or .toml file": "上传 .json、.yaml 或 .toml 文件",
  "Drop to load": "松开以加载",
  "Load example": "加载示例",
  "Build form": "生成表单",
  "Building…": "生成中…",
  "Schema is not valid JSON, YAML or TOML": "Schema 不是合法的 JSON、YAML 或 TOML",
  "Initial data is not valid JSON, YAML or TOML": "初始数据不是合法的 JSON、YAML 或 TOML",
  "Playground is part of": "Playground 是",
  ", an open-source library that turns JSON Schema documents into interactive TUI and web forms.":
    "的一部分 —— 一个把 JSON Schema 文档转换为交互式终端与网页表单的开源库。",
  "Frequently asked questions": "常见问题",
  "Does any of this get sent to a server?": "这些内容会发送到服务器吗？",
  "No. There is no server: the schema pipeline (building the form, validating, rendering the document) runs in this tab via WebAssembly. You can disconnect from the network after the page loads and it keeps working.":
    "不会。这里没有服务器：整个流程（生成表单、校验、渲染文档）都通过 WebAssembly 在这个标签页里运行。页面加载完成后，即使断网也能继续使用。",
  "What formats can I paste or drop?": "可以粘贴或拖入哪些格式？",
  "JSON, YAML or TOML, for both the schema and the initial data — whichever you have on hand. The format is detected automatically, the same way schemaui-cli does for a file passed without --format.":
    "Schema 和初始数据都支持 JSON、YAML 或 TOML —— 手头有哪种就用哪种，格式会自动识别，和 schemaui-cli 在不传 --format 时的行为一致。",
  "What happens to my data if I close the tab?": "关闭标签页后，我的数据会怎样？",
  "It's gone — nothing is saved automatically. Use \"Export\" once you're done filling out the form to download the result as a file first.":
    "会丢失 —— 不会自动保存任何内容。填完表单后，请先点击「导出」把结果下载为文件。",
  "Where do I report a bug or ask something else?": "在哪里反馈问题或提出其它疑问？",
  "Open an issue on the SchemaUI repository": "在 SchemaUI 仓库提交 issue",
  "Enjoying local-first, no-server tools? Try Syzygy": "喜欢本地优先、无需服务器的工具？试试 Syzygy",
  "A local-first, peer-to-peer clipboard workbench for keeping devices in sync without a cloud in between.":
    "一个本地优先、点对点的剪贴板工作台，无需云端即可在设备间保持同步。",
  "Syzygy on GitHub": "GitHub 上的 Syzygy",
};

export type TParams = Record<string, string | number>;

function interpolate(template: string, params?: TParams): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match,
  );
}

interface I18nContextValue {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  /** Look up a string; `key` is its English text. */
  t: (key: string, params?: TParams) => string;
}

const STORAGE_KEY = "schemaui-locale";

function detectLocale(): Locale {
  if (typeof window === "undefined") return "en";
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "zh" || stored === "en") return stored;
  return navigator.language?.toLowerCase().startsWith("zh") ? "zh" : "en";
}

/**
 * English-by-construction default: a component rendered outside the provider
 * (a unit test, a future surface) speaks English rather than crashing.
 */
const I18nContext = createContext<I18nContextValue>({
  locale: "en",
  setLocale: () => {},
  t: interpolate,
});

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocale] = useState<Locale>(detectLocale);

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, locale);
    } catch {
      // A non-durable language choice is a preference, not a promise.
    }
  }, [locale]);

  const t = useCallback(
    (key: string, params?: TParams) =>
      interpolate(locale === "zh" ? zhDictionary[key] ?? key : key, params),
    [locale],
  );

  const value = useMemo(() => ({ locale, setLocale, t }), [locale, t]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nContextValue {
  return useContext(I18nContext);
}
