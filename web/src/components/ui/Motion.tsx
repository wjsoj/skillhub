import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { motion, useReducedMotion, useInView, animate } from "motion/react";
import type { Variants } from "motion/react";

const EASE = [0.16, 1, 0.3, 1] as const;

/**
 * Shared motion primitives. One restrained vocabulary — a short rise + fade —
 * used everywhere so the whole app breathes the same way. All of it collapses
 * to instant when the user prefers reduced motion.
 */

export function Reveal({
  children,
  delay = 0,
  y = 14,
  className,
}: {
  children: ReactNode;
  delay?: number;
  y?: number;
  className?: string;
}) {
  const reduce = useReducedMotion();
  return (
    <motion.div
      className={className}
      initial={reduce ? false : { opacity: 0, y }}
      whileInView={reduce ? undefined : { opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-60px" }}
      transition={{ duration: 0.6, ease: EASE, delay }}
    >
      {children}
    </motion.div>
  );
}

/** Container that staggers its direct <Stagger.Item> children into view. */
const containerVariants: Variants = {
  hidden: {},
  show: { transition: { staggerChildren: 0.06, delayChildren: 0.04 } },
};
const itemVariants: Variants = {
  hidden: { opacity: 0, y: 16 },
  show: { opacity: 1, y: 0, transition: { duration: 0.55, ease: EASE } },
};

export function StaggerGroup({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  const reduce = useReducedMotion();
  return (
    <motion.div
      className={className}
      variants={reduce ? undefined : containerVariants}
      initial={reduce ? false : "hidden"}
      whileInView={reduce ? undefined : "show"}
      viewport={{ once: true, margin: "-40px" }}
    >
      {children}
    </motion.div>
  );
}

export function StaggerItem({
  children,
  className,
  style,
}: {
  children: ReactNode;
  className?: string;
  style?: React.CSSProperties;
}) {
  const reduce = useReducedMotion();
  return (
    <motion.div
      className={className}
      style={style}
      variants={reduce ? undefined : itemVariants}
    >
      {children}
    </motion.div>
  );
}

/** A tile that lifts a hair and warms its border on hover. */
export function HoverTile({
  children,
  className,
  style,
}: {
  children: ReactNode;
  className?: string;
  style?: React.CSSProperties;
}) {
  const reduce = useReducedMotion();
  return (
    <motion.div
      className={className}
      style={style}
      whileHover={reduce ? undefined : { y: -3 }}
      transition={{ duration: 0.3, ease: EASE }}
    >
      {children}
    </motion.div>
  );
}

/** Number that counts up once on first view. */
export function CountUp({
  value,
  format = (n) => Math.round(n).toLocaleString(),
  className,
  duration = 1.1,
}: {
  value: number;
  format?: (n: number) => string;
  className?: string;
  duration?: number;
}) {
  const reduce = useReducedMotion();
  const ref = useRef<HTMLSpanElement>(null);
  const inView = useInView(ref, { once: true, margin: "-40px" });
  const [text, setText] = useState(() => format(reduce ? value : 0));

  useEffect(() => {
    if (reduce) { setText(format(value)); return; }
    if (!inView) return;
    const controls = animate(0, value, {
      duration,
      ease: EASE,
      onUpdate: (v) => setText(format(v)),
    });
    return () => controls.stop();
  }, [inView, value, reduce, duration]); // eslint-disable-line react-hooks/exhaustive-deps

  return <span ref={ref} className={className}>{text}</span>;
}

export { motion, useReducedMotion };
