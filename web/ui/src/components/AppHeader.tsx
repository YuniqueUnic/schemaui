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
    <header className="bg-background border-b border-border/50 px-4 md:px-6 py-3 md:py-4">
      <div className="flex items-center justify-between gap-4">
        {/* Left: Title & Description */}
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h1 className="truncate text-sm md:text-base font-semibold text-foreground">
              {title || "Configuration session"}
            </h1>
            <Badge
              variant={dirty ? "default" : "secondary"}
              className="text-[10px] uppercase tracking-wide shrink-0"
            >
              {dirty ? "Unsaved" : "Saved"}
            </Badge>
          </div>
          {description && (
            <p className="mt-0.5 truncate text-xs text-muted-foreground hidden sm:block">
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
