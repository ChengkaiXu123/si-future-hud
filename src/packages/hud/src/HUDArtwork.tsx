import { Terminal } from "lucide-react";
import { agentArtwork } from "./agent-artwork";

export interface HUDArtworkProps {
  source: string;
  size?: number;
  light?: boolean;
}
/** The source artwork and tinting used by both HUDLogo and BotMark's original logo. */
export function HUDArtwork({
  source,
  size = 20,
  light = false,
}: HUDArtworkProps) {
  const artwork = agentArtwork(source, light);
  if (!artwork)
    return (
      <span
        role="img"
        aria-label={source}
        style={{
          width: size,
          height: size,
          display: "inline-grid",
          placeItems: "center",
          flexShrink: 0,
        }}
      >
        <Terminal aria-hidden="true" size={size * 0.75} strokeWidth={1.6} />
      </span>
    );
  if (artwork.tint)
    return (
      <span
        role="img"
        aria-label={source}
        style={{
          display: "inline-block",
          flexShrink: 0,
          width: size,
          height: size,
          background: artwork.tint,
          maskImage: `url("${artwork.url}")`,
          WebkitMaskImage: `url("${artwork.url}")`,
          maskPosition: "center",
          maskRepeat: "no-repeat",
          maskSize: "contain",
        }}
      />
    );
  return (
    <img
      src={artwork.url}
      alt={source}
      width={size}
      height={size}
      draggable={false}
      style={{ width: size, height: size, objectFit: "contain", flexShrink: 0 }}
    />
  );
}
export default HUDArtwork;
