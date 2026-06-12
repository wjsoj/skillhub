import { PageHeader } from "@/components/ui/PageHeader";
import { Badge, type BadgeTone } from "@/components/ui/Badge";

const ENTRIES: { ts: string; actor: string; action: string; target: string; tone: BadgeTone; what: string }[] = [
  { ts: "12 min ago", actor: "opus-4.7", action: "submitted iteration", target: "data-eng / pdf-parser", tone: "info",   what: "→ proposal 7b3f…" },
  { ts: "15 min ago", actor: "ada",      action: "opened proposal",     target: "data-eng / pdf-parser", tone: "info",   what: "0.3.0 with OCR fallback" },
  { ts: "2 hr ago",   actor: "ada",      action: "flagged duplicate",   target: "data-eng / pdf-extract", tone: "warn",  what: "cosine 0.91" },
  { ts: "5 hr ago",   actor: "carol",    action: "was denied",          target: "finance / finance-reconciler", tone: "bad", what: "no rule allowed read" },
  { ts: "yesterday",  actor: "admin",    action: "issued grant",        target: "data-eng → finance",    tone: "accent", what: "read · 14 days" },
];

export function AuditPage() {
  return (
    <>
      <PageHeader
        eyebrow="Audit"
        title={
          <>
            Everything that <span className="serif-em">happens.</span>
          </>
        }
        description="Append-only. Every decision the registry made, plus every state change you'd want to know about."
      />

      <ul>
        {ENTRIES.map((e, i) => (
          <li
            key={i}
            className="py-5"
            style={{
              borderTop: i === 0 ? "1px solid var(--border)" : "0",
              borderBottom: "1px solid var(--border)",
            }}
          >
            <div className="flex items-baseline justify-between gap-3 mb-1.5">
              <Badge tone={e.tone}>{e.action}</Badge>
              <span className="text-[12.5px]" style={{ color: "var(--fg-faint)" }}>{e.ts}</span>
            </div>
            <div className="text-[15px]" style={{ color: "var(--fg)" }}>
              <span className="font-mono text-[13.5px]" style={{ color: "var(--fg-muted)" }}>
                {e.actor}
              </span>{" "}
              {e.action}{" "}
              <span className="font-mono">{e.target}</span>
            </div>
            <div className="text-[13px] mt-1" style={{ color: "var(--fg-muted)" }}>
              {e.what}
            </div>
          </li>
        ))}
      </ul>
    </>
  );
}
