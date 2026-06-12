import { useMemo, useRef } from "react";
import { Canvas, useFrame } from "@react-three/fiber";
import * as THREE from "three";

/**
 * Ambient particle field — a slowly drifting point cloud shaped as a hollow
 * sphere, tinted with the brand coral. Deliberately low-contrast: it lives
 * *behind* the content as atmosphere, never competing with it.
 *
 * Lazy-loaded and gated (see AmbientBackdrop) so it never runs on mobile,
 * with reduced-motion preferences, or on first paint.
 */

const COUNT = 1400;

function Points({ tint }: { tint: string }) {
  const ref = useRef<THREE.Points>(null);

  const geometry = useMemo(() => {
    const positions = new Float32Array(COUNT * 3);
    const scales = new Float32Array(COUNT);
    for (let i = 0; i < COUNT; i++) {
      // Distribute on a thick spherical shell with a little jitter so it
      // reads as a soft volume rather than a hard surface.
      const r = 3.1 + Math.random() * 1.7;
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(2 * Math.random() - 1);
      positions[i * 3] = r * Math.sin(phi) * Math.cos(theta);
      positions[i * 3 + 1] = r * Math.sin(phi) * Math.sin(theta) * 0.78;
      positions[i * 3 + 2] = r * Math.cos(phi);
      scales[i] = Math.random();
    }
    const g = new THREE.BufferGeometry();
    g.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    g.setAttribute("aScale", new THREE.BufferAttribute(scales, 1));
    return g;
  }, []);

  const material = useMemo(() => {
    return new THREE.PointsMaterial({
      size: 0.032,
      color: new THREE.Color(tint),
      transparent: true,
      opacity: 0.9,
      sizeAttenuation: true,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
    });
  }, [tint]);

  useFrame((state, delta) => {
    if (!ref.current) return;
    // Very slow, continuous drift; a faint pointer parallax adds depth.
    ref.current.rotation.y += delta * 0.045;
    ref.current.rotation.x += delta * 0.012;
    const { x, y } = state.pointer;
    ref.current.rotation.y += (x * 0.18 - ref.current.rotation.y * 0) * 0;
    ref.current.position.x = THREE.MathUtils.lerp(ref.current.position.x, x * 0.35, 0.04);
    ref.current.position.y = THREE.MathUtils.lerp(ref.current.position.y, y * 0.25, 0.04);
  });

  return <points ref={ref} geometry={geometry} material={material} />;
}

export default function ParticleField({ tint }: { tint: string }) {
  return (
    <Canvas
      gl={{ antialias: true, alpha: true, powerPreference: "low-power" }}
      camera={{ position: [0, 0, 8], fov: 52 }}
      dpr={[1, 1.6]}
      frameloop="always"
      style={{ width: "100%", height: "100%" }}
    >
      <Points tint={tint} />
    </Canvas>
  );
}
