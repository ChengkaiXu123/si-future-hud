import { useEffect, useRef, useState, type ReactNode } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import type { Snapshot } from "@agent-hud/protocol";
import { TooltipProvider } from "@agent-hud/ui/components/tooltip";
import { snapshotLanguage } from "@agent-hud/ui/format";
import {
  Divider,
  Row,
  Section,
  SmallButton,
  ToggleRow,
  secondary,
  useText,
} from "./SettingsPrimitives";
import {
  isUpdateEvent,
  lastChecked,
  megabytes,
  percent,
  pick,
  type UpdateView,
} from "../lib/updates";

/** Settings → Updates: SI Future HUD's own updater (src/crates/updater),
 * driven through the product extension's requests. */
export default function UpdatesPane({
  snapshot,
  request,
}: {
  snapshot: Snapshot;
  request: (method: string, params?: unknown) => Promise<unknown>;
}) {
  const t = useText(snapshot);
  const zh = snapshotLanguage(snapshot) === "zh";
  const [view, setView] = useState<UpdateView | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  // Settings makes a new request function every render; the page keeps one
  // subscription and reads through the latest.
  const send = useRef(request);
  send.current = request;
  useEffect(() => {
    let disposed = false;
    const read = () =>
      void send
        .current("updates.state")
        .then((value) => {
          if (disposed) return;
          setView(value as UpdateView);
          setFailure(null);
        })
        .catch((error) => {
          if (!disposed) setFailure(message(error));
        });
    read();
    const subscription = isTauri()
      ? listen<unknown>("product:event", ({ payload }) => {
          if (!disposed && isUpdateEvent(payload)) setView(payload.updates);
        })
      : null;
    // An event sent while Settings was hidden is missed; read again now and
    // then, and for the "minutes ago" line.
    const timer = window.setInterval(read, 15_000);
    return () => {
      disposed = true;
      window.clearInterval(timer);
      void subscription?.then((unlisten) => unlisten());
    };
  }, []);
  const run = async (method: string, params?: unknown) => {
    try {
      setView((await send.current(method, params)) as UpdateView);
    } catch (error) {
      toast.error(t("Update", "更新"), { description: message(error) });
    }
  };

  if (!view)
    return (
      <div className="connection-placeholder">
        {failure ?? t("Reading update status…", "正在读取更新状态…")}
      </div>
    );
  const available = view.available;
  const notes = available ? pick(available.notes, zh) : "";
  const statusLine = [
    lastChecked(view.lastCheckAt, zh),
    view.testFeed
      ? t(`Test feed: ${view.testFeed}`, `测试更新源：${view.testFeed}`)
      : "",
  ]
    .filter(Boolean)
    .join(" · ");
  return (
    <TooltipProvider delayDuration={300}>
      <div
        className="source-settings-pane"
        style={{
          width: "100%",
          maxWidth: 640,
          display: "flex",
          flexDirection: "column",
          gap: 28,
        }}
      >
        <Section title={t("Software update", "软件更新")}>
          <Row title={t("Current version", "当前版本")}>
            <span
              style={{
                ...secondary,
                fontSize: 13,
                fontVariantNumeric: "tabular-nums",
              }}
            >
              {view.currentVersion}
            </span>
          </Row>
          <Divider />
          <Row
            title={pick(view.status, zh)}
            subtitle={statusLine}
            tag={view.testFeed ? t("Test", "测试") : undefined}
          >
            <SmallButton
              disabled={!view.canCheck}
              onClick={() => void run("updates.check")}
            >
              {t("Check Now", "立即检查")}
            </SmallButton>
          </Row>
          {view.phase === "downloading" && (
            <Note>
              <div
                role="progressbar"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={percent(view.progress)}
                style={{
                  height: 4,
                  borderRadius: 2,
                  overflow: "hidden",
                  background: "var(--muted)",
                }}
              >
                <div
                  style={{
                    width: `${percent(view.progress)}%`,
                    height: "100%",
                    background: "var(--primary)",
                    transition: "width 200ms ease",
                  }}
                />
              </div>
            </Note>
          )}
          {view.error && <Note tone="error">{pick(view.error, zh)}</Note>}
          {view.blocked && <Note>{pick(view.blocked, zh)}</Note>}
          {available && (
            <>
              <Divider />
              <Row
                title={t(
                  `Version ${available.version}`,
                  `新版本 ${available.version}`,
                )}
                subtitle={[
                  megabytes(available.size),
                  available.pubDate
                    ? new Date(available.pubDate).toLocaleDateString(
                        zh ? "zh-CN" : "en-US",
                      )
                    : "",
                ]
                  .filter(Boolean)
                  .join(" · ")}
              >
                <SmallButton
                  disabled={!view.canInstall}
                  onClick={() => void run("updates.install")}
                >
                  {t("Install and Restart", "安装并重启")}
                </SmallButton>
              </Row>
            </>
          )}
          {view.lastInstall && (
            <>
              <Divider />
              <Note spaced tone={view.lastInstall.ok ? undefined : "error"}>
                {pick(view.lastInstall.message, zh)}
              </Note>
            </>
          )}
        </Section>
        <Section title={t("Automatic updates", "自动更新")}>
          <ToggleRow
            title={t("Check for updates automatically", "自动检查更新")}
            subtitle={t(
              "About a minute after launch, then every 6 hours.",
              "启动约 1 分钟后检查一次，之后每 6 小时一次。",
            )}
            checked={view.autoCheck}
            change={(value) =>
              void run("updates.set", { key: "autoCheck", value })
            }
          />
          <Divider />
          <ToggleRow
            title={t(
              "Install new versions automatically",
              "有新版本时自动安装",
            )}
            subtitle={t(
              "Once downloaded and verified, the update installs and SI Future HUD restarts when no card waits on the notch.",
              "下载并校验后，等刘海上没有待处理的卡片时自动安装并重启。",
            )}
            checked={view.autoInstall}
            change={(value) =>
              void run("updates.set", { key: "autoInstall", value })
            }
          />
        </Section>
        {available && notes && (
          <Section
            title={t(
              `What’s new in ${available.version}`,
              `${available.version} 更新说明`,
            )}
          >
            <div
              style={{
                padding: "12px 16px",
                fontSize: 13,
                lineHeight: 1.5,
                whiteSpace: "pre-wrap",
              }}
            >
              {notes}
            </div>
          </Section>
        )}
      </div>
    </TooltipProvider>
  );
}

/** A line under a row, in the same card. */
function Note({
  children,
  tone,
  spaced = false,
}: {
  children: ReactNode;
  tone?: "error";
  /** Its own row, not a line under one. */
  spaced?: boolean;
}) {
  return (
    <div
      style={{
        ...secondary,
        padding: spaced ? "11px 16px" : "0 16px 11px",
        fontSize: 11,
        ...(tone === "error" ? { color: "var(--destructive)" } : {}),
      }}
    >
      {children}
    </div>
  );
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
