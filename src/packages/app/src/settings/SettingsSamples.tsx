import { useFormat } from "@agent-hud/ui/format-context";
import { quotaPercent, quotaPercentLabel } from "@agent-hud/ui/format";
import settingsWallpaper from "@agent-hud/ui/assets/settings-wallpaper.png";
import {
  DollarSign,
  Flame,
  Gauge,
  MessageCircleQuestion,
  Percent,
  Sigma,
} from "lucide-react";
import type { Snapshot } from "@agent-hud/protocol";
import { Button } from "@agent-hud/ui/components/button";
import { HUDArtwork as HUDLogo } from "@agent-hud/hud/HUDArtwork";
import { BotMark } from "@agent-hud/hud/BotMark";
import { HUDGlow } from "@agent-hud/hud/HUDGlow";
import {
  agentVendorName,
  defaultHUDConfig,
  meterLabel,
  statusColor,
  vendorsFromSnapshot,
  type HUDConfig,
} from "@agent-hud/hud/model";
import { islandPath, stripMetrics } from "@agent-hud/hud/geometry";
import {
  Divider,
  Row,
  type SettingsPaneProps,
  type Text,
} from "./SettingsPrimitives";

const originalWallpaper = settingsWallpaper;

/** The same short wallpaper samples as SettingsPreview; these never open a panel. */
export function SmallHUDPreview({
  snapshot,
  config,
  active,
}: {
  snapshot: Snapshot;
  config: HUDConfig;
  active: boolean;
}) {
  const sample = { ...defaultHUDConfig, ...config, edge: "top" as const };
  const vendors = vendorsFromSnapshot(snapshot, sample.mode);
  const hasActiveVendor = vendors.some(
    (vendor) => vendor.working > 0 || vendor.waiting > 0,
  );
  const metrics = stripMetrics(sample, vendors, snapshot.settings.quotaDisplay);
  const simple = sample.mode === "simple";
  const light =
    sample.appearance === "light" ||
    (sample.appearance === "system" &&
      !matchMedia("(prefers-color-scheme: dark)").matches);
  const alpha = 1 - sample.solidTransparency;
  const surface = light ? `rgba(245,245,247,${alpha})` : `rgba(0,0,0,${alpha})`;
  return (
    <div style={{ padding: 12 }}>
      <div
        className="source-settings-preview"
        aria-hidden="true"
        style={{
          position: "relative",
          height: simple ? 84 : 112,
          borderRadius: 8,
          overflow: "hidden",
          pointerEvents: "none",
          backgroundImage: `url("${originalWallpaper}")`,
          backgroundSize: "cover",
          backgroundPosition: "center",
        }}
      >
        {active && (
          <div
            style={{
              position: "absolute",
              left: "50%",
              top: simple ? "50%" : 0,
              transform: simple ? "translate(-50%, -50%)" : "translateX(-50%)",
              width: metrics.width,
              height: metrics.height,
            }}
          >
            {!simple && (
              <HUDGlow
                outwardOnly={sample.glowOutwardOnly}
                width={metrics.width}
                height={metrics.height}
                radius={metrics.radius}
                attached={sample.mode === "notch"}
                style={sample.glowStyle}
                effect={sample.glowEffect}
                colors={vendors
                  .filter((vendor) => vendor.used !== null)
                  .map((vendor) => statusColor(vendor.level))}
                active={hasActiveVendor}
                period={sample.breathSeconds}
                amplitude={sample.breathAmplitude}
                brightness={sample.glowBrightness}
                range={sample.glowRange}
                blur={sample.glowBlur}
                pitch={sample.glowGridPitch}
                core={sample.glowGridCore}
                fade={sample.glowGridFade}
                density={sample.glowGridDensity}
                reduceMotion={!sample.animations}
              />
            )}
            <div
              style={{
                position: "absolute",
                inset: 0,
                display: "flex",
                alignItems: "center",
                gap: metrics.gap,
                padding: simple
                  ? `${metrics.padding.top}px ${metrics.padding.right}px ${metrics.padding.bottom}px ${metrics.padding.left}px`
                  : 0,
                borderRadius: simple ? metrics.radius : "0 0 10px 10px",
                background: simple ? surface : "transparent",
                color: light ? "#161618" : "#f2f2f5",
                boxShadow: simple
                  ? `0 1px 3px rgba(0,0,0,${0.133 * alpha})`
                  : undefined,
              }}
            >
              {sample.mode === "notch" && (
                <svg
                  width={metrics.width + 16}
                  height={metrics.height}
                  style={{
                    position: "absolute",
                    left: -8,
                    top: 0,
                    overflow: "visible",
                  }}
                >
                  <path
                    d={islandPath(
                      metrics.width + 16,
                      metrics.height,
                      8,
                      metrics.radius,
                    )}
                    fill={sample.hasNotch ? "#000" : surface}
                  />
                </svg>
              )}
              {sample.mode !== "notch" &&
                (sample.mode !== "logos" || sample.showsLogos) &&
                vendors.map((vendor, index) => (
                  <div
                    key={vendor.key}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: metrics.labelGap,
                      flexShrink: 0,
                    }}
                  >
                    <span
                      style={{
                        position: "relative",
                        width: metrics.mark,
                        height: metrics.mark,
                        display: "inline-grid",
                        placeItems: "center",
                      }}
                    >
                      {simple && sample.showsRings && (
                        <svg
                          width={metrics.mark}
                          height={metrics.mark}
                          style={{
                            position: "absolute",
                            inset: 0,
                            transform: "rotate(-90deg)",
                          }}
                        >
                          <circle
                            cx={metrics.mark / 2}
                            cy={metrics.mark / 2}
                            r={(metrics.mark - sample.ringWidth) / 2}
                            fill="none"
                            stroke={light ? "#00000010" : "#ffffff18"}
                            strokeWidth={sample.ringWidth}
                          />
                          <circle
                            cx={metrics.mark / 2}
                            cy={metrics.mark / 2}
                            r={(metrics.mark - sample.ringWidth) / 2}
                            fill="none"
                            pathLength={100}
                            stroke={
                              (quotaPercent(
                                vendor.used,
                                snapshot.settings.quotaDisplay,
                              ) ?? 0) <= 0
                                ? "transparent"
                                : statusColor(vendor.level)
                            }
                            strokeWidth={sample.ringWidth}
                            strokeLinecap="round"
                            strokeDasharray={`${quotaPercent(vendor.used, snapshot.settings.quotaDisplay) ?? 0} 100`}
                          />
                        </svg>
                      )}
                      <BotMark
                        source={vendor.name}
                        size={metrics.logo}
                        light={light}
                        enabled={sample.botLogos}
                        state={
                          vendor.waiting > 0
                            ? "asking"
                            : vendor.working > 0 ||
                                (!hasActiveVendor && index === 0)
                              ? "working"
                              : "idle"
                        }
                      />
                    </span>
                    {simple && sample.showsNumbers && (
                      <span
                        style={{
                          fontSize: metrics.font,
                          color: statusColor(vendor.level),
                          fontVariantNumeric: "tabular-nums",
                        }}
                      >
                        {meterLabel(vendor, snapshot.settings.quotaDisplay)}
                      </span>
                    )}
                  </div>
                ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export function PartSample({
  kind,
  enabled,
  config,
  t,
}: {
  kind: "sessions" | "quota" | "tokens";
  enabled: boolean;
  config: HUDConfig;
  t: Text;
}) {
  const light =
    config.appearance === "light" ||
    (config.appearance === "system" &&
      !matchMedia("(prefers-color-scheme: dark)").matches);
  const { quotaDisplay } = useFormat();
  const amounts = [4, 8, 5, 11, 16, 9, 6, 13, 20, 14, 10, 7];
  return (
    <div
      aria-hidden="true"
      className="source-island-part-sample"
      style={{
        margin: "0 12px 12px",
        padding: "0 14px",
        height: kind === "sessions" ? 56 : 64,
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        gap: kind === "sessions" ? 8 : 10,
        borderRadius: 9,
        background: light ? "#f5f5f7" : "#000",
        color: light ? "#1d1d1f" : "#f5f5f7",
        opacity: enabled ? 1 : 0.25,
        pointerEvents: "none",
        fontSize: 11,
      }}
    >
      {kind === "sessions" ? (
        <>
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <strong style={{ fontWeight: 600 }}>{t("Sessions", "会话")}</strong>
            <span style={{ opacity: 0.55 }}>
              {t("1 running", "1 个运行中")}
            </span>
          </div>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              opacity: 0.65,
            }}
          >
            <span style={{ position: "relative", width: 14, height: 14 }}>
              <HUDLogo source="Claude" size={14} light={light} />
              <i
                style={{
                  position: "absolute",
                  width: 6,
                  height: 6,
                  background: "#34c759",
                  borderRadius: "50%",
                  bottom: -2,
                  right: -2,
                  boxShadow: `0 0 0 1.5px ${light ? "#f5f5f7" : "#000"}`,
                }}
              />
            </span>
            <span>{t("Fix sign-in issue", "修复登录问题")}</span>
            <span style={{ flex: 1 }} />
            <span
              style={{
                display: "flex",
                alignItems: "center",
                gap: 3,
                color: "#e6b94d",
                padding: "3px 6px",
                borderRadius: 20,
                background: "#e6b94d24",
                fontSize: 9,
              }}
            >
              <MessageCircleQuestion size={9} />1
            </span>
            <span>60k tok</span>
          </div>
        </>
      ) : kind === "quota" ? (
        <>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              opacity: 0.7,
            }}
          >
            <HUDLogo source="Claude" size={12} light={light} />
            <strong style={{ fontWeight: 600 }}>Claude</strong>
            <span style={{ flex: 1 }} />
            {[Percent, Flame, Sigma, Gauge, DollarSign].map((Icon, index) => (
              <span
                key={index}
                style={{
                  display: "inline-grid",
                  placeItems: "center",
                  width: 20,
                  height: 18,
                  borderRadius: 5,
                  background: index === 0 ? "#ffffff12" : "transparent",
                  opacity: index === 0 ? 1 : 0.5,
                }}
              >
                <Icon size={10} />
              </span>
            ))}
          </div>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 10,
              fontSize: 12,
            }}
          >
            <i
              style={{
                width: 7,
                height: 7,
                borderRadius: "50%",
                background: "#34c759",
              }}
            />
            <strong style={{ fontWeight: 600 }}>{t("Session", "会话")}</strong>
            <span
              style={{
                flex: 1,
                height: 4,
                background: light ? "#0001" : "#fff1",
                borderRadius: 5,
              }}
            >
              <i
                style={{
                  display: "block",
                  width: `${quotaPercent(28, quotaDisplay)}%`,
                  height: 4,
                  borderRadius: 5,
                  background: "#34c759",
                }}
              />
            </span>
            <strong style={{ color: "#34c759", fontWeight: 600 }}>
              {quotaPercentLabel(28, quotaDisplay)}
            </strong>
            {config.resetCountdown && (
              <span
                style={{
                  width: 78,
                  textAlign: "right",
                  opacity: 0.55,
                  fontSize: 11,
                }}
              >
                {t("2h 14m", "2 小时 14 分")}
              </span>
            )}
          </div>
        </>
      ) : (
        <>
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <strong style={{ fontWeight: 600 }}>
              {t("Tokens", "Token 消耗")}
            </strong>
            <span style={{ opacity: 0.55 }}>
              {t("1hr · 46M tok", "1 小时 · 46M tok")}
            </span>
          </div>
          <div
            style={{ display: "flex", alignItems: "end", gap: 4, height: 30 }}
          >
            {Array.from({ length: 24 }, (_, i) => (
              <div
                key={i}
                style={{
                  flex: 1,
                  display: "flex",
                  flexDirection: "column",
                  justifyContent: "end",
                  height: `${((amounts[i % 12] + amounts[(i + 4) % 12]) / 38) * 100}%`,
                  minHeight: 2,
                }}
              >
                <i
                  style={{
                    background: "#3aa0ef",
                    height: `${(amounts[(i + 4) % 12] / (amounts[i % 12] + amounts[(i + 4) % 12])) * 100}%`,
                  }}
                />
                <i style={{ background: "#20b9a8", flex: 1 }} />
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}

export function PanelOrderRows({
  snapshot,
  config,
  update,
  t,
}: Pick<SettingsPaneProps, "snapshot" | "config" | "update"> & { t: Text }) {
  const sectionNames = {
    blocks: t("Quota details", "额度明细"),
    tokens: t("Token usage", "Token 消耗"),
    sessions: t("Session list", "会话列表"),
  };
  const sections = [
    ...new Set([...config.panelSections, ...defaultHUDConfig.panelSections]),
  ];
  const blocks = [
    ...new Map([
      ...snapshot.quotas.map(
        (row) =>
          [
            row.vendor,
            { id: row.vendor, title: agentVendorName(row.vendor) },
          ] as const,
      ),
      ...snapshot.billing.map(
        (account) =>
          [
            `api:${account.vendor}`,
            {
              id: `api:${account.vendor}`,
              title: `${agentVendorName(account.vendor)} · ${t("Balance", "余额")}`,
            },
          ] as const,
      ),
    ]).values(),
  ];
  const allBlockKeys = [
    ...new Set([...config.panelOrder, ...blocks.map((block) => block.id)]),
  ];
  const orderedBlocks = allBlockKeys.flatMap((key) => {
    const block = blocks.find((item) => item.id === key);
    return block ? [block] : [];
  });
  const moveSection = (index: number, by: number) => {
    const next = [...sections];
    [next[index], next[index + by]] = [next[index + by], next[index]];
    update({ panelSections: next });
  };
  const moveBlock = (index: number, by: number) => {
    const next = [...allBlockKeys],
      from = next.indexOf(orderedBlocks[index].id),
      to = next.indexOf(orderedBlocks[index + by].id);
    [next[from], next[to]] = [next[to], next[from]];
    update({ panelOrder: next });
  };
  const buttons = (
    name: string,
    index: number,
    count: number,
    move: (index: number, by: number) => void,
  ) => (
    <>
      <Button
        variant="outline"
        size="icon-sm"
        disabled={index === 0}
        aria-label={t(`Move ${name} up`, `上移${name}`)}
        title={t("Move up", "上移")}
        onClick={() => move(index, -1)}
        style={{ width: 24, height: 24, minHeight: 24 }}
      >
        ↑
      </Button>
      <Button
        variant="outline"
        size="icon-sm"
        disabled={index === count - 1}
        aria-label={t(`Move ${name} down`, `下移${name}`)}
        title={t("Move down", "下移")}
        onClick={() => move(index, 1)}
        style={{ width: 24, height: 24, minHeight: 24 }}
      >
        ↓
      </Button>
    </>
  );
  return (
    <>
      <Divider />
      {sections.map((section, index) => (
        <Row
          key={section}
          title={`${index + 1}. ${sectionNames[section]}`}
          subtitle={
            index === 0
              ? t(
                  "Section order on this display. Hidden sections keep their place.",
                  "这块屏幕的内容顺序；隐藏的内容会保留原位置。",
                )
              : undefined
          }
        >
          {buttons(sectionNames[section], index, sections.length, moveSection)}
        </Row>
      ))}
      {orderedBlocks.length > 1 && (
        <>
          <Divider />
          {orderedBlocks.map((block, index) => (
            <Row
              key={block.id}
              title={block.title}
              subtitle={
                index === 0
                  ? t(
                      "Agent block order on this display",
                      "这块屏幕的 Agent 明细顺序",
                    )
                  : undefined
              }
            >
              {buttons(block.title, index, orderedBlocks.length, moveBlock)}
            </Row>
          ))}
        </>
      )}
    </>
  );
}
