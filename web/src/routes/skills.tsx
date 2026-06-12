import { Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { Search, Plus, Loader2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { PageHeader } from "@/components/ui/PageHeader";
import { Badge, Tag } from "@/components/ui/Badge";
import { listSkills, searchSkills, type Skill } from "@/lib/api";
import { useT } from "@/i18n";
import type { TKey } from "@/i18n/dict";

/** Debounce a fast-changing value so we don't hit /search on every keystroke. */
function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setV(value), ms);
    return () => clearTimeout(id);
  }, [value, ms]);
  return v;
}

type Vis = "all" | "private" | "team" | "global";

const VIS_LABEL: Record<Vis, TKey> = {
  all: "skills.filter.all",
  private: "skills.filter.private",
  team: "skills.filter.team",
  global: "skills.filter.global",
};

export function SkillsListPage() {
  const t = useT();
  const [query, setQuery] = useState("");
  const [vis, setVis] = useState<Vis>("all");
  const debounced = useDebounced(query.trim(), 250);
  const searching = debounced.length > 0;

  // Base listing for browse mode; server-side FTS when there's a query.
  const base = useQuery({ queryKey: ["skills"], queryFn: listSkills, enabled: !searching });
  const search = useQuery({
    queryKey: ["search", debounced],
    queryFn: () => searchSkills(debounced),
    enabled: searching,
  });
  const q = searching ? search : base;

  const filtered = useMemo<Skill[]>(() => {
    const rows = (q.data ?? []) as Skill[];
    return rows.filter((r) => (vis === "all" ? true : r.visibility === vis));
  }, [q.data, vis]);

  return (
    <>
      <PageHeader
        eyebrow={t("skills.eyebrow")}
        title={
          <>
            {t("skills.titleLead")}<span className="serif-em">{t("skills.titleEm")}</span>
          </>
        }
        description={t("skills.desc")}
        actions={
          <Link to="/skills/new" className="btn btn-primary">
            <Plus size={15} /> {t("skills.new")}
          </Link>
        }
      />

      {/* Filters */}
      <div className="flex flex-col sm:flex-row gap-3 mb-8">
        <div className="relative flex-1">
          <Search size={14} className="absolute left-4 top-1/2 -translate-y-1/2" style={{ color: "var(--fg-faint)" }} />
          <input
            className="input"
            style={{ paddingLeft: 38, borderRadius: 999 }}
            placeholder={t("skills.search")}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
        <div className="inline-flex p-1 rounded-full self-start sm:self-auto" style={{ background: "var(--bg-2)" }}>
          {(["all", "private", "team", "global"] as Vis[]).map((v) => (
            <button
              key={v}
              onClick={() => setVis(v)}
              className="px-3.5 h-8 text-[13px] font-medium rounded-full transition-colors"
              style={
                vis === v
                  ? { background: "var(--surface)", color: "var(--fg)", cursor: "pointer" }
                  : { color: "var(--fg-muted)", cursor: "pointer" }
              }
            >
              {t(VIS_LABEL[v])}
            </button>
          ))}
        </div>
      </div>

      {q.isLoading && (
        <div className="py-20 flex justify-center">
          <Loader2 size={20} className="animate-spin" style={{ color: "var(--fg-muted)" }} />
        </div>
      )}
      {q.error && (
        <div className="py-6 px-5 rounded-2xl" style={{ background: "var(--bad-soft)", color: "var(--bad)" }}>
          <div className="font-mono text-[13px]">{(q.error as Error).message}</div>
        </div>
      )}

      {!q.isLoading && filtered.length === 0 && (
        <div className="py-20 text-center">
          <div className="text-[16px] font-medium mb-1">{t("skills.empty.title")}</div>
          <p className="text-[13px]" style={{ color: "var(--fg-muted)" }}>{t("skills.empty.body")}</p>
        </div>
      )}

      {/* Stacked list — magazine table of contents */}
      <ul>
        {filtered.map((s, i) => (
          <li
            key={s.id}
            style={{ borderTop: i === 0 ? "1px solid var(--border)" : "0", borderBottom: "1px solid var(--border)" }}
          >
            <SkillRow skill={s} />
          </li>
        ))}
      </ul>
    </>
  );
}

function SkillRow({ skill }: { skill: Skill }) {
  const t = useT();
  const m = skill.manifest ?? {};
  const version = m.version ? `v${String(m.version)}` : null;
  const category = m.category ? String(m.category) : null;

  return (
    <Link
      to="/skills/$namespace/$slug"
      params={{ namespace: skill.namespace_slug, slug: skill.slug }}
      className="group flex items-start gap-6 py-7 -mx-2 px-2 rounded-lg transition-colors"
      style={{ cursor: "pointer" }}
    >
      <div className="flex-1 min-w-0">
        <div className="flex items-baseline gap-2 mb-1.5">
          <span className="text-[12.5px] font-mono" style={{ color: "var(--fg-muted)" }}>
            {skill.namespace_slug}<span style={{ color: "var(--fg-faint)" }}>/</span>{skill.slug}
          </span>
          {version && (
            <span className="text-[11.5px] font-mono" style={{ color: "var(--fg-faint)" }}>{version}</span>
          )}
          {m.deprecated ? (
            <Badge tone="bad">{t("skills.deprecated")}</Badge>
          ) : null}
        </div>
        <h3 className="text-[20px] font-semibold tracking-tight group-hover:text-[var(--accent)] transition-colors">
          {skill.display_name}
        </h3>
        {skill.description && (
          <p className="mt-2 text-[14.5px] leading-[1.55] max-w-2xl" style={{ color: "var(--fg-muted)" }}>
            {skill.description}
          </p>
        )}
        <div className="mt-3 flex flex-wrap items-center gap-2 text-[12.5px]" style={{ color: "var(--fg-subtle)" }}>
          {category && <Tag>{category}</Tag>}
          {skill.tags.slice(0, 4).map((t) => (
            <Tag key={t}>{t}</Tag>
          ))}
          {skill.tags.length > 4 && <span style={{ color: "var(--fg-faint)" }}>+{skill.tags.length - 4}</span>}
        </div>
      </div>

      <div className="text-right flex-shrink-0 hidden sm:flex sm:flex-col sm:items-end gap-1">
        <Badge tone={skill.visibility === "global" ? "info" : skill.visibility === "team" ? "default" : "warn"}>
          {t(VIS_LABEL[skill.visibility])}
        </Badge>
        <span className="text-[12.5px]" style={{ color: "var(--fg-muted)" }}>
          {t("skills.installs", { count: formatNum(skill.install_count) })}
        </span>
      </div>
    </Link>
  );
}

function formatNum(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toLocaleString();
}
