import { Check } from "lucide-react";
import { cn } from "@/lib/cn";

export interface StepperNode {
  key: string;
  label: string;
}

export function Stepper({
  nodes,
  activeKey,
  doneKeys = [],
}: {
  nodes: StepperNode[];
  activeKey?: string;
  doneKeys?: string[];
}) {
  return (
    <div className="stepper">
      {nodes.map((n, i) => {
        const done = doneKeys.includes(n.key);
        const active = n.key === activeKey;
        return (
          <div key={n.key} className="flex items-center gap-2">
            <div className={cn("node", done && "done", active && "active")}>
              <span className="dot">
                {done ? <Check size={11} strokeWidth={2.5} /> : i + 1}
              </span>
              <span>{n.label}</span>
            </div>
            {i < nodes.length - 1 && <span className="arm" />}
          </div>
        );
      })}
    </div>
  );
}
