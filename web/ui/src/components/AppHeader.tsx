import { memo } from "react";
import { ArrowLeft, ExternalLink, Moon, Power, Save, Sun } from "lucide-react";
import { useTheme } from "../theme";
import { useI18n, type Locale } from "../i18n";
import { Button } from "@/components/ui/button";
import { CountdownBadge } from "./CountdownBadge";

const SCHEMAUI_URL = "https://github.com/yuniqueunic/schemaui";

const LOCALES: Array<{ id: Locale; label: string }> = [
  { id: "en", label: "EN" },
  { id: "zh", label: "中文" },
];

interface AppHeaderProps {
  title?: string | null;
  description?: string | null;
  saving: boolean;
  exiting?: boolean;
  /** Whole seconds until the session closes, or null when unbounded. */
  secondsLeft?: number | null;
  onSave(): void;
  onExit(): void;
  /** "Exit" for a server session; "Export" when exiting downloads a file
   * instead (the Playground, which has no server to hand the document to).
   * Both arrive already translated. */
  exitLabel?: string;
  exitingLabel?: string;
  /** Present only for the Playground: there is a paste screen to return to,
   * with what was typed still in it. A server session has nothing to go back
   * to — closing it back at `/session` is what Exit already means — so this
   * button does not exist there at all rather than being disabled. */
  onBack?: () => void;
}

/**
 * The session's identity bar: what is being edited, and how long is left.
 *
 * Deliberately carries no save state. The status bar at the bottom owns that,
 * and it knows more than a badge could — "saving", "3 errors", "exiting" all
 * collapse into one tone there. Showing "Unsaved" in both places meant the same
 * fact appeared twice in two different styles.
 */
export const AppHeader = memo(function AppHeader({
  title,
  description,
  saving,
  exiting = false,
  secondsLeft = null,
  onSave,
  onExit,
  exitLabel,
  exitingLabel,
  onBack,
}: AppHeaderProps) {
  const { theme, toggle } = useTheme();
  const { t, locale, setLocale } = useI18n();
  const exitText = exitLabel ?? t("Exit");
  const exitingText = exitingLabel ?? t("Exiting…");
  return (
    <header className="border-b border-border/50 bg-background px-4 py-3 md:px-6 md:py-3.5">
      <div className="flex items-center justify-between gap-4">
        {/* Left: Brand, Title & Description */}
        <div className="flex min-w-0 flex-1 items-center gap-3">
          {onBack && (
            <Button
              variant="ghost"
              size="sm"
              onClick={onBack}
              title={t("Back to schema")}
              className="shrink-0"
            >
              <ArrowLeft className="h-4 w-4" />
              <span className="hidden md:inline ml-1">{t("Back")}</span>
            </Button>
          )}
          <a
            href={SCHEMAUI_URL}
            target="_blank"
            rel="noreferrer"
            title={t("SchemaUI on GitHub")}
            className="hidden shrink-0 items-center gap-1 text-xs font-semibold text-muted-foreground transition-colors hover:text-foreground sm:flex"
          >
            <ExternalLink className="h-3.5 w-3.5" aria-hidden="true" />
            SchemaUI
          </a>
          <span
            aria-hidden="true"
            className="hidden h-4 w-px shrink-0 bg-border sm:block"
          />
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <h1 className="truncate text-sm font-semibold text-foreground md:text-[15px]">
                {title || t("Configuration session")}
              </h1>
              {secondsLeft != null && <CountdownBadge secondsLeft={secondsLeft} />}
            </div>
            {description && (
              <p className="mt-0.5 hidden truncate text-xs text-muted-foreground sm:block">
                {description}
              </p>
            )}
          </div>
        </div>

        {/* Right: Actions */}
        <div className="flex items-center gap-1.5">
          <Button
            variant="ghost"
            size="sm"
            onClick={onSave}
            disabled={saving}
            title={t("Save (Ctrl+S)")}
          >
            <Save className="h-4 w-4" />
            <span className="hidden md:inline ml-1">{t("Save")}</span>
          </Button>

          <Button
            variant="ghost"
            size="sm"
            onClick={toggle}
            title={theme === "dark" ? t("Switch to light mode") : t("Switch to dark mode")}
          >
            {theme === "dark"
              ? <Sun className="h-4 w-4" />
              : <Moon className="h-4 w-4" />}
            <span className="hidden md:inline ml-1">
              {theme === "dark" ? t("Light") : t("Dark")}
            </span>
          </Button>

          {/* Language names are never translated — "EN" means English in
              every locale — so the group label is the only translated part. */}
          <div
            role="group"
            aria-label={t("Language")}
            data-testid="language-switcher"
            className="flex shrink-0 items-center rounded-md border border-border p-0.5"
          >
            {LOCALES.map((option) => (
              <button
                key={option.id}
                type="button"
                aria-pressed={locale === option.id}
                onClick={() => setLocale(option.id)}
                className={`rounded-sm px-1.5 py-0.5 text-[11px] leading-none transition-colors ${
                  locale === option.id
                    ? "bg-foreground/10 font-semibold text-foreground"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                {option.label}
              </button>
            ))}
          </div>

          <Button
            variant="ghost"
            size="sm"
            onClick={onExit}
            disabled={saving || exiting}
            className="text-destructive hover:text-destructive hover:bg-destructive/10"
            title={exitText}
          >
            <Power className="h-4 w-4" />
            <span className="hidden md:inline ml-1">
              {exiting ? exitingText : exitText}
            </span>
          </Button>
        </div>
      </div>
    </header>
  );
});
