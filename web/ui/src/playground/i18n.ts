/**
 * The Playground landing screen's own, small translation table.
 *
 * Not a general-purpose i18n system for the whole app: `App` and its form
 * renderer are driven by whatever `title`/`description`/labels the *schema*
 * carries, which is the author's content, not this codebase's — translating
 * that is a different problem this page does not try to solve. This covers
 * exactly the strings this screen itself wrote.
 */
export type Language = "en" | "zh";

export const LANGUAGE_STORAGE_KEY = "schemaui-playground-lang";

export interface PlaygroundStrings {
  title: string;
  description: string;
  privacyNote: string;
  privacyLinkText: string;
  schemaLabel: string;
  dataLabel: string;
  uploadFile: string;
  uploadHint: string;
  dropHint: string;
  loadExample: string;
  buildForm: string;
  building: string;
  schemaParseError: string;
  dataParseError: string;
  aboutPrefix: string;
  aboutSuffix: string;
  faqTitle: string;
  faqServerQuestion: string;
  faqServerAnswer: string;
  faqFormatsQuestion: string;
  faqFormatsAnswer: string;
  faqCloseQuestion: string;
  faqCloseAnswer: string;
  faqSupportQuestion: string;
  faqSupportLinkText: string;
  syzygyHeadline: string;
  syzygyDescription: string;
}

const en: PlaygroundStrings = {
  title: "SchemaUI Playground",
  description:
    "Paste or drop a JSON Schema below (JSON, YAML or TOML). Everything after this runs in your browser via WebAssembly — no data leaves this tab.",
  privacyNote:
    "Everything stays on your computer. Your schema and data are parsed and rendered entirely in this browser tab via WebAssembly — nothing is uploaded, and there is no server on the other end to send it to. See the",
  privacyLinkText: "source on GitHub",
  schemaLabel: "JSON Schema",
  dataLabel: "Initial data (optional)",
  uploadFile: "Upload file",
  uploadHint: "Upload a .json, .yaml or .toml file",
  dropHint: "Drop to load",
  loadExample: "Load example",
  buildForm: "Build form",
  building: "Building…",
  schemaParseError: "Schema is not valid JSON, YAML or TOML",
  dataParseError: "Initial data is not valid JSON, YAML or TOML",
  aboutPrefix: "Playground is part of",
  aboutSuffix:
    ", an open-source library that turns JSON Schema documents into interactive TUI and web forms.",
  faqTitle: "Frequently asked questions",
  faqServerQuestion: "Does any of this get sent to a server?",
  faqServerAnswer:
    "No. There is no server: the schema pipeline (building the form, validating, rendering the document) runs in this tab via WebAssembly. You can disconnect from the network after the page loads and it keeps working.",
  faqFormatsQuestion: "What formats can I paste or drop?",
  faqFormatsAnswer:
    "JSON, YAML or TOML, for both the schema and the initial data — whichever you have on hand. The format is detected automatically, the same way schemaui-cli does for a file passed without --format.",
  faqCloseQuestion: "What happens to my data if I close the tab?",
  faqCloseAnswer:
    "It's gone — nothing is saved automatically. Use \"Export\" once you're done filling out the form to download the result as a file first.",
  faqSupportQuestion: "Where do I report a bug or ask something else?",
  faqSupportLinkText: "Open an issue on the SchemaUI repository",
  syzygyHeadline: "Enjoying local-first, no-server tools? Try Syzygy",
  syzygyDescription:
    "A local-first, peer-to-peer clipboard workbench for keeping devices in sync without a cloud in between.",
};

const zh: PlaygroundStrings = {
  title: "SchemaUI 演练场",
  description:
    "在下方粘贴或拖入一个 JSON Schema（支持 JSON、YAML 或 TOML）。此后的一切都会在你的浏览器里通过 WebAssembly 运行 —— 没有任何数据离开这个标签页。",
  privacyNote:
    "一切都保留在你的电脑上。你的 Schema 和数据完全在这个浏览器标签页里通过 WebAssembly 解析与渲染 —— 不会上传任何内容，另一端也没有服务器可以接收它。可以查看",
  privacyLinkText: "GitHub 上的源代码",
  schemaLabel: "JSON Schema",
  dataLabel: "初始数据（可选）",
  uploadFile: "上传文件",
  uploadHint: "上传 .json、.yaml 或 .toml 文件",
  dropHint: "松开以加载",
  loadExample: "加载示例",
  buildForm: "生成表单",
  building: "生成中…",
  schemaParseError: "Schema 不是合法的 JSON、YAML 或 TOML",
  dataParseError: "初始数据不是合法的 JSON、YAML 或 TOML",
  aboutPrefix: "Playground 是",
  aboutSuffix: "的一部分 —— 一个把 JSON Schema 文档转换为交互式终端与网页表单的开源库。",
  faqTitle: "常见问题",
  faqServerQuestion: "这些内容会发送到服务器吗？",
  faqServerAnswer:
    "不会。这里没有服务器：整个流程（生成表单、校验、渲染文档）都通过 WebAssembly 在这个标签页里运行。页面加载完成后，即使断网也能继续使用。",
  faqFormatsQuestion: "可以粘贴或拖入哪些格式？",
  faqFormatsAnswer:
    "Schema 和初始数据都支持 JSON、YAML 或 TOML —— 手头有哪种就用哪种，格式会自动识别，和 schemaui-cli 在不传 --format 时的行为一致。",
  faqCloseQuestion: "关闭标签页后，我的数据会怎样？",
  faqCloseAnswer:
    "会丢失 —— 不会自动保存任何内容。填完表单后，请先点击「导出」把结果下载为文件。",
  faqSupportQuestion: "在哪里反馈问题或提出其它疑问？",
  faqSupportLinkText: "在 SchemaUI 仓库提交 issue",
  syzygyHeadline: "喜欢本地优先、无需服务器的工具？试试 Syzygy",
  syzygyDescription: "一个本地优先、点对点的剪贴板工作台，无需云端即可在设备间保持同步。",
};

export const STRINGS: Record<Language, PlaygroundStrings> = { en, zh };
