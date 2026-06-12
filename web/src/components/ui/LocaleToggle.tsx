import { Languages } from "lucide-react";
import { useI18n } from "@/i18n";

/** Compact language switch — flips between English and 简体中文. The current
 *  locale's short label sits next to a globe-ish glyph. */
export function LocaleToggle() {
  const { locale, toggle, t } = useI18n();
  return (
    <button
      type="button"
      onClick={toggle}
      aria-label={t("lang.switch")}
      title={t("lang.switch")}
      className="btn btn-ghost"
      style={{ color: "var(--fg-muted)", height: 38, padding: "0 10px", gap: 6 }}
    >
      <Languages size={16} />
      <span className="text-[12.5px] font-medium tabular-nums">
        {locale === "zh" ? "中" : "EN"}
      </span>
    </button>
  );
}
