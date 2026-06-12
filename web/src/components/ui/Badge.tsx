import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

export type BadgeTone = "default" | "accent" | "ok" | "warn" | "bad" | "info";

/** Soft dot + label, intentionally not a coloured pill. */
export function Badge({
  tone = "default",
  className,
  children,
}: {
  tone?: BadgeTone;
  className?: string;
  children: ReactNode;
}) {
  return (
    <span
      className={cn(
        "badge",
        tone === "accent" && "badge-accent",
        tone === "ok" && "badge-ok",
        tone === "warn" && "badge-warn",
        tone === "bad" && "badge-bad",
        tone === "info" && "badge-info",
        className
      )}
    >
      {children}
    </span>
  );
}

/** Small bordered chip for keywords / tags. */
export function Tag({
  className,
  children,
}: {
  className?: string;
  children: ReactNode;
}) {
  return <span className={cn("tag", className)}>{children}</span>;
}
