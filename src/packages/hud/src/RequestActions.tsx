import { useEffect, useRef, useState } from "react";
import type { Approval, RequestDecision } from "@agent-hud/protocol";
import { PermissionPlanCard } from "./permissions/PermissionPlanCard";
import { PermissionQuestionCard } from "./permissions/PermissionQuestionCard";
import { PermissionToolCard } from "./permissions/PermissionToolCard";
import { useGround } from "./ground";
import { alertText } from "./tokens";
import { cardColumn } from "./card-column";

export interface RequestActionsProps {
  request: Approval;
  onDecision?: (decision: RequestDecision) => void | Promise<void>;
  /** Settings, on a page such as "agent-plugins". */
  onOpenSettings?: (page: string) => void;
  light?: boolean;
  language?: string;
}

/**
 * Mirrors PermissionAlertViews.swift, PermissionPlanViews.swift, and PermissionQuestionViews.swift.
 * Preserves native capabilities and exact answer keys.
 */
export function RequestActions({
  request,
  onDecision,
  onOpenSettings,
  light = false,
  language,
}: RequestActionsProps) {
  const zh = language?.startsWith("zh") ?? false;
  const t = (en: string, cn: string) => (zh ? cn : en);
  const identity = `${request.id}:${JSON.stringify(request.questions ?? [])}`;
  const current = useRef(identity);
  current.current = identity;
  const pendingRef = useRef<string | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const ground = useGround(light);
  const errorColor = ground.seeThrough
    ? alertText("warning", ground)
    : "#e9a16d";

  useEffect(() => {
    pendingRef.current = null;
    setPending(false);
    setError(null);
  }, [identity]);

  const answer = async (decision: RequestDecision) => {
    if (!onDecision || pendingRef.current === identity) return;
    const captured = identity;
    pendingRef.current = captured;
    setPending(true);
    setError(null);
    try {
      await onDecision(decision);
    } catch (failure) {
      if (current.current === captured)
        setError(failure instanceof Error ? failure.message : String(failure));
    } finally {
      if (current.current === captured) {
        pendingRef.current = null;
        setPending(false);
      }
    }
  };

  const disabled = !onDecision || pending;
  const isQuestion =
    request.kind === "question" || Boolean(request.questions?.length);
  const isPlan = request.kind === "plan";

  return (
    <div
      className="hud-request-actions"
      data-request-id={request.id}
      style={{ ...cardColumn(9), fontSize: 12 }}
    >
      {isQuestion ? (
        <PermissionQuestionCard
          key={identity}
          request={request}
          disabled={disabled}
          pending={pending}
          light={light}
          zh={zh}
          answer={answer}
          openSettings={onOpenSettings}
        />
      ) : isPlan ? (
        <PermissionPlanCard
          key={identity}
          request={request}
          disabled={disabled}
          light={light}
          zh={zh}
          answer={answer}
        />
      ) : (
        <PermissionToolCard
          key={identity}
          request={request}
          disabled={disabled}
          pending={pending}
          light={light}
          zh={zh}
          answer={answer}
        />
      )}
      {!onDecision && (
        <p style={{ margin: 0, fontSize: 11, opacity: 0.6 }}>
          {t("Approval delivery is not connected.", "尚未连接审批处理。")}
        </p>
      )}
      {error && (
        <p
          role="alert"
          style={{
            margin: 0,
            color: errorColor,
            fontSize: 11,
            lineHeight: 1.4,
          }}
        >
          {error}
        </p>
      )}
    </div>
  );
}
export default RequestActions;
