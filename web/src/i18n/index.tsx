import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { dictionaries, type TKey } from "./dict";

export type Locale = "en" | "zh";

const STORAGE_KEY = "skillhub.locale";

/** First visit: follow the browser's language preference order. */
function detectLocale(): Locale {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "en" || stored === "zh") return stored;
  const langs =
    navigator.languages && navigator.languages.length
      ? navigator.languages
      : [navigator.language];
  for (const raw of langs) {
    const l = (raw || "").toLowerCase();
    if (l.startsWith("zh")) return "zh";
    if (l.startsWith("en")) return "en";
  }
  return "en";
}

function applyLang(locale: Locale) {
  document.documentElement.lang = locale === "zh" ? "zh-CN" : "en";
}

type TFn = (key: TKey, vars?: Record<string, string | number>) => string;

interface Ctx {
  locale: Locale;
  setLocale: (l: Locale) => void;
  toggle: () => void;
  t: TFn;
}

const I18nContext = createContext<Ctx | null>(null);

function interpolate(s: string, vars?: Record<string, string | number>): string {
  if (!vars) return s;
  return s.replace(/\{(\w+)\}/g, (m, k) =>
    k in vars ? String(vars[k]) : m
  );
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(() => detectLocale());

  useEffect(() => {
    applyLang(locale);
  }, [locale]);

  const setLocale = useCallback((l: Locale) => {
    setLocaleState(l);
    localStorage.setItem(STORAGE_KEY, l);
  }, []);

  const toggle = useCallback(() => {
    setLocale(locale === "zh" ? "en" : "zh");
  }, [locale, setLocale]);

  const t = useCallback<TFn>(
    (key, vars) => {
      const table = dictionaries[locale];
      const raw = table[key] ?? dictionaries.en[key] ?? key;
      return interpolate(raw, vars);
    },
    [locale]
  );

  const value = useMemo<Ctx>(
    () => ({ locale, setLocale, toggle, t }),
    [locale, setLocale, toggle, t]
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used inside I18nProvider");
  return ctx;
}

/** Convenience: just the translate function. */
export function useT() {
  return useI18n().t;
}
