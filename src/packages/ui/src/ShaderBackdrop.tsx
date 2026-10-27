import { lazy, Suspense, useEffect, useState } from "react";
import { useReducedMotion } from "motion/react";

const SwirlCanvas = lazy(async () => {
  const { Shader, Swirl } = await import("shaders/react");
  return {
    default: ({ unavailable }: { unavailable: () => void }) => (
      <Shader
        disableTelemetry
        onUnavailable={unavailable}
        style={{ width: "100%", height: "100%" }}
      >
        <Swirl
          colorA="#132b23"
          colorB="#b8ee83"
          speed={0.13}
          detail={1.3}
          blend={24}
        />
      </Shader>
    ),
  };
});

/** Small, decorative GPU surface; unmounted when hidden or animation is disabled. */
export default function ShaderBackdrop({
  enabled = true,
  className = "",
}: {
  enabled?: boolean;
  className?: string;
}) {
  const reduced = useReducedMotion();
  const [available, setAvailable] = useState(
    () => typeof navigator !== "undefined" && "gpu" in navigator,
  );
  const [visible, setVisible] = useState(() => !document.hidden);
  useEffect(() => {
    const change = () => setVisible(!document.hidden);
    document.addEventListener("visibilitychange", change);
    return () => document.removeEventListener("visibilitychange", change);
  }, []);
  return (
    <div
      aria-hidden="true"
      className={`shader-backdrop ${className}`}
      style={{
        position: "absolute",
        inset: 0,
        pointerEvents: "none",
        overflow: "hidden",
        opacity: 0.35,
        background:
          "radial-gradient(ellipse at 70% 30%, #92d65f55, transparent 65%), linear-gradient(125deg, #142c24, #101113)",
      }}
    >
      {enabled && !reduced && available && visible && (
        <Suspense fallback={null}>
          <SwirlCanvas unavailable={() => setAvailable(false)} />
        </Suspense>
      )}
    </div>
  );
}
