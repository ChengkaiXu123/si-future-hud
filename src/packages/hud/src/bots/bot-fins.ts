import type { BotSpec, Fin } from "./bot-spec";
/** BotFins.path: tangent discs, in body radii with y running down. */
export function finPath(fin: Fin, grown: number, raised: number): Path2D {
  const angle = ((fin.angle - fin.hand * raised) * Math.PI) / 180,
    along = [Math.cos(angle), Math.sin(angle)],
    across = [Math.sin(angle), -Math.cos(angle)],
    length = fin.length * grown,
    a = fin.root,
    ra = fin.base * grown,
    rb = fin.tip * grown,
    b = [
      a[0] + along[0] * (length - rb) + across[0] * fin.sweep * grown,
      a[1] + along[1] * (length - rb) + across[1] * fin.sweep * grown,
    ],
    dx = b[0] - a[0],
    dy = b[1] - a[1],
    d = Math.max(1e-6, Math.hypot(dx, dy)),
    s = Math.max(-1, Math.min(1, (ra - rb) / d)),
    c = Math.sqrt(1 - s * s),
    toward = Math.atan2(dy, dx),
    spread = Math.atan2(c, s),
    path = new Path2D();
  let empty = true;
  const arc = (
    center: number[],
    radius: number,
    start: number,
    end: number,
  ) => {
    const count = Math.max(2, Math.ceil((end - start) / (Math.PI / 16)));
    for (let i = 0; i <= count; i++) {
      const a = start + ((end - start) * i) / count,
        x = center[0] + radius * Math.cos(a),
        y = center[1] + radius * Math.sin(a);
      if (empty) {
        path.moveTo(x, y);
        empty = false;
      } else path.lineTo(x, y);
    }
  };
  arc(b, rb, toward - spread, toward + spread);
  arc(a, ra, toward + spread, toward + 2 * Math.PI - spread);
  path.closePath();
  return path;
}
export function finSilhouette(spec: BotSpec): Path2D | null {
  const path = new Path2D();
  let any = false;
  for (const item of spec.decor ?? [])
    if (item.kind === "fins")
      for (const fin of item.fins) {
        path.addPath(
          finPath(fin, 1, 0),
          new DOMMatrix().translate(...spec.center).scale(spec.radius),
        );
        any = true;
      }
  return any ? path : null;
}
