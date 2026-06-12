import type { ReactNode } from "react";

export function PageHeader({
  eyebrow,
  title,
  description,
  actions,
}: {
  eyebrow?: ReactNode;
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <header className="mb-12 mt-4">
      {eyebrow && (
        <div
          className="text-[13px] font-medium mb-4"
          style={{ color: "var(--fg-subtle)" }}
        >
          {eyebrow}
        </div>
      )}
      <h1 className="display-1 reveal max-w-3xl">{title}</h1>
      {description && (
        <p
          className="mt-5 text-[16.5px] leading-[1.55] max-w-xl"
          style={{ color: "var(--fg-muted)" }}
        >
          {description}
        </p>
      )}
      {actions && (
        <div className="mt-7 flex flex-wrap items-center gap-3">{actions}</div>
      )}
    </header>
  );
}
