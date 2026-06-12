import { Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import {
  ArrowRight,
  ArrowUpRight,
  Sparkles,
  Bot,
  Shield,
  GitPullRequest,
  Package,
  Download,
  Boxes,
} from "lucide-react";
import { Badge, Tag } from "@/components/ui/Badge";
import {
  Reveal,
  StaggerGroup,
  StaggerItem,
  HoverTile,
  CountUp,
} from "@/components/ui/Motion";
import { getMockUser, listSkills, type Skill } from "@/lib/api";
import { useT } from "@/i18n";
import type { TKey } from "@/i18n/dict";

const RECENT = [
  { actor: "opus-4.7", action: "act.submittedIteration", target: "pdf-parser", tone: "info", badge: "act.badge.ok", when: "12 min ago" },
  { actor: "ada", action: "act.openedProposal", target: "pdf-parser", tone: "info", badge: "act.badge.ok", when: "15 min ago" },
  { actor: "ada", action: "act.flaggedDuplicate", target: "pdf-extract", tone: "warn", badge: "act.badge.warn", when: "2 hr ago" },
  { actor: "carol", action: "act.wasDenied", target: "finance-reconciler", tone: "bad", badge: "act.badge.denied", when: "5 hr ago" },
  { actor: "admin", action: "act.grantedAccess", target: "data-eng → finance", tone: "accent", badge: "act.badge.grant", when: "yesterday" },
] as const satisfies ReadonlyArray<{
  actor: string; action: TKey; target: string;
  tone: "info" | "warn" | "bad" | "accent"; badge: TKey; when: string;
}>;

const QUICK = [
  { icon: Sparkles, label: "dash.quick.compose", to: "/skills/new" as const },
  { icon: GitPullRequest, label: "dash.quick.browse", to: "/skills" as const },
  { icon: Bot, label: "dash.quick.iteration", to: "/skills" as const },
  { icon: Shield, label: "dash.quick.grant", to: "/grants" as const },
] as const satisfies ReadonlyArray<{ icon: typeof Sparkles; label: TKey; to: string }>;

export function DashboardPage() {
  const t = useT();
  const user = getMockUser();
  const name = user?.name || "friend";
  const hour = new Date().getHours();
  const greetingKey: TKey =
    hour < 12 ? "dash.greeting.morning" : hour < 18 ? "dash.greeting.afternoon" : "dash.greeting.evening";
  const greeting = t(greetingKey);

  const q = useQuery({ queryKey: ["skills"], queryFn: listSkills });

  const stats = useMemo(() => {
    const rows: Skill[] = q.data ?? [];
    const installs = rows.reduce((a, s) => a + (s.install_count || 0), 0);
    const namespaces = new Set(rows.map((s) => s.namespace_slug)).size;
    const vis = { private: 0, team: 0, global: 0 } as Record<string, number>;
    for (const s of rows) vis[s.visibility] = (vis[s.visibility] ?? 0) + 1;
    const top = [...rows].sort((a, b) => b.install_count - a.install_count)[0];
    return { count: rows.length, installs, namespaces, vis, top };
  }, [q.data]);

  return (
    <div className="pt-4">
      {/* eyebrow */}
      <Reveal y={10}>
        <div className="eyebrow mb-7">{t("dash.eyebrow")}</div>
      </Reveal>

      {/* ── Bento grid ──────────────────────────────────────────────── */}
      <StaggerGroup className="grid grid-cols-1 lg:grid-cols-6 gap-4">
        {/* Hero identity — anchor tile, spans two rows */}
        <StaggerItem className="lg:col-span-4 lg:row-span-2">
          <HoverTile className="h-full">
            <div className="tile tile-hover h-full overflow-hidden p-8 sm:p-10 flex flex-col">
              <div className="badge badge-accent mb-6">{t("dash.signedInAs", { name })}</div>
              <h1 className="display-1 max-w-xl">
                {greeting}{t("dash.heroComma")}<span className="serif-em">{name}{t("dash.heroPeriod")}</span>
              </h1>
              <p className="mt-5 text-[16px] leading-[1.55] max-w-lg" style={{ color: "var(--fg-muted)" }}>
                {t("dash.heroBody")}
              </p>
              <div className="mt-8 flex flex-wrap items-center gap-3">
                <Link to="/skills/new" className="btn btn-primary btn-lg">
                  <Sparkles size={16} /> {t("dash.composeSkill")}
                </Link>
                <Link to="/skills" className="btn btn-secondary btn-lg">
                  {t("dash.browseRegistry")} <ArrowRight size={15} />
                </Link>
              </div>
              <div
                className="mt-auto pt-9 flex items-center gap-6 text-[12.5px]"
                style={{ color: "var(--fg-subtle)" }}
              >
                <span className="inline-flex items-center gap-1.5">
                  <Boxes size={13} /> {t("dash.meta.namespaces", { count: q.isLoading ? "—" : stats.namespaces })}
                </span>
                <span className="inline-flex items-center gap-1.5">
                  <Package size={13} /> {t("dash.meta.skills", { count: q.isLoading ? "—" : stats.count })}
                </span>
                <span className="inline-flex items-center gap-1.5">
                  <Download size={13} /> {t("dash.meta.installs", { count: q.isLoading ? "—" : fmtCompact(stats.installs) })}
                </span>
              </div>
            </div>
          </HoverTile>
        </StaggerItem>

        {/* Skills count */}
        <StaggerItem className="lg:col-span-2">
          <StatTile
            icon={Package}
            label={t("dash.stat.skillsVisible")}
            value={stats.count}
            loading={q.isLoading}
            foot={t("dash.stat.skillsFoot", { count: stats.namespaces || "—" })}
          />
        </StaggerItem>

        {/* Installs */}
        <StaggerItem className="lg:col-span-2">
          <StatTile
            icon={Download}
            label={t("dash.stat.totalInstalls")}
            value={stats.installs}
            loading={q.isLoading}
            foot={t("dash.stat.installsFoot")}
            accent
          />
        </StaggerItem>

        {/* Featured / most-installed skill */}
        <StaggerItem className="lg:col-span-4">
          <HoverTile className="h-full">
            <div className="tile tile-hover h-full p-7 sm:p-8 flex flex-col">
              <div className="flex items-center justify-between mb-5">
                <span className="eyebrow">{t("dash.mostInstalled")}</span>
                <Link to="/skills" className="text-[13px] inline-flex items-center gap-1" style={{ color: "var(--fg-muted)" }}>
                  {t("dash.seeAll")} <ArrowUpRight size={13} />
                </Link>
              </div>
              {stats.top ? (
                <FeaturedSkill skill={stats.top} />
              ) : (
                <div className="flex-1 flex items-center text-[14px]" style={{ color: "var(--fg-faint)" }}>
                  {q.isLoading ? t("dash.loadingRegistry") : t("dash.noSkills")}
                </div>
              )}
            </div>
          </HoverTile>
        </StaggerItem>

        {/* Visibility breakdown */}
        <StaggerItem className="lg:col-span-2">
          <HoverTile className="h-full">
            <div className="tile tile-hover h-full p-6">
              <div className="flex items-center gap-2 mb-5">
                <Boxes size={15} style={{ color: "var(--fg-subtle)" }} />
                <span className="eyebrow">{t("dash.visibility")}</span>
              </div>
              <VisBars vis={stats.vis} total={stats.count} loading={q.isLoading} />
            </div>
          </HoverTile>
        </StaggerItem>

        {/* Quick actions — full-width strip */}
        <StaggerItem className="lg:col-span-6">
          <div className="tile p-2.5">
            <ul className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-1">
              {QUICK.map((a) => (
                <li key={a.label}>
                  <Link
                    to={a.to}
                    className="group flex items-center gap-3 px-4 py-3.5 rounded-xl transition-colors"
                    style={{ cursor: "pointer" }}
                    onMouseEnter={(e) => (e.currentTarget.style.background = "var(--surface-2)")}
                    onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
                  >
                    <span
                      className="inline-flex items-center justify-center w-8 h-8 rounded-lg flex-shrink-0"
                      style={{ background: "var(--accent-soft)", color: "var(--accent)" }}
                    >
                      <a.icon size={15} />
                    </span>
                    <span className="text-[14px] font-medium flex-1">{t(a.label)}</span>
                    <ArrowRight
                      size={14}
                      className="opacity-30 group-hover:opacity-100 group-hover:translate-x-0.5 transition-all"
                      style={{ color: "var(--fg-muted)" }}
                    />
                  </Link>
                </li>
              ))}
            </ul>
          </div>
        </StaggerItem>
      </StaggerGroup>

      {/* ── Activity ─────────────────────────────────────────────────── */}
      <Reveal className="mt-14" delay={0.05}>
        <div className="flex items-baseline justify-between mb-5">
          <h2 className="display-2">{t("dash.whatsHappening")}</h2>
          <Link to="/audit" className="text-[13.5px] inline-flex items-center gap-1" style={{ color: "var(--fg-muted)" }}>
            {t("dash.viewAll")} <ArrowUpRight size={14} />
          </Link>
        </div>
        <div className="tile p-2 sm:p-3">
          <ul>
            {RECENT.map((r, i) => (
              <li
                key={i}
                className="flex items-baseline justify-between gap-4 px-4 py-3.5"
                style={{ borderBottom: i === RECENT.length - 1 ? "0" : "1px solid var(--border)" }}
              >
                <div className="flex items-baseline gap-3 min-w-0">
                  <span className="font-mono text-[12.5px]" style={{ color: "var(--fg-muted)" }}>{r.actor}</span>
                  <span className="text-[14.5px] flex-1 min-w-0">
                    <span style={{ color: "var(--fg-muted)" }}>{t(r.action)}</span>{" "}
                    <span className="font-mono" style={{ color: "var(--fg)" }}>{r.target}</span>
                  </span>
                </div>
                <div className="flex items-center gap-3 flex-shrink-0">
                  <Badge tone={r.tone}>{t(r.badge)}</Badge>
                  <span className="text-[12.5px] hidden sm:inline" style={{ color: "var(--fg-faint)" }}>{r.when}</span>
                </div>
              </li>
            ))}
          </ul>
        </div>
      </Reveal>
    </div>
  );
}

function StatTile({
  icon: Icon,
  label,
  value,
  foot,
  loading,
  accent,
}: {
  icon: typeof Package;
  label: string;
  value: number;
  foot: string;
  loading?: boolean;
  accent?: boolean;
}) {
  return (
    <HoverTile className="h-full">
      <div className="tile tile-hover h-full p-6 flex flex-col">
        <div className="flex items-center gap-2 mb-auto">
          <Icon size={15} style={{ color: accent ? "var(--accent)" : "var(--fg-subtle)" }} />
          <span className="eyebrow">{label}</span>
        </div>
        <div
          className="stat-num mt-8"
          style={{
            fontSize: "clamp(2.4rem, 4vw, 3.2rem)",
            color: accent ? "var(--accent)" : "var(--fg)",
          }}
        >
          {loading ? "—" : <CountUp value={value} format={fmtCompact} />}
        </div>
        <div className="mt-2 text-[12.5px]" style={{ color: "var(--fg-subtle)" }}>{foot}</div>
      </div>
    </HoverTile>
  );
}

function VisBars({ vis, total, loading }: { vis: Record<string, number>; total: number; loading?: boolean }) {
  const t = useT();
  const rows = [
    { key: "global", label: t("vis.global"), tone: "var(--info)" },
    { key: "team", label: t("vis.team"), tone: "var(--fg)" },
    { key: "private", label: t("vis.private"), tone: "var(--warn)" },
  ];
  return (
    <div className="flex flex-col gap-4">
      {rows.map((r) => {
        const n = vis[r.key] ?? 0;
        const pct = total > 0 ? Math.round((n / total) * 100) : 0;
        return (
          <div key={r.key}>
            <div className="flex items-baseline justify-between mb-1.5">
              <span className="text-[13px] font-medium">{r.label}</span>
              <span className="font-mono text-[12px]" style={{ color: "var(--fg-muted)" }}>
                {loading ? "—" : n}
              </span>
            </div>
            <div className="meter">
              <span style={{ width: loading ? "0%" : `${pct}%`, background: r.tone }} />
            </div>
          </div>
        );
      })}
    </div>
  );
}

function FeaturedSkill({ skill }: { skill: Skill }) {
  const t = useT();
  const m = skill.manifest ?? {};
  const version = m.version ? `v${String(m.version)}` : null;
  return (
    <Link
      to="/skills/$id"
      params={{ id: skill.id }}
      className="group flex flex-col flex-1"
      style={{ cursor: "pointer" }}
    >
      <div className="flex items-baseline gap-2 mb-2">
        <span className="text-[12.5px] font-mono" style={{ color: "var(--fg-muted)" }}>
          {skill.namespace_slug}<span style={{ color: "var(--fg-faint)" }}>/</span>{skill.slug}
        </span>
        {version && <span className="text-[11.5px] font-mono" style={{ color: "var(--fg-faint)" }}>{version}</span>}
      </div>
      <h3 className="text-[24px] font-semibold tracking-tight transition-colors group-hover:text-[var(--accent)]">
        {skill.display_name}
      </h3>
      {skill.description && (
        <p className="mt-2 text-[14.5px] leading-[1.55] max-w-xl" style={{ color: "var(--fg-muted)" }}>
          {skill.description}
        </p>
      )}
      <div className="mt-auto pt-5 flex items-center justify-between">
        <div className="flex flex-wrap items-center gap-2">
          {skill.tags.slice(0, 3).map((t) => <Tag key={t}>{t}</Tag>)}
        </div>
        <span className="text-[13px] font-medium" style={{ color: "var(--fg)" }}>
          {fmtCompact(skill.install_count)}{" "}
          <span style={{ color: "var(--fg-subtle)" }}>{t("dash.installs")}</span>
        </span>
      </div>
    </Link>
  );
}

function fmtCompact(n: number): string {
  const v = Math.round(n);
  if (v >= 1_000_000) return `${(v / 1_000_000).toFixed(1)}M`;
  if (v >= 1_000) return `${(v / 1_000).toFixed(1)}K`;
  return v.toLocaleString();
}
