import { useEffect, useRef, useState } from "react";
import type { NativeMethod } from "@agent-hud/protocol";
import { Button } from "@agent-hud/ui/components/button";

interface CliStatus {
  installable: boolean;
  defaultSettings: string;
  notice?: string | null;
}

export default function AntigravityAccounts({
  settings,
  zh,
  action,
}: {
  settings: string[];
  zh: boolean;
  action: (
    method: NativeMethod,
    params?: Record<string, unknown>,
  ) => Promise<unknown>;
}) {
  const saved = settings.join("\n");
  const [text, setText] = useState(saved);
  const [status, setStatus] = useState<CliStatus | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const latestAction = useRef(action);
  latestAction.current = action;
  useEffect(() => setText(saved), [saved]);
  useEffect(() => {
    let active = true;
    void latestAction.current("antigravity_cli_status").then(
      (value) => {
        if (active) setStatus(value as CliStatus);
      },
      (error) => {
        if (active) setError(String(error));
      },
    );
    return () => {
      active = false;
    };
  }, []);
  const t = (en: string, cn: string) => (zh ? cn : en);
  const paths = () => [
    ...new Set(
      text
        .split(/\r?\n/)
        .map((path) => path.trim())
        .filter(Boolean),
    ),
  ];

  async function save() {
    setPending(true);
    setError(null);
    try {
      await action("set_setting", {
        key: "antigravityCliSettings",
        value: paths(),
      });
    } catch (error) {
      setError(String(error));
    } finally {
      setPending(false);
    }
  }

  return (
    <section
      style={{
        border: "1px solid var(--border)",
        background: "var(--card)",
        borderRadius: 12,
        padding: 16,
      }}
    >
      <label htmlFor="antigravity-cli-settings" style={{ fontWeight: 600 }}>
        {t("Antigravity CLI accounts", "Antigravity CLI 账户")}
      </label>
      <p
        id="antigravity-cli-settings-help"
        style={{
          fontSize: 12,
          color: "var(--muted-foreground)",
          margin: "6px 0 12px",
        }}
      >
        {t(
          "Connect each CLI settings.json, one per line. Start or restart the CLI after connecting. Accounts appear as you use them, including after switching accounts. Inactive quotas are marked stale.",
          "填写各个 CLI 的 settings.json 路径，每行一个。连接后启动或重启 CLI，使用过的账户会陆续显示，切换账户后也会保留；不活跃账户的额度会标为过期。",
        )}
      </p>
      <textarea
        id="antigravity-cli-settings"
        aria-describedby="antigravity-cli-settings-help antigravity-cli-settings-restore"
        value={text}
        onChange={(event) => setText(event.target.value)}
        disabled={pending || !status?.installable}
        rows={3}
        spellCheck={false}
        placeholder="~/.gemini/antigravity-cli/settings.json"
        style={{
          width: "100%",
          boxSizing: "border-box",
          resize: "vertical",
          border: "1px solid var(--border)",
          borderRadius: 6,
          padding: 8,
          color: "inherit",
          background: "var(--background)",
          fontFamily: "monospace",
          fontSize: 12,
        }}
      />
      <p
        id="antigravity-cli-settings-restore"
        style={{
          fontSize: 12,
          color: "var(--muted-foreground)",
          margin: "6px 0 12px",
        }}
      >
        {t(
          "Connecting adds a local quota callback and preserves your status line. Remove a path and save to restore its original status line.",
          "连接会添加本地额度回调，并保留原有状态栏。移除路径并保存后，会恢复该配置的原有状态栏。",
        )}
      </p>
      {status?.notice && (
        <p style={{ fontSize: 12, color: "var(--muted-foreground)" }}>
          {status.notice}
        </p>
      )}
      {error && (
        <p role="alert" style={{ fontSize: 12, color: "var(--destructive)" }}>
          {error}
        </p>
      )}
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        <Button
          size="sm"
          variant="outline"
          disabled={
            pending ||
            !status?.installable ||
            paths().includes(status?.defaultSettings ?? "")
          }
          onClick={() =>
            setText([...paths(), status!.defaultSettings].join("\n"))
          }
        >
          {t("Add default configuration", "添加默认配置")}
        </Button>
        <Button
          size="sm"
          disabled={pending || !status?.installable || text === saved}
          onClick={() => void save()}
        >
          {pending
            ? t("Saving…", "正在保存…")
            : t("Connect accounts", "连接账户")}
        </Button>
      </div>
    </section>
  );
}
