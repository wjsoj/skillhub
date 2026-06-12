import { Moon, Sun } from "lucide-react";
import { useTheme } from "@/lib/theme";

/** Single icon toggle — light <-> dark. System default is the implicit
 *  starting state; clicking once locks in a manual preference. */
export function ThemeToggle() {
  const { resolved, toggle } = useTheme();
  const isDark = resolved === "dark";
  return (
    <button
      type="button"
      onClick={toggle}
      aria-label={isDark ? "Switch to light mode" : "Switch to dark mode"}
      className="btn btn-ghost btn-icon"
      style={{ color: "var(--fg-muted)" }}
    >
      {isDark ? <Sun size={16} /> : <Moon size={16} />}
    </button>
  );
}
