import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/utils";

interface PanelProps extends HTMLAttributes<HTMLElement> {
  as?: "aside" | "section" | "main" | "div";
  tone?: "default" | "muted" | "subtle";
}

/**
 * Panel — the shell wrapper for the three main app columns (nav / editor / preview).
 * Encapsulates the "app-panel" surface + flex column layout.
 */
export function Panel({
  as: Tag = "section",
  tone = "default",
  className,
  children,
  ...rest
}: PanelProps) {
  const toneClass = tone === "muted"
    ? "app-panel-muted"
    : tone === "subtle"
    ? "app-panel-subtle"
    : "app-panel";
  return (
    <Tag
      className={cn("flex flex-col overflow-hidden", toneClass, className)}
      {...rest}
    >
      {children}
    </Tag>
  );
}

interface PanelHeaderProps extends HTMLAttributes<HTMLDivElement> {
  label?: ReactNode;
  actions?: ReactNode;
  icon?: ReactNode;
}

/**
 * PanelHeader — consistent header strip for every panel:
 * small uppercase label on the left, optional icon, actions on the right.
 *
 * The height comes from `.app-panel-header` rather than from its own padding,
 * so it matches the editor's tab band, which is not a PanelHeader at all.
 */
export function PanelHeader({
  label,
  actions,
  icon,
  className,
  children,
  ...rest
}: PanelHeaderProps) {
  return (
    <div
      className={cn(
        "app-panel-header justify-between gap-2 border-b border-theme bg-panel/60 px-4",
        className,
      )}
      {...rest}
    >
      <div className="flex min-w-0 items-center gap-2">
        {icon && (
          <span className="flex shrink-0 items-center text-muted-foreground">
            {icon}
          </span>
        )}
        {label && <span className="section-label truncate">{label}</span>}
        {children}
      </div>
      {actions && <div className="flex items-center gap-1.5">{actions}</div>}
    </div>
  );
}

interface PanelBodyProps extends HTMLAttributes<HTMLDivElement> {
  padded?: boolean;
  scroll?: boolean;
}

export function PanelBody({
  padded = false,
  scroll = true,
  className,
  children,
  ...rest
}: PanelBodyProps) {
  return (
    <div
      className={cn(
        "flex-1 min-h-0",
        scroll && "overflow-auto",
        padded && "px-4 py-4",
        className,
      )}
      {...rest}
    >
      {children}
    </div>
  );
}
