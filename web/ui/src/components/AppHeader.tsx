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
    <header className="app-panel flex flex-wrap items-center justify-between gap-6 border-b border-theme px-8 py-6 text-[var(--app-text)]">
      <div className="min-w-0 flex-1">
        <p className="text-xs uppercase tracking-[0.3em] text-muted">
          SchemaUI Web
        </p>
        <h1 className="truncate text-lg font-semibold">
          {title || "Configuration session"}
        </h1>
        {description
          ? (
            <p className="mt-1 line-clamp-2 text-sm text-muted">
              {description}
            </p>
          )
          : null}
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <Badge
          variant={dirty ? "default" : "secondary"}
          className="px-4 py-1 text-xs uppercase tracking-wide"
        >
          {dirty ? "Unsaved" : "Synced"}
        </Badge>

        <Button
          variant="outline"
          size="sm"
          onClick={onSave}
          disabled={saving}
        >
          <Save className="h-4 w-4" />
          Save
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={toggle}
        >
          {theme === "dark"
            ? (
              <>
                <Sun className="h-4 w-4" /> Light
              </>
            )
            : (
              <>
                <Moon className="h-4 w-4" /> Dark
              </>
            )}
        </Button>
        <Button
          variant="destructive"
          size="sm"
          onClick={onExit}
          disabled={saving || exiting}
        >
          <Power className="h-4 w-4" />
          {exiting ? "Exiting…" : "Exit"}
        </Button>
      </div>
    </header>
  );
});
