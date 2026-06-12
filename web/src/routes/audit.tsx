import { PageHeader } from "@/components/ui/PageHeader";
import { Badge, type BadgeTone } from "@/components/ui/Badge";
import { useT } from "@/i18n";
import type { TKey } from "@/i18n/dict";

const ENTRIES: { ts: string; actor: string; action: TKey; target: string; tone: BadgeTone; what: TKey }[] = [
  { ts: "12 min ago", actor: "opus-4.7", action: "audit.act.submittedIteration", target: "data-eng / pdf-parser", tone: "info",   what: "audit.what.proposal" },
  { ts: "15 min ago", actor: "ada",      action: "audit.act.openedProposal",     target: "data-eng / pdf-parser", tone: "info",   what: "audit.what.ocr" },
  { ts: "2 hr ago",   actor: "ada",      action: "audit.act.flaggedDuplicate",   target: "data-eng / pdf-extract", tone: "warn",  what: "audit.what.cosine" },
  { ts: "5 hr ago",   actor: "carol",    action: "audit.act.wasDenied",          target: "finance / finance-reconciler", tone: "bad", what: "audit.what.denied" },
  { ts: "yesterday",  actor: "admin",    action: "audit.act.issuedGrant",        target: "data-eng → finance",    tone: "accent", what: "audit.what.grant" },
];

export function AuditPage() {
  const t = useT();
  return (
    <>
      <PageHeader
        eyebrow={t("audit.eyebrow")}
        title={
          <>
            {t("audit.titleLead")}<span className="serif-em">{t("audit.titleEm")}</span>
          </>
        }
        description={t("audit.desc")}
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
              <Badge tone={e.tone}>{t(e.action)}</Badge>
              <span className="text-[12.5px]" style={{ color: "var(--fg-faint)" }}>{e.ts}</span>
            </div>
            <div className="text-[15px]" style={{ color: "var(--fg)" }}>
              <span className="font-mono text-[13.5px]" style={{ color: "var(--fg-muted)" }}>
                {e.actor}
              </span>{" "}
              {t(e.action)}{" "}
              <span className="font-mono">{e.target}</span>
            </div>
            <div className="text-[13px] mt-1" style={{ color: "var(--fg-muted)" }}>
              {t(e.what)}
            </div>
          </li>
        ))}
      </ul>
    </>
  );
}
