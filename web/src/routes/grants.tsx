import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Plus, Loader2 } from "lucide-react";
import { PageHeader } from "@/components/ui/PageHeader";
import { Badge } from "@/components/ui/Badge";
import { createGrant } from "@/lib/api";
import { useT } from "@/i18n";

export function GrantsPage() {
  const t = useT();
  const [granteeDept, setGranteeDept] = useState("");
  const [granteeUser, setGranteeUser] = useState("");
  const [targetSkill, setTargetSkill] = useState("");
  const [scope, setScope] = useState<"read" | "write" | "admin">("read");
  const [reason, setReason] = useState("");
  const [granted, setGranted] = useState<
    { id: string; scope: string; reason: string; target: string }[]
  >([]);

  const grant = useMutation({
    mutationFn: () => createGrant({
      grantee_department_id: granteeDept || null,
      grantee_user_id: granteeUser || null,
      target_skill_id: targetSkill || null,
      scope,
      reason,
    }),
    onSuccess: (r) => {
      setGranted((g) => [{ id: r.id, scope, reason, target: targetSkill }, ...g]);
      setReason("");
    },
  });

  return (
    <>
      <PageHeader
        eyebrow={t("grants.eyebrow")}
        title={
          <>
            {t("grants.titleLead")}<span className="serif-em">{t("grants.titleEm")}</span>
          </>
        }
        description={t("grants.desc")}
      />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-12">
        {/* Form */}
        <section>
          <h3 className="text-[15px] font-semibold mb-4">{t("grants.open")}</h3>
          <form
            className="space-y-4"
            onSubmit={(e) => {
              e.preventDefault();
              if ((granteeDept || granteeUser) && targetSkill && reason) grant.mutate();
            }}
          >
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <Field label={t("grants.field.dept")}>
                <input className="input input-mono" value={granteeDept} onChange={(e) => setGranteeDept(e.target.value)} placeholder={t("grants.ph.dept")} data-testid="grant-dept" />
              </Field>
              <Field label={t("grants.field.user")}>
                <input className="input input-mono" value={granteeUser} onChange={(e) => setGranteeUser(e.target.value)} placeholder={t("grants.ph.user")} data-testid="grant-user" />
              </Field>
              <Field label={t("grants.field.skill")}>
                <input className="input input-mono" value={targetSkill} onChange={(e) => setTargetSkill(e.target.value)} placeholder={t("grants.ph.skill")} data-testid="grant-target" />
              </Field>
              <Field label={t("grants.field.scope")}>
                <select className="select" value={scope} onChange={(e) => setScope(e.target.value as typeof scope)} data-testid="grant-scope">
                  <option value="read">{t("grants.scope.read")}</option>
                  <option value="write">{t("grants.scope.write")}</option>
                  <option value="admin">{t("grants.scope.admin")}</option>
                </select>
              </Field>
            </div>
            <Field label={t("grants.field.why")}>
              <input className="input" value={reason} onChange={(e) => setReason(e.target.value)} placeholder={t("grants.ph.why")} data-testid="grant-reason" />
            </Field>
            <div className="flex items-center gap-3 pt-2">
              <button className="btn btn-primary" disabled={!reason || !targetSkill || grant.isPending} data-testid="grant-create">
                {grant.isPending ? <><Loader2 size={14} className="animate-spin" /> {t("grants.creating")}</> : <><Plus size={14} /> {t("grants.create")}</>}
              </button>
              {grant.error && <Badge tone="bad">{(grant.error as Error).message}</Badge>}
            </div>
          </form>
        </section>

        {/* Ledger */}
        <section>
          <h3 className="text-[15px] font-semibold mb-4">{t("grants.recent")}</h3>
          {granted.length === 0 ? (
            <p className="text-[13.5px]" style={{ color: "var(--fg-muted)" }}>
              {t("grants.none")}
            </p>
          ) : (
            <ul>
              {granted.map((g, i) => (
                <li
                  key={g.id}
                  className="py-4"
                  style={{ borderTop: i === 0 ? "1px solid var(--border)" : "0", borderBottom: "1px solid var(--border)" }}
                  data-testid="grant-row"
                >
                  <div className="flex items-baseline justify-between gap-3 mb-1">
                    <span className="font-mono text-[12px]" style={{ color: "var(--fg-muted)" }}>{g.id.slice(0, 8)}</span>
                    <Badge tone="accent">{g.scope}</Badge>
                  </div>
                  <div className="text-[14.5px]" style={{ color: "var(--fg)" }}>{g.reason}</div>
                  <div className="text-[12px] font-mono mt-1" style={{ color: "var(--fg-faint)" }}>
                    → {g.target.slice(0, 8)}
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block">
      <div className="text-[13px] font-medium mb-1.5">{label}</div>
      {children}
    </label>
  );
}
