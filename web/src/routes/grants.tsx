import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Plus, Loader2 } from "lucide-react";
import { PageHeader } from "@/components/ui/PageHeader";
import { Badge } from "@/components/ui/Badge";
import { createGrant } from "@/lib/api";

export function GrantsPage() {
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
        eyebrow="Grants"
        title={
          <>
            Borrow, <span className="serif-em">with a paper trail.</span>
          </>
        }
        description="A grant lets someone outside the home department read or write a specific skill. Every grant is written down — who, what, why, and for how long."
      />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-12">
        {/* Form */}
        <section>
          <h3 className="text-[15px] font-semibold mb-4">Open a grant</h3>
          <form
            className="space-y-4"
            onSubmit={(e) => {
              e.preventDefault();
              if ((granteeDept || granteeUser) && targetSkill && reason) grant.mutate();
            }}
          >
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <Field label="To a department (id)">
                <input className="input input-mono" value={granteeDept} onChange={(e) => setGranteeDept(e.target.value)} placeholder="dept uuid (optional)" data-testid="grant-dept" />
              </Field>
              <Field label="…or to a person (id)">
                <input className="input input-mono" value={granteeUser} onChange={(e) => setGranteeUser(e.target.value)} placeholder="user uuid (optional)" data-testid="grant-user" />
              </Field>
              <Field label="On this skill (id)">
                <input className="input input-mono" value={targetSkill} onChange={(e) => setTargetSkill(e.target.value)} placeholder="skill uuid" data-testid="grant-target" />
              </Field>
              <Field label="Can do what">
                <select className="select" value={scope} onChange={(e) => setScope(e.target.value as typeof scope)} data-testid="grant-scope">
                  <option value="read">Read</option>
                  <option value="write">Write</option>
                  <option value="admin">Admin</option>
                </select>
              </Field>
            </div>
            <Field label="Why">
              <input className="input" value={reason} onChange={(e) => setReason(e.target.value)} placeholder="Q2 close — finance needs read access for reconciliation." data-testid="grant-reason" />
            </Field>
            <div className="flex items-center gap-3 pt-2">
              <button className="btn btn-primary" disabled={!reason || !targetSkill || grant.isPending} data-testid="grant-create">
                {grant.isPending ? <><Loader2 size={14} className="animate-spin" /> Creating…</> : <><Plus size={14} /> Create grant</>}
              </button>
              {grant.error && <Badge tone="bad">{(grant.error as Error).message}</Badge>}
            </div>
          </form>
        </section>

        {/* Ledger */}
        <section>
          <h3 className="text-[15px] font-semibold mb-4">Recent grants</h3>
          {granted.length === 0 ? (
            <p className="text-[13.5px]" style={{ color: "var(--fg-muted)" }}>
              Nothing issued yet in this session.
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
