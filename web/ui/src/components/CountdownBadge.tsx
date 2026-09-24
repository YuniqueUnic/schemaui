import { memo } from "react";
import { Timer } from "lucide-react";
import { cn } from "@/lib/utils";
import { useI18n } from "../i18n";
import { countdownUrgency, formatCountdown } from "../hooks/useCountdown";

interface CountdownBadgeProps {
  /** Whole seconds left; the caller owns the ticking. */
  secondsLeft: number;
  className?: string;
}

/**
 * Remaining time before the backend aborts this session.
 *
 * Sits next to the title so the deadline is visible without hunting for it —
 * the whole point is that a timeout should never arrive unannounced. The tone
 * escalates as the deadline approaches, and the tooltip states the consequence
 * outright, because "time runs out" is only actionable if you know the answers
 * are lost with it.
 */
export const CountdownBadge = memo(function CountdownBadge({
  secondsLeft,
  className,
}: CountdownBadgeProps) {
  const { t } = useI18n();
  const urgency = countdownUrgency(secondsLeft);

  // Escalates through weight, then colour: quiet outline → full-contrast text
  // → destructive. Three steps is enough to notice a deadline approaching
  // without turning the header into a traffic light.
  const toneClass = {
    calm: "border-border/70 bg-muted/60 text-muted-foreground",
    warning: "border-foreground/30 bg-muted text-foreground",
    critical: "border-destructive/40 bg-destructive/10 text-destructive",
  }[urgency];

  return (
    <span
      role="timer"
      aria-live="off"
      aria-label={t("{time} until this session closes", {
        time: formatCountdown(secondsLeft),
      })}
      title={t(
        "This session closes when the countdown reaches zero. Anything unsaved is discarded.",
      )}
      className={cn(
        "inline-flex shrink-0 items-center gap-1 rounded-full border px-2 py-0.5",
        "text-[11px] font-semibold tabular-nums",
        toneClass,
        urgency === "critical" && "animate-pulse",
        className,
      )}
    >
      <Timer className="h-3 w-3" aria-hidden="true" />
      <span>{formatCountdown(secondsLeft)}</span>
    </span>
  );
});
