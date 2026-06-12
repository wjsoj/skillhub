import { cn } from "@/lib/cn";

export function Meter({
  value,
  tone = "primary",
}: {
  value: number;
  tone?: "primary" | "high" | "med" | "low";
}) {
  const clamped = Math.max(0, Math.min(1, value));
  return (
    <div
      className={cn(
        "meter w-full",
        tone === "high" && "meter-high",
        tone === "med" && "meter-med",
        tone === "low" && "meter-low"
      )}
    >
      <span style={{ width: `${clamped * 100}%` }} />
    </div>
  );
}
