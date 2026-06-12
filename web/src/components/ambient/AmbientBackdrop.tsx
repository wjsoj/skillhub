import { Suspense, lazy, useEffect, useState } from "react";

const ParticleField = lazy(() => import("./ParticleField"));

/**
 * The atmospheric layer behind the whole app: a static gradient-mesh + grain
 * base (cheap, always on) with an optional WebGL particle field on top.
 *
 * The 3D layer is opt-in by capability — desktop, motion allowed, not a
 * coarse pointer — and lazy-loaded so it never blocks first paint or ships
 * three.js to phones. Everything sits at pointer-events:none behind content.
 */
export function AmbientBackdrop() {
  const [show3d, setShow3d] = useState(false);
  const [tint, setTint] = useState("#ff5340");

  useEffect(() => {
    const motionOk = !window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const wideEnough = window.matchMedia("(min-width: 1024px)").matches;
    const finePointer = window.matchMedia("(pointer: fine)").matches;
    setShow3d(motionOk && wideEnough && finePointer);

    const readTint = () => {
      const c = getComputedStyle(document.documentElement)
        .getPropertyValue("--accent")
        .trim();
      if (c) setTint(c);
    };
    readTint();
    // Re-read the accent when the theme flips so the cloud matches.
    const obs = new MutationObserver(readTint);
    obs.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
    return () => obs.disconnect();
  }, []);

  return (
    <div aria-hidden className="ambient-root">
      {/* Layered radial gradient mesh — warm coral haze top-right, cool drift
          bottom-left, kept very low alpha for depth without colour noise. */}
      <div className="ambient-mesh" />
      {/* Fine film grain to kill banding and add tactile texture. */}
      <div className="ambient-grain" />

      {show3d && (
        <div className="ambient-canvas">
          <Suspense fallback={null}>
            <ParticleField tint={tint} />
          </Suspense>
        </div>
      )}
    </div>
  );
}
