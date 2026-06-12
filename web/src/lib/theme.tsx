import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

export type Theme = "light" | "dark" | "system";

interface Ctx {
  theme: Theme;            // user pref (may be "system")
  resolved: "light" | "dark"; // effective
  set: (t: Theme) => void;
  toggle: () => void;
}

const ThemeContext = createContext<Ctx | null>(null);
const STORAGE_KEY = "skillhub.theme";

function systemPref(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function applyClass(resolved: "light" | "dark") {
  const root = document.documentElement;
  if (resolved === "dark") root.classList.add("dark");
  else root.classList.remove("dark");
  // also keep the <meta name="theme-color"> reasonable
  const meta = document.querySelector('meta[name="theme-color"]') as HTMLMetaElement | null;
  if (meta) meta.content = resolved === "dark" ? "#0a0a0a" : "#ffffff";
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useState<Theme>(() => {
    const v = localStorage.getItem(STORAGE_KEY);
    return v === "light" || v === "dark" ? v : "system";
  });
  const [systemResolved, setSystemResolved] = useState<"light" | "dark">(() => systemPref());

  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => setSystemResolved(e.matches ? "dark" : "light");
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  const resolved = theme === "system" ? systemResolved : theme;

  useEffect(() => {
    applyClass(resolved);
  }, [resolved]);

  const set = useCallback((t: Theme) => {
    setTheme(t);
    if (t === "system") localStorage.removeItem(STORAGE_KEY);
    else localStorage.setItem(STORAGE_KEY, t);
  }, []);

  const toggle = useCallback(() => {
    set(resolved === "dark" ? "light" : "dark");
  }, [resolved, set]);

  const value = useMemo<Ctx>(
    () => ({ theme, resolved, set, toggle }),
    [theme, resolved, set, toggle]
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used inside ThemeProvider");
  return ctx;
}
