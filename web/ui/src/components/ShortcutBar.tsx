interface ShortcutBarProps {
  shortcuts?: { combo: string; label: string }[];
}

const DEFAULT_SHORTCUTS: { combo: string; label: string }[] = [
  { combo: 'Ctrl/Cmd + S', label: 'Save' },
  { combo: 'Ctrl/Cmd + Enter', label: 'Validate' },
  { combo: 'Ctrl/Cmd + .' , label: 'Toggle theme' },
];

export function ShortcutBar({ shortcuts = DEFAULT_SHORTCUTS }: ShortcutBarProps) {
  return (
    <footer className="flex items-center gap-4 border-t border-slate-800/80 bg-slate-900/60 px-6 py-2 text-xs text-slate-300">
      {shortcuts.map((shortcut) => (
        <span key={shortcut.combo} className="inline-flex items-center gap-2">
          <kbd className="rounded-md border border-slate-700/70 bg-slate-900/80 px-2 py-1 text-[10px] font-semibold text-slate-100">
            {shortcut.combo}
          </kbd>
          <span>{shortcut.label}</span>
        </span>
      ))}
    </footer>
  );
}
