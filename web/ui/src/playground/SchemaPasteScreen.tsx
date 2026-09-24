import { useId, useState } from "react";
import type { DragEvent, ReactNode } from "react";
import {
  ChevronDown,
  ExternalLink,
  FileUp,
  Languages,
  Moon,
  ShieldCheck,
  Sun,
  UploadCloud,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import { useTheme } from "../theme";
import { useI18n } from "../i18n";
import { parseDocumentText } from "../transport/wasmTransport";
import type { JsonValue } from "../types";
// The repo's own control gallery (`examples/controls-gallery.schema.json`),
// imported verbatim rather than duplicated: one control per property, every
// default filled in, kept in sync with the schema by construction instead of
// by remembering to update a second copy. A two-field toy schema would build
// a form in one glance and prove nothing about what this build can render.
import controlsGallerySchema from "../../../../examples/controls-gallery.schema.json?raw";

interface SchemaPasteScreenProps {
  /** `rawSchemaText`/`rawDataText` are the exact textarea contents, alongside
   * the parsed values — `PlaygroundRoot` keeps them so "Back" from the built
   * form can hand them straight back to `initialSchemaText`/`initialDataText`
   * instead of losing what was typed. */
  onReady(
    schema: JsonValue,
    defaults: JsonValue,
    rawSchemaText: string,
    rawDataText: string,
  ): void;
  /** What was last typed here, handed back by `PlaygroundRoot` when a visitor
   * returns from the built form — so "back" does not mean "retype it". */
  initialSchemaText?: string;
  initialDataText?: string;
}

const EXAMPLE_SCHEMA = controlsGallerySchema.trim();

const SCHEMAUI_URL = "https://github.com/yuniqueunic/schemaui";
const SYZYGY_SITE_URL = "https://www.syzygysync.com/";
const SYZYGY_REPO_URL = "https://github.com/astrolix-ai/syzygy";
const SYZYGY_ICON_URL =
  "https://github.com/astrolix-ai/.github/raw/main/brand/syzygy-app-icon.svg";

/**
 * The Playground's entry screen: paste — or drop — a schema, get a form.
 *
 * There is no server here to bootstrap a session from, so the schema has to
 * come from the visitor before `App` has anything to render. Once `onReady`
 * fires this screen unmounts, but `PlaygroundRoot` keeps what was typed here
 * and hands it back via `initialSchemaText`/`initialDataText` if the visitor
 * returns from the built form (see `PlaygroundRoot`'s "Back" wiring) — so
 * coming back means picking up where you left off, not retyping it.
 *
 * Speaks the app-wide i18n dictionary like every other surface: this screen
 * used to keep its own table and storage key, which is how the built form's
 * language switch could disagree with the screen that led to it.
 */
export function SchemaPasteScreen(
  { onReady, initialSchemaText = "", initialDataText = "" }: SchemaPasteScreenProps,
) {
  const { t, locale, setLocale } = useI18n();
  const { theme, toggle: toggleTheme } = useTheme();

  const [schemaText, setSchemaText] = useState(initialSchemaText);
  const [dataText, setDataText] = useState(initialDataText);
  const [error, setError] = useState<string | null>(null);
  const [building, setBuilding] = useState(false);

  const handleBuild = async () => {
    setBuilding(true);
    try {
      let schema: JsonValue;
      try {
        schema = await parseDocumentText(schemaText);
      } catch (err) {
        setError(`${t("Schema is not valid JSON, YAML or TOML")}: ${(err as Error).message}`);
        return;
      }

      let defaults: JsonValue = {};
      if (dataText.trim()) {
        try {
          defaults = await parseDocumentText(dataText);
        } catch (err) {
          setError(`${t("Initial data is not valid JSON, YAML or TOML")}: ${(err as Error).message}`);
          return;
        }
      }

      setError(null);
      onReady(schema, defaults, schemaText, dataText);
    } finally {
      setBuilding(false);
    }
  };

  return (
    <div className="flex min-h-screen flex-col items-center gap-6 bg-background px-4 py-10 text-foreground">
      <div className="flex w-full max-w-5xl justify-end gap-1.5">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={toggleTheme}
          title={theme === "dark" ? t("Switch to light mode") : t("Switch to dark mode")}
        >
          {theme === "dark"
            ? <Sun className="h-3.5 w-3.5" aria-hidden="true" />
            : <Moon className="h-3.5 w-3.5" aria-hidden="true" />}
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => setLocale(locale === "en" ? "zh" : "en")}
          title="English / 中文"
        >
          <Languages className="h-3.5 w-3.5" aria-hidden="true" />
          <span className="ml-1">{locale === "en" ? "中文" : "English"}</span>
        </Button>
      </div>

      <SyzygyPromo />

      <Card className="w-full max-w-5xl">
        <CardHeader>
          <CardTitle className="text-lg">{t("SchemaUI Playground")}</CardTitle>
          <CardDescription>
            {t(
              "Paste or drop a JSON Schema below (JSON, YAML or TOML). Everything after this runs in your browser via WebAssembly — no data leaves this tab.",
            )}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-start gap-2 rounded-md border border-theme bg-muted/40 px-3 py-2.5">
            <ShieldCheck
              className="mt-0.5 h-4 w-4 shrink-0 text-emerald-500"
              aria-hidden="true"
            />
            <p className="text-xs leading-relaxed text-muted-foreground">
              {t(
                "Everything stays on your computer. Your schema and data are parsed and rendered entirely in this browser tab via WebAssembly — nothing is uploaded, and there is no server on the other end to send it to. See the",
              )}{" "}
              <a
                href={SCHEMAUI_URL}
                target="_blank"
                rel="noreferrer"
                className="font-medium text-foreground underline-offset-4 hover:underline"
              >
                {t("source on GitHub")}
              </a>
              .
            </p>
          </div>
          {/* Side by side rather than stacked: the two documents are meant to
              be compared against each other (which pointer in the schema does
              this default belong to?), and a wide screen has the room to make
              that a glance instead of a scroll. `md:` rather than `lg:` — a
              tablet-width window already has enough room for two columns of
              monospace text, and stacking is one column's worth of it. */}
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            <DocumentField
              id="playground-schema"
              label={t("JSON Schema")}
              value={schemaText}
              onChange={setSchemaText}
              placeholder={EXAMPLE_SCHEMA}
              minHeightClassName="min-h-[420px]"
            />
            <DocumentField
              id="playground-data"
              label={t("Initial data (optional)")}
              value={dataText}
              onChange={setDataText}
              placeholder="{}"
              minHeightClassName="min-h-[420px]"
            />
          </div>
          {error && (
            <p role="alert" className="text-xs text-destructive">
              {error}
            </p>
          )}
          <div className="flex items-center justify-between gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => {
                setSchemaText(EXAMPLE_SCHEMA);
                setError(null);
              }}
            >
              {t("Load example")}
            </Button>
            <Button
              type="button"
              size="sm"
              disabled={!schemaText.trim() || building}
              onClick={() => void handleBuild()}
            >
              {building ? t("Building…") : t("Build form")}
            </Button>
          </div>
        </CardContent>
      </Card>

      <p className="max-w-5xl text-center text-xs text-muted-foreground">
        {t("Playground is part of")}{" "}
        <a
          href={SCHEMAUI_URL}
          target="_blank"
          rel="noreferrer"
          className="inline-flex items-center gap-1 font-medium text-foreground underline-offset-4 hover:underline"
        >
          <ExternalLink className="h-3.5 w-3.5" aria-hidden="true" />
          SchemaUI
        </a>
        {t(", an open-source library that turns JSON Schema documents into interactive TUI and web forms.")}
      </p>

      <Faq />
    </div>
  );
}

interface DocumentFieldProps {
  id: string;
  label: string;
  value: string;
  onChange(value: string): void;
  placeholder: string;
  minHeightClassName: string;
}

/**
 * A textarea that also accepts a dropped or browsed-for file.
 *
 * Reads the file as text and hands it straight to the textarea's own value —
 * parsing (and the error it might produce) happens once, at "Build form",
 * rather than twice for the two different ways content can arrive here.
 */
function DocumentField({
  id,
  label,
  value,
  onChange,
  placeholder,
  minHeightClassName,
}: DocumentFieldProps) {
  const { t } = useI18n();
  const [dragging, setDragging] = useState(false);
  const fileInputId = useId();

  const loadFile = async (file: File) => {
    onChange(await file.text());
  };

  const handleDrop = (event: DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    setDragging(false);
    const file = event.dataTransfer.files[0];
    if (file) void loadFile(file);
  };

  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between">
        <Label htmlFor={id}>{label}</Label>
        <label
          htmlFor={fileInputId}
          className="flex cursor-pointer items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
          title={t("Upload a .json, .yaml or .toml file")}
        >
          <FileUp className="h-3.5 w-3.5" aria-hidden="true" />
          {t("Upload file")}
        </label>
        <input
          id={fileInputId}
          type="file"
          accept=".json,.yaml,.yml,.toml,application/json,text/yaml,text/x-yaml"
          className="sr-only"
          onChange={(event) => {
            const file = event.target.files?.[0];
            if (file) void loadFile(file);
            event.target.value = "";
          }}
        />
      </div>
      <div
        onDragOver={(event) => {
          event.preventDefault();
          setDragging(true);
        }}
        onDragLeave={() => setDragging(false)}
        onDrop={handleDrop}
        className={cn(
          "relative rounded-md transition-colors",
          dragging && "ring-2 ring-primary ring-offset-2 ring-offset-background",
        )}
      >
        <Textarea
          id={id}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          placeholder={placeholder}
          className={cn("font-mono text-xs", minHeightClassName)}
          spellCheck={false}
        />
        {dragging && (
          <div className="pointer-events-none absolute inset-0 flex items-center justify-center gap-2 rounded-md bg-background/90 text-xs font-medium text-primary">
            <UploadCloud className="h-4 w-4" aria-hidden="true" />
            {t("Drop to load")}
          </div>
        )}
      </div>
    </div>
  );
}

function Faq() {
  const { t } = useI18n();
  const items: { question: string; answer: ReactNode }[] = [
    {
      question: t("Does any of this get sent to a server?"),
      answer: t(
        "No. There is no server: the schema pipeline (building the form, validating, rendering the document) runs in this tab via WebAssembly. You can disconnect from the network after the page loads and it keeps working.",
      ),
    },
    {
      question: t("What formats can I paste or drop?"),
      answer: t(
        "JSON, YAML or TOML, for both the schema and the initial data — whichever you have on hand. The format is detected automatically, the same way schemaui-cli does for a file passed without --format.",
      ),
    },
    {
      question: t("What happens to my data if I close the tab?"),
      answer: t(
        "It's gone — nothing is saved automatically. Use \"Export\" once you're done filling out the form to download the result as a file first.",
      ),
    },
    {
      question: t("Where do I report a bug or ask something else?"),
      answer: (
        <a
          href={`${SCHEMAUI_URL}/issues`}
          target="_blank"
          rel="noreferrer"
          className="font-medium text-foreground underline-offset-4 hover:underline"
        >
          {t("Open an issue on the SchemaUI repository")}
        </a>
      ),
    },
  ];
  return (
    <div className="w-full max-w-5xl rounded-lg border border-theme bg-card px-4 py-3">
      <h2 className="mb-1.5 text-xs font-semibold text-foreground">
        {t("Frequently asked questions")}
      </h2>
      <div className="divide-y divide-border">
        {items.map((item) => (
          <details key={item.question} className="group py-2 first:pt-0 last:pb-0">
            <summary className="flex cursor-pointer list-none items-center justify-between gap-2 text-xs font-medium text-foreground marker:content-none">
              {item.question}
              <ChevronDown
                className="h-3.5 w-3.5 shrink-0 text-muted-foreground transition-transform group-open:rotate-180"
                aria-hidden="true"
              />
            </summary>
            <p className="mt-1.5 text-xs leading-relaxed text-muted-foreground">
              {item.answer}
            </p>
          </details>
        ))}
      </div>
    </div>
  );
}

/**
 * A friend-project callout, first screen only (see `AppHeader` for the
 * SchemaUI mark that appears everywhere else instead). Placed at the top:
 * the main card below it is the thing visitors came here to use, but this is
 * the first thing on the page precisely so it isn't missed by anyone who
 * doesn't scroll past the fold on a short screen.
 */
function SyzygyPromo() {
  const { t } = useI18n();
  return (
    <div className="flex w-full max-w-5xl items-center gap-3 rounded-lg border border-theme bg-card px-4 py-3">
      <img
        src={SYZYGY_ICON_URL}
        alt=""
        aria-hidden="true"
        className="h-9 w-9 shrink-0 rounded-md"
      />
      <div className="min-w-0 flex-1">
        <p className="text-xs font-semibold text-foreground">
          {t("Enjoying local-first, no-server tools? Try Syzygy")}
        </p>
        <p className="truncate text-xs text-muted-foreground">
          {t("A local-first, peer-to-peer clipboard workbench for keeping devices in sync without a cloud in between.")}
        </p>
      </div>
      <div className="flex shrink-0 items-center gap-3 text-xs text-muted-foreground">
        <a
          href={SYZYGY_SITE_URL}
          target="_blank"
          rel="noreferrer"
          className="hidden underline-offset-4 hover:text-foreground hover:underline sm:inline"
        >
          syzygysync.com
        </a>
        <a
          href={SYZYGY_REPO_URL}
          target="_blank"
          rel="noreferrer"
          className="flex items-center gap-1 hover:text-foreground"
          title={t("Syzygy on GitHub")}
        >
          <ExternalLink className="h-3.5 w-3.5" aria-hidden="true" />
        </a>
      </div>
    </div>
  );
}
