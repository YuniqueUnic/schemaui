import { memo } from "react";
import { Moon, Power, Save, Sun } from "lucide-react";
import { useTheme } from "../theme";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

interface AppHeaderProps {
  title?: string | null;
  description?: string | null;
  dirty: boolean;
  saving: boolean;
  exiting?: boolean;
  onSave(): void;
  onExit(): void;
}

export const AppHeader = memo(function AppHeader({
  title,
  description,
  dirty,
  saving,
  exiting = false,
  onSave,
  onExit,
}: AppHeaderProps) {
  const { theme, toggle } = useTheme();
  return (
    <header className="app-panel flex flex-wrap items-center justify-between gap-4 md:gap-6 border-b border-theme px-4 md:px-8 py-4 md:py-6 text-[var(--app-text)]">
      <div className="min-w-0 flex-1">
        <p className="text-xs uppercase tracking-[0.3em] text-muted">
          SchemaUI Web
        </p>
        <h1 className="truncate text-base md:text-lg font-semibold">
          {title || "Configuration session"}
        </h1>
        {description
          ? (
            <p className="mt-1 line-clamp-2 text-xs md:text-sm text-muted hidden sm:block">
              {description}
            </p>
          )
          : null}
      </div>
      <div className="flex flex-wrap items-center gap-2 md:gap-3">
        <Badge
          variant={dirty ? "default" : "secondary"}
          className="px-3 md:px-4 py-1 text-xs uppercase tracking-wide"
        >
          {dirty ? "Unsaved" : "Synced"}
        </Badge>

        <Button
          variant="outline"
          size="sm"
          onClick={onSave}
          disabled={saving}
          className="hidden sm:flex"
        >
          <Save className="h-4 w-4" />
          <span className="hidden md:inline">Save</span>
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={onSave}
          disabled={saving}
          className="sm:hidden"
        >
          <Save className="h-4 w-4" />
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={toggle}
          className="hidden sm:flex"
        >
          {theme === "dark"
            ? (
              <>
                <Sun className="h-4 w-4" />
                <span className="hidden md:inline">Light</span>
              </>
            )
            : (
              <>
                <Moon className="h-4 w-4" />
                <span className="hidden md:inline">Dark</span>
              </>
            )}
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={toggle}
          className="sm:hidden"
        >
          {theme === "dark"
            ? <Sun className="h-4 w-4" />
            : <Moon className="h-4 w-4" />}
        </Button>
        <Button
          variant="destructive"
          size="sm"
          onClick={onExit}
          disabled={saving || exiting}
          className="hidden sm:flex"
        >
          <Power className="h-4 w-4" />
          <span className="hidden md:inline">
            {exiting ? "Exiting…" : "Exit"}
          </span>
        </Button>
        <Button
          variant="destructive"
          size="sm"
          onClick={onExit}
          disabled={saving || exiting}
          className="sm:hidden"
        >
          <Power className="h-4 w-4" />
        </Button>
      </div>
    </header>
  );
});
