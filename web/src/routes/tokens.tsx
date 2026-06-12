import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus, Loader2, Trash2, Copy, Check, KeyRound } from "lucide-react";
import { PageHeader } from "@/components/ui/PageHeader";
import { Badge, Tag } from "@/components/ui/Badge";
import {
  listTokens,
  createToken,
  revokeToken,
  type ApiToken,
} from "@/lib/api";
import { useT } from "@/i18n";

export function TokensPage() {
  const t = useT();
  const qc = useQueryClient();
  const q = useQuery({ queryKey: ["tokens"], queryFn: listTokens });

  const [name, setName] = useState("");
  const [scopes, setScopes] = useState("");
  const [fresh, setFresh] = useState<{ id: string; token: string } | null>(null);

  const create = useMutation({
    mutationFn: () =>
      createToken({
        name: name.trim(),
        scopes: scopes ? scopes.split(",").map((s) => s.trim()).filter(Boolean) : [],
      }),
    onSuccess: (tk) => {
      setFresh({ id: tk.id, token: tk.token });
      setName("");
      setScopes("");
      qc.invalidateQueries({ queryKey: ["tokens"] });
    },
  });

  const revoke = useMutation({
    mutationFn: (id: string) => revokeToken(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["tokens"] }),
  });

  return (
    <>
      <PageHeader
        eyebrow={t("tokens.eyebrow")}
        title={<>{t("tokens.titleLead")}<span className="serif-em">{t("tokens.titleEm")}</span></>}
        description={t("tokens.desc")}
      />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-12">
        <section>
          <h3 className="text-[15px] font-semibold mb-4">{t("tokens.create")}</h3>
          <form
            className="space-y-3"
            onSubmit={(e) => { e.preventDefault(); if (name.trim()) create.mutate(); }}
          >
            <label className="block">
              <div className="text-[13px] font-medium mb-1.5">{t("tokens.name")}</div>
              <input className="input" value={name} onChange={(e) => setName(e.target.value)} placeholder={t("tokens.namePh")} data-testid="token-name" />
            </label>
            <label className="block">
              <div className="text-[13px] font-medium mb-1.5">{t("tokens.scopes")}</div>
              <input className="input input-mono" value={scopes} onChange={(e) => setScopes(e.target.value)} placeholder={t("tokens.scopesPh")} data-testid="token-scopes" />
            </label>
            <button className="btn btn-primary" disabled={!name.trim() || create.isPending} data-testid="token-create">
              {create.isPending ? <><Loader2 size={14} className="animate-spin" /> {t("tokens.creating")}</> : <><Plus size={14} /> {t("tokens.createBtn")}</>}
            </button>
            {create.error && <Badge tone="bad">{(create.error as Error).message}</Badge>}
          </form>

          {fresh && <FreshToken token={fresh.token} />}
        </section>

        <section>
          <h3 className="text-[15px] font-semibold mb-4">{t("tokens.your")}</h3>
          {q.isLoading && <Loader2 size={18} className="animate-spin" style={{ color: "var(--fg-muted)" }} />}
          {!q.isLoading && (q.data?.length ?? 0) === 0 && (
            <p className="text-[13.5px]" style={{ color: "var(--fg-muted)" }}>{t("tokens.none")}</p>
          )}
          <ul data-testid="token-list">
            {q.data?.map((tk, i) => (
              <TokenRow key={tk.id} tk={tk} first={i === 0} onRevoke={() => revoke.mutate(tk.id)} revoking={revoke.isPending} />
            ))}
          </ul>
        </section>
      </div>
    </>
  );
}

function FreshToken({ token }: { token: string }) {
  const t = useT();
  const [copied, setCopied] = useState(false);
  return (
    <div className="mt-5 p-4 rounded-2xl" style={{ background: "var(--ok-soft)" }} data-testid="token-fresh">
      <div className="flex items-center gap-2 mb-2">
        <KeyRound size={14} style={{ color: "var(--ok)" }} />
        <span className="text-[13px] font-semibold" style={{ color: "var(--ok)" }}>{t("tokens.copyHint")}</span>
      </div>
      <div className="flex items-center gap-2">
        <code className="font-mono text-[12px] flex-1 truncate px-3 py-2 rounded-lg" style={{ background: "var(--surface)", border: "1px solid var(--border)" }}>
          {token}
        </code>
        <button
          className="btn btn-secondary btn-sm"
          onClick={() => { navigator.clipboard.writeText(token); setCopied(true); setTimeout(() => setCopied(false), 1500); }}
        >
          {copied ? <><Check size={13} /> {t("tokens.copied")}</> : <><Copy size={13} /> {t("tokens.copy")}</>}
        </button>
      </div>
    </div>
  );
}

function TokenRow({ tk, first, onRevoke, revoking }: { tk: ApiToken; first: boolean; onRevoke: () => void; revoking: boolean }) {
  const t = useT();
  return (
    <li
      className="py-4 flex items-start justify-between gap-3"
      style={{ borderTop: first ? "1px solid var(--border)" : "0", borderBottom: "1px solid var(--border)" }}
      data-testid="token-row"
    >
      <div className="min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <span className="text-[14.5px] font-semibold">{tk.name}</span>
          <span className="font-mono text-[11.5px]" style={{ color: "var(--fg-faint)" }}>{tk.prefix}…</span>
        </div>
        <div className="flex flex-wrap items-center gap-1.5 mb-1">
          {tk.scopes.length === 0 ? (
            <span className="text-[12px]" style={{ color: "var(--fg-faint)" }}>—</span>
          ) : tk.scopes.map((s) => <Tag key={s}>{s}</Tag>)}
        </div>
        <div className="text-[12px]" style={{ color: "var(--fg-faint)" }}>
          {t("tokens.lastUsed")}: {tk.last_used_at ? new Date(tk.last_used_at).toLocaleString() : t("tokens.never")}
        </div>
      </div>
      <button className="btn btn-ghost btn-sm" onClick={onRevoke} disabled={revoking} data-testid="token-revoke" style={{ color: "var(--bad)" }}>
        <Trash2 size={13} /> {t("tokens.revoke")}
      </button>
    </li>
  );
}
