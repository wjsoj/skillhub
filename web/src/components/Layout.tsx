import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import * as Dialog from "@radix-ui/react-dialog";
import { Menu, X, LogOut } from "lucide-react";
import { useEffect, useState } from "react";
import { clearMockUser, getMockUser, setMockUser } from "@/lib/api";
import { cn } from "@/lib/cn";
import { ThemeToggle } from "@/components/ui/ThemeToggle";
import { LocaleToggle } from "@/components/ui/LocaleToggle";
import { AmbientBackdrop } from "@/components/ambient/AmbientBackdrop";
import { useT } from "@/i18n";
import type { TKey } from "@/i18n/dict";

const NAV = [
  { to: "/", label: "nav.home" },
  { to: "/skills", label: "nav.skills" },
  { to: "/orgs", label: "nav.org" },
  { to: "/grants", label: "nav.grants" },
  { to: "/audit", label: "nav.audit" },
] as const satisfies ReadonlyArray<{ to: string; label: TKey }>;

export function Layout() {
  const t = useT();
  const user = getMockUser();
  const [drawer, setDrawer] = useState(false);
  const [showLogin, setShowLogin] = useState(!user);

  return (
    <div className="relative" style={{ background: "var(--bg)", minHeight: "100vh" }}>
      <AmbientBackdrop />

      <div className="relative" style={{ zIndex: 1 }}>
      <TopBar user={user} onMenu={() => setDrawer(true)} onLogout={() => { clearMockUser(); location.reload(); }} />

      <main className="px-5 sm:px-8 lg:px-12 pb-24" style={{ minHeight: "calc(100vh - 64px - 80px)" }}>
        <div className="mx-auto w-full max-w-[1140px]">
          <Outlet />
        </div>
      </main>

      <footer
        className="px-5 sm:px-8 lg:px-12 py-7 text-[13px] flex items-center justify-between"
        style={{ color: "var(--fg-subtle)", borderTop: "1px solid var(--border)" }}
      >
        <span>SkillHub · {t("footer.tagline")}</span>
        <span className="font-mono text-[11.5px]">v0.1</span>
      </footer>
      </div>

      {/* Mobile drawer */}
      <Dialog.Root open={drawer} onOpenChange={setDrawer}>
        <Dialog.Portal>
          <Dialog.Overlay
            className="fixed inset-0 z-40 transition-opacity duration-200 data-[state=open]:opacity-100 data-[state=closed]:opacity-0"
            style={{ background: "rgba(0,0,0,0.4)" }}
          />
          <Dialog.Content
            className="fixed inset-x-3 top-3 z-50 rounded-2xl p-5 outline-none data-[state=open]:animate-in data-[state=closed]:animate-out"
            style={{ background: "var(--surface)", border: "1px solid var(--border)" }}
          >
            <div className="flex items-center justify-between mb-5">
              <Dialog.Title className="text-[15px] font-semibold">{t("nav.navigate")}</Dialog.Title>
              <Dialog.Close asChild>
                <button className="btn btn-ghost btn-icon" aria-label={t("org.cancel")}><X size={16} /></button>
              </Dialog.Close>
            </div>
            <nav className="flex flex-col gap-1">
              {NAV.map((n) => (
                <Link
                  key={n.to}
                  to={n.to}
                  onClick={() => setDrawer(false)}
                  className="nav-link justify-between"
                >
                  {t(n.label)}
                </Link>
              ))}
            </nav>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>

      {showLogin && (
        <LoginGate
          onSubmit={(id, name) => {
            setMockUser(id, name);
            setShowLogin(false);
            location.reload();
          }}
        />
      )}
    </div>
  );
}

function TopBar({
  user,
  onMenu,
  onLogout,
}: {
  user: { id: string; name: string } | null;
  onMenu: () => void;
  onLogout: () => void;
}) {
  const t = useT();
  const state = useRouterState();
  const path = state.location.pathname;
  const isActive = (to: string) =>
    to === "/" ? path === "/" : path === to || path.startsWith(to + "/");

  return (
    <header
      className="sticky top-0 z-30"
      style={{
        background: "color-mix(in srgb, var(--bg) 82%, transparent)",
        backdropFilter: "saturate(180%) blur(10px)",
        WebkitBackdropFilter: "saturate(180%) blur(10px)",
        borderBottom: "1px solid var(--border)",
      }}
    >
      <div className="mx-auto w-full max-w-[1080px] px-5 sm:px-8 lg:px-12 h-16 flex items-center gap-3">
        {/* Brand */}
        <Link to="/" className="flex items-center gap-2.5">
          <Mark />
          <span className="text-[16.5px] font-semibold tracking-tight">SkillHub</span>
        </Link>

        {/* Desktop nav */}
        <nav className="ml-6 hidden md:flex items-center gap-1">
          {NAV.map((n) => (
            <Link key={n.to} to={n.to} className={cn("nav-link", isActive(n.to) && "active")}>
              {t(n.label)}
            </Link>
          ))}
        </nav>

        <div className="flex-1" />

        <LocaleToggle />
        <ThemeToggle />

        {user ? (
          <div className="flex items-center gap-2 pl-2 md:pl-3" style={{ borderLeft: "1px solid var(--border)" }}>
            <div className="hidden sm:flex flex-col items-end leading-tight">
              <span className="text-[13.5px] font-medium">{user.name || t("shell.user")}</span>
              <span className="font-mono text-[10.5px]" style={{ color: "var(--fg-faint)" }}>
                {user.id.slice(0, 8)}
              </span>
            </div>
            <button onClick={onLogout} aria-label={t("shell.signOut")} className="btn btn-ghost btn-icon">
              <LogOut size={15} />
            </button>
          </div>
        ) : null}

        <button onClick={onMenu} aria-label="Open menu" className="md:hidden btn btn-ghost btn-icon">
          <Menu size={16} />
        </button>
      </div>
    </header>
  );
}

function Mark() {
  return (
    <span
      className="inline-flex items-center justify-center w-7 h-7 rounded-full"
      style={{ background: "var(--accent)" }}
      aria-hidden
    >
      <span
        className="block w-2 h-2 rounded-full"
        style={{ background: "var(--accent-fg)" }}
      />
    </span>
  );
}

function LoginGate({ onSubmit }: { onSubmit: (id: string, name: string) => void }) {
  const t = useT();
  const [id, setId] = useState("");
  const [name, setName] = useState("");
  const presets = [
    { id: "00000000-0000-0000-0000-000000000001", name: "ada",   role: t("login.role.data") },
    { id: "00000000-0000-0000-0000-000000000003", name: "carol", role: t("login.role.finance") },
    { id: "00000000-0000-0000-0000-000000000009", name: "admin", role: t("login.role.admin") },
  ];
  useEffect(() => {
    document.body.style.overflow = "hidden";
    return () => { document.body.style.overflow = ""; };
  }, []);
  return (
    <div
      className="fixed inset-0 z-50 flex items-end sm:items-center justify-center p-3 sm:p-6"
      style={{ background: "color-mix(in srgb, var(--bg) 60%, transparent)", backdropFilter: "blur(8px)" }}
    >
      <div className="card card-elevated w-full max-w-[440px] p-7 reveal">
        <div className="flex items-center gap-3 mb-5">
          <Mark />
          <h2 className="text-[18px] font-semibold tracking-tight">{t("login.welcome")}</h2>
        </div>
        <p className="text-[14px] mb-5" style={{ color: "var(--fg-muted)" }}>
          {t("login.pickIdentity")}
        </p>

        <div className="flex flex-col gap-2 mb-5">
          {presets.map((p) => (
            <button
              key={p.id}
              type="button"
              onClick={() => { setId(p.id); setName(p.name); }}
              className="flex items-center justify-between rounded-xl px-4 py-3 text-left transition-all"
              style={{
                background: id === p.id ? "var(--accent-soft)" : "var(--surface-2)",
                border: id === p.id ? "1px solid var(--accent)" : "1px solid var(--border)",
                cursor: "pointer",
              }}
            >
              <div>
                <div className="text-[14.5px] font-semibold" style={{ color: id === p.id ? "var(--accent-soft-fg)" : "var(--fg)" }}>
                  {p.name}
                </div>
                <div className="text-[12.5px]" style={{ color: "var(--fg-muted)" }}>
                  {p.role}
                </div>
              </div>
              <span className="font-mono text-[11px]" style={{ color: "var(--fg-faint)" }}>
                {p.id.slice(0, 8)}
              </span>
            </button>
          ))}
        </div>

        <details className="mb-5">
          <summary className="text-[13px] font-medium cursor-pointer" style={{ color: "var(--fg-muted)" }}>
            {t("login.orPaste")}
          </summary>
          <div className="mt-3 flex flex-col gap-2">
            <input className="input input-mono" placeholder="00000000-…" value={id} onChange={(e) => setId(e.target.value)} />
            <input className="input" placeholder={t("login.displayName")} value={name} onChange={(e) => setName(e.target.value)} />
          </div>
        </details>

        <button
          className="btn btn-primary w-full"
          disabled={!id}
          onClick={() => id && onSubmit(id.trim(), name.trim())}
        >
          {t("login.continue")}
        </button>
      </div>
    </div>
  );
}
