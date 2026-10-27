import { useEffect, useState } from "react";
import { Button } from "@agent-hud/ui/components/button";

export default function CodexAccountHomes({
  homes,
  zh,
  onSave,
}: {
  homes: string[];
  zh: boolean;
  onSave: (homes: string[]) => Promise<unknown>;
}) {
  const saved = homes.join("\n");
  const [text, setText] = useState(saved);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => setText(saved), [saved]);
  const t = (en: string, cn: string) => (zh ? cn : en);

  async function save() {
    setPending(true);
    setError(null);
    const next = [
      ...new Set(
        text
          .split(/\r?\n/)
          .map((path) => path.trim())
          .filter(Boolean),
      ),
    ];
    try {
      await onSave(next);
      setText(next.join("\n"));
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
      <label htmlFor="codex-account-homes" style={{ fontWeight: 600 }}>
        {t("Additional Codex accounts", "更多 Codex 账户")}
      </label>
      <p
        id="codex-account-homes-help"
        style={{
          fontSize: 12,
          color: "var(--muted-foreground)",
          margin: "6px 0 12px",
        }}
      >
        {t(
          "Add the CODEX_HOME folder for each signed-in account, one per line. Use an absolute path or ~/. Your default account stays included.",
          "填写各个已登录账户的 CODEX_HOME 文件夹，每行一个，使用绝对路径或 ~/。默认账户会继续显示。",
        )}
      </p>
      <textarea
        id="codex-account-homes"
        aria-describedby="codex-account-homes-help"
        value={text}
        onChange={(event) => setText(event.target.value)}
        disabled={pending}
        rows={3}
        spellCheck={false}
        placeholder="~/.codex-work"
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
      {error && (
        <p role="alert" style={{ fontSize: 12, color: "var(--destructive)" }}>
          {error}
        </p>
      )}
      <Button
        size="sm"
        disabled={pending || text === saved}
        onClick={() => void save()}
        style={{ marginTop: 10 }}
      >
        {pending ? t("Saving…", "正在保存…") : t("Save accounts", "保存账户")}
      </Button>
    </section>
  );
}
