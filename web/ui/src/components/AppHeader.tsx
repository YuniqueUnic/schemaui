import { memo } from "react";
import { Moon, Power, Save, Sun } from "lucide-react";
import { useTheme } from "../theme";
import { Button } from "@/components/ui/button";
import { CountdownBadge } from "./CountdownBadge";

interface AppHeaderProps {
  title?: string | null;
  description?: string | null;
  saving: boolean;
  exiting?: boolean;
  /** Whole seconds until the session closes, or null when unbounded. */
  secondsLeft?: number | null;
  onSave(): void;
  onExit(): void;
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
}: AppHeaderProps) {
  const { theme, toggle } = useTheme();
  return (
    <header className="border-b border-border/50 bg-background px-4 py-3 md:px-6 md:py-3.5">
      <div className="flex items-center justify-between gap-4">
        {/* Left: Title & Description */}
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h1 className="truncate text-sm font-semibold text-foreground md:text-[15px]">
              {title || "Configuration session"}
            </h1>
            {secondsLeft != null && <CountdownBadge secondsLeft={secondsLeft} />}
          </div>
          {description && (
            <p className="mt-0.5 hidden truncate text-xs text-muted-foreground sm:block">
              {description}
            </p>
          )}
        </div>

        {/* Right: Actions */}
        <div className="flex items-center gap-1.5">
          <Button
            variant="ghost"
            size="sm"
            onClick={onSave}
            disabled={saving}
            title="Save (Ctrl+S)"
          >
            <Save className="h-4 w-4" />
            <span className="hidden md:inline ml-1">Save</span>
          </Button>

          <Button
            variant="ghost"
            size="sm"
            onClick={toggle}
            title={`Switch to ${theme === "dark" ? "light" : "dark"} mode`}
          >
            {theme === "dark"
              ? <Sun className="h-4 w-4" />
              : <Moon className="h-4 w-4" />}
            <span className="hidden md:inline ml-1">
              {theme === "dark" ? "Light" : "Dark"}
            </span>
          </Button>

          <Button
            variant="ghost"
            size="sm"
            onClick={onExit}
            disabled={saving || exiting}
            className="text-destructive hover:text-destructive hover:bg-destructive/10"
            title="Exit"
          >
            <Power className="h-4 w-4" />
            <span className="hidden md:inline ml-1">
              {exiting ? "Exiting…" : "Exit"}
            </span>
          </Button>
        </div>
      </div>
    </header>
  );
});
