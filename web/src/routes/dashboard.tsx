import { Link } from "@tanstack/react-router";
import { ArrowRight, Sparkles, Bot, Shield, GitPullRequest } from "lucide-react";
import { PageHeader } from "@/components/ui/PageHeader";
import { Badge } from "@/components/ui/Badge";
import { getMockUser } from "@/lib/api";

const RECENT = [
  { actor: "opus-4.7", action: "submitted iteration", target: "pdf-parser",   tone: "info"   as const, when: "12 min ago" },
  { actor: "ada",      action: "opened proposal",     target: "pdf-parser",   tone: "info"   as const, when: "15 min ago" },
  { actor: "ada",      action: "flagged duplicate",   target: "pdf-extract",  tone: "warn"   as const, when: "2 hr ago" },
  { actor: "carol",    action: "was denied",          target: "finance-reconciler", tone: "bad" as const, when: "5 hr ago" },
  { actor: "admin",    action: "granted access",      target: "data-eng → finance", tone: "accent" as const, when: "yesterday" },
];

const QUICK = [
  { icon: Sparkles,        label: "Compose a new skill", to: "/skills/new" as const },
  { icon: GitPullRequest,  label: "Browse the registry", to: "/skills"     as const },
  { icon: Bot,             label: "Open an iteration",   to: "/skills"     as const },
  { icon: Shield,          label: "Issue a grant",       to: "/grants"     as const },
];

export function DashboardPage() {
  const user = getMockUser();
  const name = user?.name || "friend";
  const hour = new Date().getHours();
  const greeting = hour < 12 ? "Morning" : hour < 18 ? "Afternoon" : "Evening";

  return (
    <>
      <PageHeader
        eyebrow="Home"
        title={
          <>
            {greeting}, <span className="serif-em">{name}.</span>
          </>
        }
        description="A small, private place to publish and share agent skills with the rest of your team. No drama, no leaks across departments."
      />

      <section className="mb-16">
        <ul className="grid grid-cols-1 sm:grid-cols-2 gap-px overflow-hidden rounded-2xl" style={{ background: "var(--border)" }}>
          {QUICK.map((q) => (
            <li key={q.label}>
              <Link
                to={q.to}
                className="group flex items-center gap-4 px-5 py-5 transition-colors"
                style={{ background: "var(--surface)", cursor: "pointer" }}
              >
                <span
                  className="inline-flex items-center justify-center w-9 h-9 rounded-full flex-shrink-0"
                  style={{ background: "var(--accent-soft)", color: "var(--accent)" }}
                >
                  <q.icon size={16} />
                </span>
                <span className="text-[15px] font-medium flex-1">{q.label}</span>
                <ArrowRight
                  size={15}
                  className="opacity-40 group-hover:opacity-100 group-hover:translate-x-0.5 transition-all"
                  style={{ color: "var(--fg-muted)" }}
                />
              </Link>
            </li>
          ))}
        </ul>
      </section>

      <section>
        <div className="flex items-baseline justify-between mb-6">
          <h2 className="display-2">What's happening</h2>
          <Link to="/audit" className="text-[13.5px]" style={{ color: "var(--fg-muted)" }}>
            View all →
          </Link>
        </div>
        <ul>
          {RECENT.map((r, i) => (
            <li
              key={i}
              className="flex items-baseline justify-between gap-4 py-4"
              style={{ borderBottom: i === RECENT.length - 1 ? "0" : "1px solid var(--border)" }}
            >
              <div className="flex items-baseline gap-3 min-w-0">
                <span className="font-mono text-[12.5px]" style={{ color: "var(--fg-muted)" }}>
                  {r.actor}
                </span>
                <span className="text-[14.5px] flex-1 min-w-0">
                  <span style={{ color: "var(--fg-muted)" }}>{r.action}</span>{" "}
                  <span className="font-mono" style={{ color: "var(--fg)" }}>{r.target}</span>
                </span>
              </div>
              <div className="flex items-center gap-3 flex-shrink-0">
                <Badge tone={r.tone}>{r.tone === "bad" ? "denied" : r.tone === "warn" ? "warn" : r.tone === "accent" ? "grant" : "ok"}</Badge>
                <span className="text-[12.5px]" style={{ color: "var(--fg-faint)" }}>{r.when}</span>
              </div>
            </li>
          ))}
        </ul>
      </section>
    </>
  );
}
