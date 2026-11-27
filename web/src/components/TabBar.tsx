import { cn } from "../lib/cn";

export interface TabOption<T extends string> {
  id: T;
  label: string;
  badge?: number;
}

interface TabBarProps<T extends string> {
  label: string;
  tabs: readonly TabOption<T>[];
  active: T;
  onSelect: (id: T) => void;
  className?: string;
}

export function TabBar<T extends string>({
  label,
  tabs,
  active,
  onSelect,
  className,
}: TabBarProps<T>) {
  return (
    <div
      role="tablist"
      aria-label={label}
      className={cn("inline-flex items-center gap-1 rounded-md border border-line bg-app p-0.5", className)}
    >
      {tabs.map((tab) => {
        const isActive = tab.id === active;
        return (
          <button
            key={tab.id}
            type="button"
            role="tab"
            aria-selected={isActive}
            onClick={() => onSelect(tab.id)}
            className={cn(
              "inline-flex items-center gap-1.5 rounded-sm px-2.5 py-1 text-xs font-medium transition-colors",
              isActive ? "bg-raised text-fg" : "text-fg-3 hover:text-fg",
            )}
          >
            <span>{tab.label}</span>
            {tab.badge && tab.badge > 0 ? (
              <span className="rounded-sm bg-accent-soft px-1 font-mono text-[10px] text-accent">
                {tab.badge}
              </span>
            ) : null}
          </button>
        );
      })}
    </div>
  );
}
