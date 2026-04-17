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
  dense?: boolean;
}

/**
 * PanelHeader — consistent header strip for every panel:
 * small uppercase label on the left, optional icon, actions on the right.
 */
export function PanelHeader({
  label,
  actions,
  icon,
  dense = false,
  className,
  children,
  ...rest
}: PanelHeaderProps) {
  return (
    <div
      className={cn(
        "flex items-center justify-between gap-2 border-b border-theme bg-panel/60",
        dense ? "px-3 py-2" : "px-4 py-3",
        className,
      )}
      {...rest}
    >
      <div className="flex items-center gap-2 min-w-0">
        {icon && (
          <span className="text-muted-foreground shrink-0 flex items-center">
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
