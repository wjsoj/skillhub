import { useMemo, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { ArrowLeft, Sparkles, CheckCircle2, AlertTriangle, Loader2 } from "lucide-react";
import { PageHeader } from "@/components/ui/PageHeader";
import { Badge, Tag } from "@/components/ui/Badge";
import { Meter } from "@/components/ui/Meter";
import { checkDuplicate, createSkill, listNamespaces, type DuplicateReport } from "@/lib/api";
import { useT } from "@/i18n";
import type { TKey } from "@/i18n/dict";

export function SkillsNewPage() {
  const t = useT();
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [namespace, setNamespace] = useState("");
  const [description, setDescription] = useState("");
  const [readme, setReadme] = useState("");
  const [tags, setTags] = useState("");
  const [report, setReport] = useState<DuplicateReport | null>(null);
  const [override, setOverride] = useState(false);

  const namespaces = useQuery({ queryKey: ["namespaces"], queryFn: listNamespaces });

  const tagList = () => tags.split(",").map((s) => s.trim()).filter(Boolean);

  const dup = useMutation({
    mutationFn: () =>
      checkDuplicate({
        display_name: name,
        slug,
        description: description || undefined,
        readme: readme || undefined,
        tags: tags ? tagList() : undefined,
      }),
    onSuccess: setReport,
  });

  const publish = useMutation({
    mutationFn: () =>
      createSkill({
        namespace,
        slug: slug.trim(),
        display_name: name.trim(),
        description: description.trim() || undefined,
        readme: readme.trim() || undefined,
        tags: tagList(),
        manifest: { version: "0.1.0" },
      }),
    onSuccess: (skill) =>
      navigate({
        to: "/skills/$namespace/$slug",
        params: { namespace: skill.namespace_slug, slug: skill.slug },
      }),
  });

  const enough = useMemo(() => name.trim().length >= 4 && slug.trim().length >= 3, [name, slug]);
  const high = report?.candidates.find((c) => c.confidence === "high") ?? null;
  const canPublish = enough && !!namespace && (!high || override);

  return (
    <>
      <Link to="/skills" className="inline-flex items-center gap-1 text-[13.5px] mt-2 mb-2" style={{ color: "var(--fg-muted)" }}>
        <ArrowLeft size={13} /> {t("new.back")}
      </Link>
      <PageHeader
        eyebrow={t("new.eyebrow")}
        title={
          <>
            {t("new.titleLead")}<span className="serif-em">{t("new.titleEm")}</span>{t("new.titleTail")}
          </>
        }
        description={t("new.desc")}
      />

      <div className="grid grid-cols-1 lg:grid-cols-[1fr_380px] gap-12">
        {/* Form */}
        <form
          className="space-y-7"
          onSubmit={(e) => { e.preventDefault(); dup.mutate(); }}
        >
          <Field label={t("new.field.name")}>
            <input
              data-testid="field-name"
              className="input input-lg"
              style={{ height: 52, fontSize: 17, padding: "0 16px" }}
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="PDF parser"
            />
          </Field>
          <Field label={t("new.field.slug")} hint={t("new.field.slugHint")}>
            <input
              data-testid="field-slug"
              className="input input-mono"
              value={slug}
              onChange={(e) => setSlug(e.target.value)}
              placeholder="pdf-parser"
            />
          </Field>
          <Field label={t("new.field.namespace")}>
            <select
              data-testid="field-namespace"
              className="select"
              value={namespace}
              onChange={(e) => setNamespace(e.target.value)}
            >
              <option value="">—</option>
              {(namespaces.data ?? []).map((n) => (
                <option key={n.id} value={n.slug}>
                  {n.slug} · {n.display_name}
                </option>
              ))}
            </select>
          </Field>
          <Field label={t("new.field.desc")}>
            <input
              data-testid="field-description"
              className="input"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Extract text and tables from PDF documents."
            />
          </Field>
          <Field label={t("new.field.tags")} hint={t("new.field.tagsHint")}>
            <input
              data-testid="field-tags"
              className="input input-mono"
              value={tags}
              onChange={(e) => setTags(e.target.value)}
              placeholder="pdf, text, tables, ocr"
            />
          </Field>
          <Field label={t("new.field.notes")} hint={t("new.field.notesHint")}>
            <textarea
              data-testid="field-readme"
              className="textarea"
              rows={5}
              value={readme}
              onChange={(e) => setReadme(e.target.value)}
              placeholder={t("new.ph.notes")}
            />
          </Field>

          <div className="flex flex-wrap items-center gap-3 pt-2">
            <button
              type="submit"
              className="btn btn-primary"
              disabled={!enough || dup.isPending}
              data-testid="btn-check"
            >
              {dup.isPending ? <><Loader2 size={14} className="animate-spin" /> {t("new.checking")}</> : <><Sparkles size={14} /> {t("new.check")}</>}
            </button>
            <button
              type="button"
              className="btn btn-primary"
              disabled={!canPublish || publish.isPending}
              onClick={() => publish.mutate()}
              data-testid="btn-submit"
            >
              {publish.isPending ? <><Loader2 size={14} className="animate-spin" /> {t("new.publishing")}</> : t("new.publish")}
            </button>
            {high && !override && (
              <span className="text-[13px]" style={{ color: "var(--bad)" }}>
                {t("new.dupWarn")}
              </span>
            )}
            {enough && !namespace && (
              <span className="text-[13px]" style={{ color: "var(--fg-muted)" }}>
                {t("new.needNamespace")}
              </span>
            )}
          </div>

          {high && (
            <label className="flex items-start gap-2 text-[13px] cursor-pointer mt-2" style={{ color: "var(--fg-muted)" }}>
              <input type="checkbox" checked={override} onChange={(e) => setOverride(e.target.checked)} />
              {t("new.dupConfirmPre")} <span className="font-mono">{high.hit.namespace_slug}/{high.hit.slug}</span> {t("new.dupConfirmPost")}
            </label>
          )}

          {dup.error && (
            <div className="mt-4 px-4 py-3 rounded-lg" style={{ background: "var(--bad-soft)", color: "var(--bad)" }}>
              <span className="font-mono text-[12.5px]">{(dup.error as Error).message}</span>
            </div>
          )}
          {publish.error && (
            <div className="mt-4 px-4 py-3 rounded-lg" style={{ background: "var(--bad-soft)", color: "var(--bad)" }} data-testid="publish-error">
              <span className="font-mono text-[12.5px]">{(publish.error as Error).message}</span>
            </div>
          )}
        </form>

        {/* Side: matches */}
        <aside className="lg:sticky lg:top-24 self-start space-y-3">
          <div className="flex items-baseline justify-between mb-3">
            <h3 className="text-[15px] font-semibold">{t("new.similar")}</h3>
            {report && (
              <span className="text-[12px]" style={{ color: "var(--fg-faint)" }}>
                {t("common.found", { count: report.candidates.length })}
              </span>
            )}
          </div>

          {!report && (
            <div className="py-12 text-center">
              <Sparkles size={20} className="mx-auto mb-2" style={{ color: "var(--fg-faint)" }} />
              <p className="text-[13px] max-w-[260px] mx-auto" style={{ color: "var(--fg-muted)" }}>
                {t("new.emptyHint")}
              </p>
            </div>
          )}

          {report && report.candidates.length === 0 && (
            <div className="flex items-start gap-3 p-4 rounded-2xl" style={{ background: "var(--ok-soft)" }}>
              <CheckCircle2 size={18} style={{ color: "var(--ok)", flexShrink: 0, marginTop: 1 }} />
              <div>
                <div className="text-[14px] font-semibold" style={{ color: "var(--ok)" }}>
                  {t("new.nothingSimilar")}
                </div>
                <div className="text-[12.5px]" style={{ color: "var(--fg-muted)" }}>
                  {t("new.goodToPublish")}
                </div>
              </div>
            </div>
          )}

          {report?.candidates.map((c) => {
            const tone = c.confidence === "high" ? "bad" : c.confidence === "medium" ? "warn" : "default";
            const confKey: TKey = c.confidence === "high" ? "conf.high" : c.confidence === "medium" ? "conf.medium" : "conf.low";
            return (
              <div key={c.hit.skill_id} className="py-4" style={{ borderBottom: "1px solid var(--border)" }} data-testid="dup-candidate">
                <div className="flex items-baseline justify-between mb-1 gap-2">
                  <span className="text-[12px] font-mono truncate" style={{ color: "var(--fg-muted)" }}>
                    {c.hit.namespace_slug}/{c.hit.slug}
                  </span>
                  <Badge tone={tone}>{t(confKey)}</Badge>
                </div>
                <div className="text-[15px] font-semibold tracking-tight mb-2">{c.hit.display_name}</div>
                {c.hit.description && (
                  <p className="text-[13px] leading-snug mb-3" style={{ color: "var(--fg-muted)" }}>
                    {c.hit.description}
                  </p>
                )}
                <div className="flex items-center gap-3">
                  <Meter
                    value={c.hit.score}
                    tone={c.confidence === "high" ? "high" : c.confidence === "medium" ? "med" : "low"}
                  />
                  <span className="font-mono text-[11.5px] tabular-nums" style={{ color: "var(--fg-muted)" }}>
                    {c.hit.score.toFixed(2)}
                  </span>
                </div>
                {c.suggested_action === "use_existing" && (
                  <div className="mt-2">
                    <Tag>{t("new.useExisting")}</Tag>
                  </div>
                )}
              </div>
            );
          })}
        </aside>
      </div>
    </>
  );
}

function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <label className="block">
      <div className="flex items-baseline justify-between mb-2">
        <span className="text-[14px] font-medium">{label}</span>
        {hint && (
          <span className="text-[12px]" style={{ color: "var(--fg-faint)" }}>{hint}</span>
        )}
      </div>
      {children}
    </label>
  );
}
