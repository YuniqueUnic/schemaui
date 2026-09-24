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
