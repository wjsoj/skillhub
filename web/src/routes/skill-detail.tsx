import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import * as Tabs from "@radix-ui/react-tabs";
import { useParams, Link } from "@tanstack/react-router";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  ArrowLeft, Plus, GitMerge, MessageSquare, Play, Send, Loader2, ThumbsUp,
  Copy, Check, Github, FileCode, FileText, FolderTree, Package, AlertTriangle,
  Star, Download, Calendar, ExternalLink, Folder, Hash, Tag as TagIcon,
} from "lucide-react";
import { PageHeader } from "@/components/ui/PageHeader";
import { Card } from "@/components/ui/Card";
import { Badge, Tag } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Stepper } from "@/components/ui/Stepper";
import {
  addCollaborator,
  createDraft,
  decideProposal,
  getSkill,
  listCollaborators,
  listIterations,
  listProposals,
  openIteration,
  openProposal,
  reviewProposal,
  runIterationTests,
  submitIteration,
  listVersions,
  publishVersion,
  getStarStatus,
  addStar,
  removeStar,
  type Proposal,
  type IterationJob,
  type Skill,
} from "@/lib/api";
import { useT, useI18n, type Locale } from "@/i18n";
import type { TKey } from "@/i18n/dict";

type TFn = ReturnType<typeof useT>;

const PROPOSAL_NODES: { key: string; labelKey: TKey }[] = [
  { key: "open", labelKey: "detail.prop.node.open" },
  { key: "changes_requested", labelKey: "detail.prop.node.changes" },
  { key: "approved", labelKey: "detail.prop.node.approved" },
  { key: "merged", labelKey: "detail.prop.node.merged" },
];

const ITER_NODES: { key: string; labelKey: TKey }[] = [
  { key: "queued", labelKey: "detail.iter.node.queued" },
  { key: "running", labelKey: "detail.iter.node.running" },
  { key: "succeeded", labelKey: "detail.iter.node.succeeded" },
  { key: "submitted", labelKey: "detail.iter.node.submitted" },
];

const trNodes = (nodes: { key: string; labelKey: TKey }[], t: TFn) =>
  nodes.map((n) => ({ key: n.key, label: t(n.labelKey) }));

export function SkillDetailPage() {
  const t = useT();
  const { id } = useParams({ from: "/skills/$id" });
  const q = useQuery({ queryKey: ["skill", id], queryFn: () => getSkill(id) });

  return (
    <>
      <div className="mb-4">
        <Link to="/skills">
          <Button variant="ghost" size="sm"><ArrowLeft size={14} /> {t("detail.back")}</Button>
        </Link>
      </div>

      {q.isLoading && (
        <Card className="p-10 text-center">
          <Loader2 size={20} className="animate-spin mx-auto" style={{ color: "var(--fg-muted)" }} />
        </Card>
      )}
      {q.error && (
        <Card className="p-6 border-l-4" style={{ borderLeftColor: "var(--bad)" }}>
          <div className="font-mono text-[12.5px]" style={{ color: "var(--bad)" }}>
            {(q.error as Error).message}
          </div>
        </Card>
      )}

      {q.data && <SkillBody skill={q.data} />}
    </>
  );
}

function SkillBody({ skill }: { skill: Skill }) {
  const t = useT();
  const m = skill.manifest ?? {};
  const visKey: TKey = skill.visibility === "global" ? "skills.filter.global" : skill.visibility === "team" ? "skills.filter.team" : "skills.filter.private";
  return (
    <>
      <PageHeader
        eyebrow={
          <span className="font-mono">
            {skill.namespace_slug} <span style={{ color: "var(--fg-faint)" }}>/</span> {skill.slug}
          </span>
        }
        title={
          <span className="inline-flex items-center gap-3 flex-wrap">
            {skill.display_name}
            {m.version && (
              <Badge tone="default" className="text-[11.5px] font-mono">v{String(m.version)}</Badge>
            )}
          </span>
        }
        description={skill.description ?? undefined}
        actions={
          <div className="flex items-center gap-2 flex-wrap">
            <Badge tone={skill.visibility === "global" ? "info" : skill.visibility === "team" ? "default" : "warn"}>
              {t(visKey)}
            </Badge>
            {m.deprecated && (
              <Badge tone="bad"><AlertTriangle size={11} /> {t("detail.deprecated")}</Badge>
            )}
          </div>
        }
      />

      <Tabs.Root defaultValue="overview" className="space-y-6">
        <Tabs.List className="tabs-list">
          <Tabs.Trigger value="overview" className="tab">{t("detail.tab.overview")}</Tabs.Trigger>
          <Tabs.Trigger value="versions" className="tab">{t("detail.tab.versions")}</Tabs.Trigger>
          <Tabs.Trigger value="proposals" className="tab">{t("detail.tab.proposals")}</Tabs.Trigger>
          <Tabs.Trigger value="collaborators" className="tab">{t("detail.tab.collaborators")}</Tabs.Trigger>
          <Tabs.Trigger value="iterations" className="tab">{t("detail.tab.iterations")}</Tabs.Trigger>
        </Tabs.List>

        <Tabs.Content value="overview">
          <OverviewTab skill={skill} />
        </Tabs.Content>
        <Tabs.Content value="versions">
          <VersionsTab skillId={skill.id} />
        </Tabs.Content>
        <Tabs.Content value="proposals">
          <ProposalsTab skillId={skill.id} />
        </Tabs.Content>
        <Tabs.Content value="collaborators">
          <CollaboratorsTab skillId={skill.id} />
        </Tabs.Content>
        <Tabs.Content value="iterations">
          <IterationsTab skillId={skill.id} />
        </Tabs.Content>
      </Tabs.Root>
    </>
  );
}

/* ───────── Overview ───────── */

function OverviewTab({ skill }: { skill: Skill }) {
  const t = useT();
  const { locale } = useI18n();
  const m = skill.manifest ?? {};
  const inputs = Array.isArray(m.inputs) ? m.inputs : [];
  const outputs = Array.isArray(m.outputs) ? m.outputs : [];
  const files = Array.isArray(m.files) ? m.files : [];
  const deps = Array.isArray(m.dependencies) ? m.dependencies : [];

  return (
    <div className="grid grid-cols-1 lg:grid-cols-[1fr_320px] gap-6 items-start">
      {/* Main column */}
      <div className="space-y-6 min-w-0">
        {/* Install command — built from the current origin so it works against
            whatever host is serving this page (clawhub-compatible registry). */}
        <InstallCard slug={skill.slug} />

        {m.deprecated && m.deprecation_note && (
          <Card className="p-4 flex items-start gap-3" style={{ borderLeft: "3px solid var(--bad)" }}>
            <AlertTriangle size={16} style={{ color: "var(--bad)", flexShrink: 0, marginTop: 2 }} />
            <div>
              <div className="text-[13.5px] font-semibold">{t("detail.deprecatedTitle")}</div>
              <div className="text-[13px] mt-0.5" style={{ color: "var(--fg-muted)" }}>
                {String(m.deprecation_note)}
              </div>
            </div>
          </Card>
        )}

        {/* Tags */}
        {skill.tags.length > 0 && (
          <div className="flex flex-wrap gap-2">
            {skill.tags.map((t) => (
              <Badge key={t} tone="default" className="text-[11.5px]"><Hash size={10} /> {t}</Badge>
            ))}
          </div>
        )}

        {/* README */}
        {skill.readme && (
          <Card className="overflow-hidden">
            <div className="px-5 py-3 flex items-center gap-2" style={{ borderBottom: "1px solid var(--border)" }}>
              <FileText size={14} />
              <span className="text-[13.5px] font-semibold">{t("detail.skillMd")}</span>
            </div>
            <div className="px-5 py-4">
              <Markdown content={skill.readme} />
            </div>
          </Card>
        )}

        {/* Inputs / Outputs */}
        {(inputs.length > 0 || outputs.length > 0) && (
          <Card className="overflow-hidden">
            <div className="px-5 py-3 flex items-center gap-2" style={{ borderBottom: "1px solid var(--border)" }}>
              <Package size={14} />
              <span className="text-[13.5px] font-semibold">{t("detail.contract")}</span>
            </div>
            <div className="grid md:grid-cols-2 gap-px" style={{ background: "var(--border)" }}>
              <SignatureBlock title={t("detail.inputs")} rows={inputs as ContractRow[]} />
              <SignatureBlock title={t("detail.outputs")} rows={outputs as ContractRow[]} />
            </div>
          </Card>
        )}

        {/* Files */}
        {files.length > 0 && (
          <Card className="overflow-hidden">
            <div className="px-5 py-3 flex items-center gap-2" style={{ borderBottom: "1px solid var(--border)" }}>
              <FolderTree size={14} />
              <span className="text-[13.5px] font-semibold">{t("detail.files")}</span>
              <span className="font-mono text-[11.5px] ml-1" style={{ color: "var(--fg-faint)" }}>
                {files.length}
              </span>
            </div>
            <ul className="divide-y" style={{ borderColor: "var(--border)" }}>
              {(files as FileEntry[]).map((f) => (
                <li key={f.path} className="px-5 py-2 flex items-center gap-3 text-[13px]">
                  <FileKindIcon kind={f.kind} />
                  <span className="font-mono flex-1 truncate">{f.path}</span>
                  {f.size != null && (
                    <span className="font-mono text-[11.5px]" style={{ color: "var(--fg-faint)" }}>
                      {humanBytes(f.size)}
                    </span>
                  )}
                </li>
              ))}
            </ul>
          </Card>
        )}

        {/* Raw manifest */}
        <Card className="overflow-hidden">
          <div className="px-5 py-3 flex items-center gap-2" style={{ borderBottom: "1px solid var(--border)" }}>
            <FileCode size={14} />
            <span className="text-[13.5px] font-semibold">{t("detail.rawManifest")}</span>
          </div>
          <pre
            className="px-5 py-4 text-[12px] font-mono overflow-x-auto"
            style={{ color: "var(--fg-muted)" }}
          >
{JSON.stringify(skill.manifest, null, 2)}
          </pre>
        </Card>
      </div>

      {/* Sidebar */}
      <div className="space-y-4 lg:sticky lg:top-20 self-start">
        <StarButton skillId={skill.id} initialStars={skill.stars} />
        <Card className="p-5 space-y-4">
          <Metric label={t("detail.metric.installs")} value={formatNum(skill.install_count)} icon={Download} />
          <Metric label={t("detail.metric.stars")} value={formatNum(skill.stars)} icon={Star} />
          <Metric label={t("detail.metric.firstSeen")} value={formatDate(skill.created_at, locale)} icon={Calendar} />
          <Metric label={t("detail.metric.updated")} value={formatDate(skill.updated_at, locale)} icon={Calendar} />
        </Card>

        {(m.author || m.license || m.category || m.runtime || deps.length > 0) && (
          <Card className="p-5 space-y-3">
            {m.author && <SideRow label={t("detail.side.author")} value={<span className="font-mono">{String(m.author)}</span>} />}
            {m.license && <SideRow label={t("detail.side.license")} value={<span className="font-mono">{String(m.license)}</span>} />}
            {m.category && <SideRow label={t("detail.side.category")} value={<Badge tone="accent">{String(m.category)}</Badge>} />}
            {m.entrypoint && <SideRow label={t("detail.side.entrypoint")} value={<code className="text-[12px] font-mono">{String(m.entrypoint)}</code>} />}
            {m.runtime && Object.keys(m.runtime).length > 0 && (
              <SideRow
                label={t("detail.side.runtime")}
                value={
                  <div className="flex flex-wrap gap-1">
                    {Object.entries(m.runtime).map(([k, v]) => (
                      <Tag key={k}>{k} {String(v)}</Tag>
                    ))}
                  </div>
                }
              />
            )}
            {deps.length > 0 && (
              <SideRow
                label={t("detail.side.dependencies")}
                value={
                  <ul className="space-y-0.5">
                    {(deps as string[]).map((d) => (
                      <li key={d} className="font-mono text-[12px]" style={{ color: "var(--fg)" }}>
                        {d}
                      </li>
                    ))}
                  </ul>
                }
              />
            )}
          </Card>
        )}

        {skill.repository_url && (
          <Card className="p-5">
            <div className="text-[11.5px] font-medium uppercase tracking-wider mb-2" style={{ color: "var(--fg-subtle)" }}>
              {t("detail.side.repository")}
            </div>
            <a
              href={skill.repository_url}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-2 text-[13.5px] font-medium hover:underline"
              style={{ color: "var(--accent)" }}
            >
              <Github size={14} />
              <span className="font-mono truncate">{shortRepo(skill.repository_url)}</span>
              <ExternalLink size={12} />
            </a>
          </Card>
        )}
      </div>
    </div>
  );
}

function InstallCard({ slug }: { slug: string }) {
  const t = useT();
  const [copied, setCopied] = useState(false);
  // Build the install command against whatever host is serving this page, so
  // it's correct on localhost, a staging box, or production without hardcoding.
  const registry = `${typeof window !== "undefined" ? window.location.origin : ""}/clawhub`;
  const cmd = `clawhub --registry ${registry} install ${slug}`;
  const copy = () => {
    navigator.clipboard.writeText(cmd).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  };
  return (
    <Card className="overflow-hidden">
      <div className="px-5 py-3 flex items-center gap-2" style={{ borderBottom: "1px solid var(--border)" }}>
        <Download size={14} />
        <span className="text-[13.5px] font-semibold">{t("detail.installation")}</span>
      </div>
      <div className="px-5 py-3 flex items-center gap-3" style={{ background: "var(--bg-muted)" }}>
        <span className="font-mono text-[13px] flex-1 truncate" style={{ color: "var(--fg-muted)" }}>
          <span style={{ color: "var(--fg-faint)" }}>$ </span>
          <span style={{ color: "var(--fg)" }}>{cmd}</span>
        </span>
        <button
          onClick={copy}
          className="btn btn-secondary btn-sm cursor-pointer"
          aria-label={t("detail.copy")}
        >
          {copied ? <><Check size={13} /> {t("detail.copied")}</> : <><Copy size={13} /> {t("detail.copy")}</>}
        </button>
      </div>
    </Card>
  );
}

interface ContractRow {
  name: string;
  type: string;
  required?: boolean;
  default?: unknown;
  description?: string;
}

function SignatureBlock({ title, rows }: { title: string; rows: ContractRow[] }) {
  const t = useT();
  return (
    <div className="p-5" style={{ background: "var(--surface)" }}>
      <div className="text-[11.5px] font-medium uppercase tracking-wider mb-3" style={{ color: "var(--fg-subtle)" }}>
        {title}
      </div>
      {rows.length === 0 ? (
        <div className="text-[12.5px]" style={{ color: "var(--fg-faint)" }}>{t("common.none")}</div>
      ) : (
        <ul className="space-y-3">
          {rows.map((r) => (
            <li key={r.name} className="text-[12.5px]">
              <div className="flex items-baseline gap-2 mb-0.5">
                <code className="font-mono font-semibold">{r.name}</code>
                <span className="font-mono" style={{ color: "var(--fg-muted)" }}>{r.type}</span>
                {r.required && <Badge tone="warn" className="text-[10px]">{t("detail.required")}</Badge>}
                {r.default !== undefined && r.default !== null && (
                  <span className="font-mono" style={{ color: "var(--fg-faint)" }}>
                    = {JSON.stringify(r.default)}
                  </span>
                )}
              </div>
              {r.description && (
                <div style={{ color: "var(--fg-muted)" }}>{r.description}</div>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

interface FileEntry {
  path: string;
  size: number | null;
  kind: string;
}

function FileKindIcon({ kind }: { kind: string }) {
  if (kind === "dir") return <Folder size={13} style={{ color: "var(--accent)" }} />;
  if (kind === "doc") return <FileText size={13} style={{ color: "var(--fg-muted)" }} />;
  if (kind === "code") return <FileCode size={13} style={{ color: "var(--ok)" }} />;
  if (kind === "test") return <FileCode size={13} style={{ color: "var(--warn)" }} />;
  if (kind === "config") return <FileCode size={13} style={{ color: "var(--info)" }} />;
  return <FileText size={13} style={{ color: "var(--fg-faint)" }} />;
}

function Metric({
  label, value, icon: Icon,
}: {
  label: string;
  value: string;
  icon: React.ComponentType<{ size?: number }>;
}) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-[12.5px] flex items-center gap-1.5" style={{ color: "var(--fg-muted)" }}>
        <Icon size={13} /> {label}
      </span>
      <span className="text-[14px] font-semibold tabular-nums">{value}</span>
    </div>
  );
}

function SideRow({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div>
      <div className="text-[11.5px] font-medium uppercase tracking-wider mb-1" style={{ color: "var(--fg-subtle)" }}>
        {label}
      </div>
      <div className="text-[13px]">{value}</div>
    </div>
  );
}

function Markdown({ content }: { content: string }) {
  return (
    <div className="markdown text-[14px] leading-[1.65]">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          h1: ({ children }) => <h1 className="text-[22px] font-bold tracking-tight mt-2 mb-3">{children}</h1>,
          h2: ({ children }) => <h2 className="text-[18px] font-semibold tracking-tight mt-6 mb-2">{children}</h2>,
          h3: ({ children }) => <h3 className="text-[15.5px] font-semibold tracking-tight mt-5 mb-1.5">{children}</h3>,
          p:  ({ children }) => <p className="my-2.5" style={{ color: "var(--fg)" }}>{children}</p>,
          ul: ({ children }) => <ul className="list-disc pl-5 my-2 space-y-1">{children}</ul>,
          ol: ({ children }) => <ol className="list-decimal pl-5 my-2 space-y-1">{children}</ol>,
          li: ({ children }) => <li>{children}</li>,
          a:  ({ href, children }) => (
            <a href={href} target="_blank" rel="noopener noreferrer" className="hover:underline" style={{ color: "var(--accent)" }}>
              {children}
            </a>
          ),
          blockquote: ({ children }) => (
            <blockquote
              className="my-3 pl-4 py-1 text-[13.5px]"
              style={{ borderLeft: "3px solid var(--border-strong)", color: "var(--fg-muted)" }}
            >
              {children}
            </blockquote>
          ),
          code: ({ className, children, ...props }: React.HTMLAttributes<HTMLElement>) => {
            const isBlock = (className ?? "").startsWith("language-");
            if (isBlock) {
              return (
                <code {...props} className="block font-mono text-[12.5px]">
                  {children}
                </code>
              );
            }
            return (
              <code
                {...props}
                className="font-mono text-[12.5px] px-1 py-[1px] rounded"
                style={{ background: "var(--bg-muted)", color: "var(--fg)" }}
              >
                {children}
              </code>
            );
          },
          pre: ({ children }) => (
            <pre
              className="my-3 p-3 rounded-md overflow-x-auto text-[12.5px] font-mono"
              style={{ background: "var(--bg-muted)", color: "var(--fg)" }}
            >
              {children}
            </pre>
          ),
          table: ({ children }) => (
            <div className="my-3 overflow-x-auto">
              <table className="w-full text-[12.5px] border" style={{ borderColor: "var(--border)" }}>
                {children}
              </table>
            </div>
          ),
          th: ({ children }) => (
            <th className="text-left px-3 py-1.5 border-b font-semibold" style={{ borderColor: "var(--border)", background: "var(--bg-muted)" }}>
              {children}
            </th>
          ),
          td: ({ children }) => (
            <td className="px-3 py-1.5 border-b" style={{ borderColor: "var(--border)" }}>
              {children}
            </td>
          ),
          hr: () => <hr className="my-4" style={{ borderColor: "var(--border)" }} />,
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}

function humanBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

function formatNum(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000)     return `${(n / 1_000).toFixed(1)}K`;
  return n.toLocaleString();
}

function formatDate(iso: string, locale: Locale): string {
  try {
    const tag = locale === "zh" ? "zh-CN" : "en-US";
    return new Date(iso).toLocaleDateString(tag, { month: "short", day: "numeric", year: "numeric" });
  } catch { return iso; }
}

function shortRepo(url: string): string {
  return url.replace(/^https?:\/\/[^/]+\//, "").replace(/\.git$/, "");
}

/* ───────── Star button ───────── */
function StarButton({ skillId, initialStars }: { skillId: string; initialStars: number }) {
  const t = useT();
  const qc = useQueryClient();
  const status = useQuery({ queryKey: ["star", skillId], queryFn: () => getStarStatus(skillId) });
  const toggle = useMutation({
    mutationFn: () => (status.data?.starred ? removeStar(skillId) : addStar(skillId)),
    onSuccess: (s) => qc.setQueryData(["star", skillId], s),
  });
  const starred = status.data?.starred ?? false;
  const count = status.data?.stars ?? initialStars;
  return (
    <button
      onClick={() => toggle.mutate()}
      disabled={toggle.isPending}
      className="btn btn-secondary w-full"
      data-testid="star-button"
      style={starred ? { borderColor: "var(--accent)", color: "var(--accent)" } : undefined}
    >
      <Star size={14} fill={starred ? "var(--accent)" : "none"} />
      {starred ? t("detail.starred") : t("detail.star")}
      <span className="font-mono text-[12px]" style={{ color: "var(--fg-muted)" }}>{count}</span>
    </button>
  );
}

/* ───────── Versions ───────── */
function VersionsTab({ skillId }: { skillId: string }) {
  const t = useT();
  const qc = useQueryClient();
  const q = useQuery({ queryKey: ["versions", skillId], queryFn: () => listVersions(skillId) });
  const [version, setVersion] = useState("");

  const publish = useMutation({
    mutationFn: () => publishVersion(skillId, { version: version.trim() }),
    onSuccess: () => {
      setVersion("");
      qc.invalidateQueries({ queryKey: ["versions", skillId] });
    },
  });

  return (
    <div className="space-y-6">
      <Card className="p-5">
        <div className="text-[14.5px] font-semibold mb-3">{t("detail.versions.publish")}</div>
        <form
          className="flex flex-col sm:flex-row gap-2 items-stretch sm:items-end"
          onSubmit={(e) => { e.preventDefault(); if (version.trim()) publish.mutate(); }}
        >
          <label className="flex-1">
            <div className="text-[12.5px] font-medium mb-1.5">{t("detail.versions.version")}</div>
            <input className="input input-mono" value={version} onChange={(e) => setVersion(e.target.value)} placeholder="1.0.0" data-testid="version-input" />
          </label>
          <Button disabled={!version.trim() || publish.isPending} data-testid="version-publish">
            {publish.isPending ? <><Loader2 size={14} className="animate-spin" /> {t("detail.versions.publishing")}</> : <><Plus size={14} /> {t("detail.versions.publishBtn")}</>}
          </Button>
        </form>
        {publish.error && (
          <div className="mt-3 px-3 py-2 rounded-lg text-[12.5px] font-mono" style={{ background: "var(--bad-soft)", color: "var(--bad)" }} data-testid="version-error">
            {(publish.error as Error).message}
          </div>
        )}
      </Card>

      <div>
        <div className="text-[14.5px] font-semibold mb-3">{t("detail.versions.title")}</div>
        {q.isLoading && <Loader2 size={18} className="animate-spin" style={{ color: "var(--fg-muted)" }} />}
        {!q.isLoading && (q.data?.length ?? 0) === 0 && (
          <Card className="p-6 text-center" style={{ color: "var(--fg-muted)" }}>
            {t("detail.versions.none")}
          </Card>
        )}
        <ul data-testid="version-list">
          {q.data?.map((v, i) => (
            <li
              key={v.id}
              className="py-4 flex items-start justify-between gap-3"
              style={{ borderTop: i === 0 ? "1px solid var(--border)" : "0", borderBottom: "1px solid var(--border)" }}
              data-testid="version-row"
            >
              <div className="min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <span className="text-[15px] font-semibold font-mono">v{v.version}</span>
                  <Badge tone={v.status === "approved" ? "ok" : v.status === "yanked" ? "bad" : "default"}>{v.status}</Badge>
                  {v.tags.map((tag) => <Tag key={tag}>{tag}</Tag>)}
                </div>
                <div className="text-[12px] font-mono" style={{ color: "var(--fg-faint)" }}>
                  {v.checksum_sha256.slice(0, 16)}… · {new Date(v.published_at).toLocaleDateString()}
                </div>
              </div>
              <TagIcon size={14} style={{ color: "var(--fg-faint)", flexShrink: 0, marginTop: 4 }} />
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

/* ───────── Proposals ───────── */
function ProposalsTab({ skillId }: { skillId: string }) {
  const t = useT();
  const qc = useQueryClient();
  const q = useQuery({ queryKey: ["proposals", skillId], queryFn: () => listProposals(skillId) });

  const [title, setTitle] = useState("");
  const [summary, setSummary] = useState("");
  const [target, setTarget] = useState("0.2.0");

  const open = useMutation({
    mutationFn: async () => {
      const d = await createDraft(skillId, { target_version: target, manifest: { source: "ui" }, summary });
      return openProposal(skillId, { draft_id: d.draft_id, title, body: summary });
    },
    onSuccess: () => {
      setTitle("");
      setSummary("");
      qc.invalidateQueries({ queryKey: ["proposals", skillId] });
    },
  });

  return (
    <div className="space-y-6">
      <Card className="p-5">
        <div className="text-[14.5px] font-semibold mb-1">{t("detail.prop.pipeline")}</div>
        <p className="text-[12.5px] mb-4" style={{ color: "var(--fg-muted)" }}>
          {t("detail.prop.pipelineDescPre")} <span className="font-mono">SkillVersion</span>.
        </p>
        <div className="rounded-md p-4" style={{ background: "var(--bg-muted)" }}>
          <Stepper nodes={trNodes(PROPOSAL_NODES, t)} doneKeys={["open"]} activeKey="open" />
        </div>
      </Card>

      <div className="space-y-3" data-testid="proposal-list">
        {q.data?.length === 0 && (
          <Card className="p-6 text-center" style={{ color: "var(--fg-muted)" }}>
            {t("detail.prop.none")}
          </Card>
        )}
        {q.data?.map((p) => <ProposalRow key={p.id} p={p} skillId={skillId} />)}
      </div>

      <Card className="p-5">
        <div className="text-[14.5px] font-semibold mb-3">{t("detail.prop.openNew")}</div>
        <form className="grid grid-cols-1 md:grid-cols-[2fr_1fr] gap-3" onSubmit={(e) => { e.preventDefault(); open.mutate(); }}>
          <div className="space-y-3">
            <input
              className="input"
              placeholder={t("detail.prop.ph.title")}
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              data-testid="proposal-title"
            />
            <textarea
              className="textarea"
              rows={3}
              placeholder={t("detail.prop.ph.summary")}
              value={summary}
              onChange={(e) => setSummary(e.target.value)}
              data-testid="proposal-summary"
            />
          </div>
          <div className="flex flex-col gap-2">
            <label className="text-[12.5px] font-medium">{t("detail.prop.targetVersion")}</label>
            <input className="input input-mono" value={target} onChange={(e) => setTarget(e.target.value)} />
            <Button className="mt-auto" disabled={!title || open.isPending} data-testid="proposal-submit">
              {open.isPending ? <><Loader2 size={14} className="animate-spin" /> {t("detail.prop.opening")}</> : <><Plus size={14} /> {t("detail.prop.openProposal")}</>}
            </Button>
          </div>
        </form>
      </Card>
    </div>
  );
}

const PROPOSAL_STATE_LABEL: Record<Proposal["state"], TKey> = {
  open: "detail.prop.node.open",
  changes_requested: "detail.prop.node.changes",
  approved: "detail.prop.node.approved",
  rejected: "detail.prop.node.rejected",
  merged: "detail.prop.node.merged",
  withdrawn: "detail.prop.node.withdrawn",
};

function ProposalRow({ p, skillId }: { p: Proposal; skillId: string }) {
  const t = useT();
  const qc = useQueryClient();
  const decide = useMutation({
    mutationFn: (state: Proposal["state"]) => decideProposal(skillId, p.id, state),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["proposals", skillId] }),
  });
  const comment = useMutation({
    mutationFn: () => reviewProposal(skillId, p.id, { verdict: "comment", body: "LGTM" }),
  });

  const done = p.state === "merged"
    ? ["open", "approved", "merged"]
    : p.state === "approved"
      ? ["open", "approved"]
      : ["open"];

  return (
    <Card className="p-5" data-testid="proposal-row">
      <div className="flex flex-wrap items-start gap-4">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1.5">
            <Badge
              tone={
                p.state === "merged" ? "ok"
                : p.state === "approved" ? "accent"
                : p.state === "rejected" || p.state === "withdrawn" ? "bad"
                : "default"
              }
            >
              {t(PROPOSAL_STATE_LABEL[p.state])}
            </Badge>
            <span className="font-mono text-[11.5px]" style={{ color: "var(--fg-faint)" }}>
              #{p.id.slice(0, 8)}
            </span>
          </div>
          <div className="text-[15px] font-semibold tracking-tight">{p.title}</div>
          {p.body && (
            <p className="mt-1 text-[13px]" style={{ color: "var(--fg-muted)" }}>{p.body}</p>
          )}
          <div className="mt-4">
            <Stepper nodes={trNodes(PROPOSAL_NODES, t)} doneKeys={done} activeKey={p.state} />
          </div>
        </div>
        <div className="flex flex-row md:flex-col gap-2 w-full md:w-auto md:min-w-[140px]">
          <Button variant="secondary" size="sm" onClick={() => comment.mutate()} disabled={comment.isPending} data-testid="proposal-comment">
            <MessageSquare size={13} /> {t("detail.prop.comment")}
          </Button>
          <Button variant="secondary" size="sm" onClick={() => decide.mutate("approved")} disabled={p.state !== "open" && p.state !== "changes_requested"} data-testid="proposal-approve">
            <ThumbsUp size={13} /> {t("detail.prop.approve")}
          </Button>
          <Button size="sm" onClick={() => decide.mutate("merged")} disabled={p.state !== "approved"} data-testid="proposal-merge">
            <GitMerge size={13} /> {t("detail.prop.merge")}
          </Button>
        </div>
      </div>
    </Card>
  );
}

/* ───────── Collaborators ───────── */
function CollaboratorsTab({ skillId }: { skillId: string }) {
  const t = useT();
  const qc = useQueryClient();
  const q = useQuery({ queryKey: ["collaborators", skillId], queryFn: () => listCollaborators(skillId) });

  const [userId, setUserId] = useState("");
  const [role, setRole] = useState<"maintainer" | "writer" | "reader">("writer");

  const add = useMutation({
    mutationFn: () => addCollaborator(skillId, userId, role),
    onSuccess: () => { setUserId(""); qc.invalidateQueries({ queryKey: ["collaborators", skillId] }); },
  });

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="px-5 py-4" style={{ borderBottom: "1px solid var(--border)" }}>
          <div className="text-[14.5px] font-semibold">{t("detail.collab.title")}</div>
          <p className="mt-0.5 text-[12.5px]" style={{ color: "var(--fg-muted)" }}>
            {t("detail.collab.desc")}
          </p>
        </div>
        <div className="overflow-x-auto">
          <table className="table">
            <thead>
              <tr><th>{t("detail.collab.colUser")}</th><th>{t("detail.collab.colRole")}</th><th>{t("detail.collab.colAdded")}</th></tr>
            </thead>
            <tbody data-testid="collab-tbody">
              {q.data?.length === 0 && (
                <tr><td colSpan={3} className="text-center py-10" style={{ color: "var(--fg-muted)" }}>
                  {t("detail.collab.none")}
                </td></tr>
              )}
              {q.data?.map((c) => (
                <tr key={c.user_id}>
                  <td className="font-mono text-[12.5px]">{c.user_id}</td>
                  <td>
                    <Badge tone={c.role === "maintainer" ? "accent" : c.role === "writer" ? "info" : "default"}>
                      {c.role}
                    </Badge>
                  </td>
                  <td className="font-mono text-[11.5px]" style={{ color: "var(--fg-muted)" }}>
                    {new Date(c.added_at).toISOString().slice(0, 16).replace("T", " ")}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      <Card className="p-5">
        <div className="text-[14.5px] font-semibold mb-3">{t("detail.collab.add")}</div>
        <form
          className="grid grid-cols-1 md:grid-cols-[2fr_1fr_auto] gap-3 items-end"
          onSubmit={(e) => { e.preventDefault(); if (userId) add.mutate(); }}
        >
          <label>
            <div className="text-[12.5px] font-medium mb-1.5">{t("detail.collab.userId")}</div>
            <input
              className="input input-mono"
              value={userId}
              onChange={(e) => setUserId(e.target.value)}
              placeholder="00000000-0000-0000-0000-000000000002"
              data-testid="collab-user"
            />
          </label>
          <label>
            <div className="text-[12.5px] font-medium mb-1.5">{t("detail.collab.role")}</div>
            <select className="select" value={role} onChange={(e) => setRole(e.target.value as typeof role)} data-testid="collab-role">
              <option value="reader">{t("detail.collab.role.reader")}</option>
              <option value="writer">{t("detail.collab.role.writer")}</option>
              <option value="maintainer">{t("detail.collab.role.maintainer")}</option>
            </select>
          </label>
          <Button disabled={!userId || add.isPending} data-testid="collab-add">
            <Plus size={14} /> {t("common.add")}
          </Button>
        </form>
      </Card>
    </div>
  );
}

/* ───────── Iterations ───────── */
function IterationsTab({ skillId }: { skillId: string }) {
  const t = useT();
  const qc = useQueryClient();
  const q = useQuery({ queryKey: ["iterations", skillId], queryFn: () => listIterations(skillId) });

  const [agent, setAgent] = useState("opus-4.7");
  const [intent, setIntent] = useState("");

  const open = useMutation({
    mutationFn: () => openIteration(skillId, { agent, intent }),
    onSuccess: () => { setIntent(""); qc.invalidateQueries({ queryKey: ["iterations", skillId] }); },
  });

  return (
    <div className="space-y-6">
      <Card className="p-5">
        <div className="text-[14.5px] font-semibold mb-1">{t("detail.iter.open")}</div>
        <p className="text-[12.5px] mb-4" style={{ color: "var(--fg-muted)" }}>
          {t("detail.iter.desc")}
        </p>
        <form
          className="grid grid-cols-1 md:grid-cols-[1fr_2fr_auto] gap-3 items-end"
          onSubmit={(e) => { e.preventDefault(); if (intent) open.mutate(); }}
        >
          <label>
            <div className="text-[12.5px] font-medium mb-1.5">{t("detail.iter.agent")}</div>
            <input className="input input-mono" value={agent} onChange={(e) => setAgent(e.target.value)} data-testid="iter-agent" />
          </label>
          <label>
            <div className="text-[12.5px] font-medium mb-1.5">{t("detail.iter.intent")}</div>
            <input
              className="input"
              value={intent}
              onChange={(e) => setIntent(e.target.value)}
              placeholder={t("detail.iter.ph.intent")}
              data-testid="iter-intent"
            />
          </label>
          <Button disabled={!intent || open.isPending} data-testid="iter-open">
            {open.isPending ? <><Loader2 size={14} className="animate-spin" /> {t("detail.iter.opening")}</> : <><Play size={14} /> {t("detail.iter.openJob")}</>}
          </Button>
        </form>
      </Card>

      <div className="space-y-3" data-testid="iter-list">
        {q.data?.length === 0 && (
          <Card className="p-6 text-center" style={{ color: "var(--fg-muted)" }}>
            {t("detail.iter.none")}
          </Card>
        )}
        {q.data?.map((j) => <IterationCard key={j.id} job={j} skillId={skillId} />)}
      </div>
    </div>
  );
}

const ITER_STATE_LABEL: Record<IterationJob["state"], TKey> = {
  queued: "detail.iter.node.queued",
  running: "detail.iter.node.running",
  succeeded: "detail.iter.node.succeeded",
  submitted: "detail.iter.node.submitted",
  failed: "detail.iter.node.failed",
  cancelled: "detail.iter.node.cancelled",
};

function IterationCard({ job, skillId }: { job: IterationJob; skillId: string }) {
  const t = useT();
  const qc = useQueryClient();
  const [cmd, setCmd] = useState("echo run");
  const [lastRun, setLastRun] = useState<null | {
    exit_code: number; duration_ms: number; timed_out: boolean; stdout: string; stderr: string;
  }>(null);
  const [submitTitle, setSubmitTitle] = useState(t("detail.iter.submitTitleDefault", { intent: job.intent.slice(0, 60) }));

  const runTests = useMutation({
    mutationFn: () => runIterationTests(skillId, job.id, cmd),
    onSuccess: setLastRun,
  });

  const submit = useMutation({
    mutationFn: () => submitIteration(skillId, job.id, {
      target_version: "0.2.0",
      summary: job.intent,
      title: submitTitle,
      body: `Run from iteration ${job.id}.`,
    }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["iterations", skillId] }),
  });

  const done = job.state === "succeeded" || job.state === "submitted"
    ? ["queued", "running", "succeeded"]
    : job.state === "running" ? ["queued", "running"] : ["queued"];

  return (
    <Card className="p-5">
      <div className="flex flex-wrap items-start justify-between gap-3 mb-4">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2 mb-1">
            <Badge tone={job.state === "submitted" ? "ok" : job.state === "failed" ? "bad" : job.state === "running" ? "accent" : "default"}>
              {t(ITER_STATE_LABEL[job.state])}
            </Badge>
            <Tag>{job.agent}</Tag>
            <span className="font-mono text-[11.5px]" style={{ color: "var(--fg-faint)" }}>
              #{job.id.slice(0, 8)}
            </span>
          </div>
          <div className="text-[14.5px] font-semibold">{job.intent}</div>
        </div>
        {job.submitted_proposal && (
          <Badge tone="accent">{t("detail.iter.toProposal", { id: job.submitted_proposal.slice(0, 8) })}</Badge>
        )}
      </div>

      <div className="rounded-md p-4 mb-4" style={{ background: "var(--bg-muted)" }}>
        <Stepper nodes={trNodes(ITER_NODES, t)} doneKeys={done} activeKey={job.state} />
      </div>

      {(job.state === "running" || job.state === "succeeded") && (
        <>
          <div className="grid grid-cols-1 md:grid-cols-[2fr_3fr] gap-3 mb-4">
            <div>
              <div className="text-[12.5px] font-medium mb-1.5">{t("detail.iter.runCmd")}</div>
              <input className="input input-mono mb-2" value={cmd} onChange={(e) => setCmd(e.target.value)} data-testid="iter-cmd" />
              <Button size="sm" onClick={() => runTests.mutate()} disabled={runTests.isPending} data-testid="iter-run">
                {runTests.isPending ? <><Loader2 size={13} className="animate-spin" /> {t("detail.iter.running")}</> : <><Play size={13} /> {t("detail.iter.run")}</>}
              </Button>
            </div>
            <div>
              <div className="text-[12.5px] font-medium mb-1.5">{t("detail.iter.lastRun")}</div>
              {!lastRun ? (
                <Card className="p-4 text-[12.5px]" style={{ color: "var(--fg-muted)" }}>
                  {t("detail.iter.noRuns")}
                </Card>
              ) : (
                <Card className="p-4 space-y-2 text-[12px]">
                  <div className="flex items-center gap-2">
                    <Badge tone={lastRun.exit_code === 0 ? "ok" : "bad"}>
                      {t("detail.iter.exit", { code: lastRun.exit_code })}
                    </Badge>
                    <span className="font-mono text-[11.5px]" style={{ color: "var(--fg-muted)" }}>
                      {lastRun.duration_ms}ms{lastRun.timed_out ? t("detail.iter.timeout") : ""}
                    </span>
                  </div>
                  {lastRun.stdout && (
                    <pre className="p-2 font-mono text-[11.5px] whitespace-pre-wrap max-h-32 overflow-auto rounded" style={{ background: "var(--bg-muted)" }}>
                      {lastRun.stdout}
                    </pre>
                  )}
                  {lastRun.stderr && (
                    <pre className="p-2 font-mono text-[11.5px] whitespace-pre-wrap max-h-32 overflow-auto rounded" style={{ background: "var(--danger-soft)", color: "var(--danger-soft-fg)" }}>
                      {lastRun.stderr}
                    </pre>
                  )}
                </Card>
              )}
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <input
              className="input flex-1 min-w-[260px]"
              value={submitTitle}
              onChange={(e) => setSubmitTitle(e.target.value)}
            />
            <Button onClick={() => submit.mutate()} disabled={submit.isPending} data-testid="iter-submit">
              {submit.isPending ? <><Loader2 size={14} className="animate-spin" /> {t("detail.iter.submitting")}</> : <><Send size={14} /> {t("detail.iter.submitAsProposal")}</>}
            </Button>
          </div>
        </>
      )}
    </Card>
  );
}
