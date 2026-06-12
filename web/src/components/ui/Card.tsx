import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/cn";

export function Card({
  className,
  elevated,
  ...rest
}: HTMLAttributes<HTMLDivElement> & { elevated?: boolean }) {
  return (
    <div
      className={cn("card", elevated && "card-elevated", className)}
      {...rest}
    />
  );
}

export function CardHeader({
  title,
  description,
  actions,
  className,
}: {
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex items-start justify-between gap-4 px-5 py-4 border-b", className)}
         style={{ borderColor: "var(--border)" }}>
      <div className="min-w-0">
        <div className="text-[15px] font-semibold tracking-tight">{title}</div>
        {description && (
          <div className="mt-0.5 text-[13px]" style={{ color: "var(--fg-muted)" }}>
            {description}
          </div>
        )}
      </div>
      {actions && <div className="flex items-center gap-2 flex-shrink-0">{actions}</div>}
    </div>
  );
}
