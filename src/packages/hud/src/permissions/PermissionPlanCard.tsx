import { Fragment, useState } from "react";
import { ArrowUp, Check, CheckCheck, MessageSquare, X } from "lucide-react";
import type { Approval, RequestDecision } from "@agent-hud/protocol";
import { clientDisplayName } from "../IslandNews";
import { inlineMarkdown } from "@agent-hud/ui/text/inline-markdown";
import { alertFill, onAlertFill, permissionColors } from "../tokens";
import { useGround } from "../ground";
import { cardColumn } from "../card-column";

/** Whether the user is writing what a plan should change, and what they wrote
 * so far, by request: the island redraws its cards as the pointer comes and
 * goes (Open's PlanDraft). */
const planDrafts = new Map<string, { noting: boolean; feedback: string }>();

export function PermissionPlanCard({
  request,
  disabled,
  light = false,
  zh = false,
  answer,
}: {
  request: Approval;
  disabled: boolean;
  light?: boolean;
  zh?: boolean;
  answer: (decision: RequestDecision) => void | Promise<void>;
}) {
  const t = (en: string, cn: string) => (zh ? cn : en);
  const saved = planDrafts.get(request.id);
  const [noting, setNotingState] = useState(saved?.noting ?? false);
  const [feedback, setFeedbackState] = useState(saved?.feedback ?? "");
  const keep = (next: { noting: boolean; feedback: string }) =>
    planDrafts.set(request.id, next);
  const setNoting = (value: boolean) => {
    setNotingState(value);
    keep({ noting: value, feedback });
  };
  const setFeedback = (value: string) => {
    setFeedbackState(value);
    keep({ noting, feedback: value });
  };
  const sendBack = () => {
    planDrafts.delete(request.id);
    void answer({ kind: "keep_planning", message: feedback });
  };
  const canAnswerPlan = request.answersPlan === true;
  const vendor = request.vendor ?? clientDisplayName(request.source);

  const text = (alpha: number) =>
    light ? `rgba(0,0,0,${alpha})` : `rgba(255,255,255,${alpha})`;
  const stroke = (alpha: number) =>
    light ? `rgba(0,0,0,${alpha})` : `rgba(255,255,255,${alpha})`;
  const primary = text(0.92);
  const secondary = text(0.55);
  const inset = light ? "rgba(0,0,0,0.045)" : "rgba(255,255,255,0.055)";
  const ground = useGround(light);
  const controls = permissionColors(ground);
  const allow = controls.allowFill;
  const allowText = controls.alwaysText;
  const allowEdge = controls.alwaysEdge;
  const onAllow = controls.onAllow;
  const signal = alertFill("warning", ground);
  const onSignal = onAlertFill(ground);

  return (
    <div style={cardColumn(8)}>
      {(request.plan || request.detail || request.input) && (
        <div
          aria-label={t("Plan", "计划")}
          style={{
            maxHeight: 150,
            overflowY: "auto",
            padding: 9,
            borderRadius: 7,
            background: inset,
            fontSize: 11,
            lineHeight: 1.5,
            color: text(0.86),
            whiteSpace: "pre-wrap",
            overflowWrap: "anywhere",
            userSelect: "text",
          }}
        >
          {inlineMarkdown(
            request.plan || request.detail || request.input || "",
          ).map((line, index) => (
            <Fragment key={index}>
              {index > 0 && "\n"}
              {line.map((segment, part) =>
                segment.kind === "strong" ? (
                  <strong key={part} style={{ fontWeight: 600 }}>
                    {segment.text}
                  </strong>
                ) : segment.kind === "emphasis" ? (
                  <em key={part}>{segment.text}</em>
                ) : segment.kind === "code" ? (
                  <code
                    key={part}
                    style={{
                      fontFamily: "ui-monospace, monospace",
                      fontSize: 10.5,
                    }}
                  >
                    {segment.text}
                  </code>
                ) : segment.kind === "link" ? (
                  <span
                    key={part}
                    title={segment.href}
                    style={{ textDecoration: "underline" }}
                  >
                    {segment.text}
                  </span>
                ) : (
                  <Fragment key={part}>{segment.text}</Fragment>
                ),
              )}
            </Fragment>
          ))}
        </div>
      )}
      {canAnswerPlan ? (
        noting ? (
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <input
              type="text"
              aria-label={t("Plan feedback", "计划反馈")}
              placeholder={t(
                "What should change? (optional)",
                "要改什么（可不填）",
              )}
              value={feedback}
              disabled={disabled}
              onChange={(e) => setFeedback(e.target.value)}
              onKeyDown={(e) => {
                // Return while an input method is composing belongs to it.
                if (
                  e.key === "Enter" &&
                  !e.nativeEvent.isComposing &&
                  e.keyCode !== 229
                ) {
                  e.preventDefault();
                  sendBack();
                } else if (e.key === "Escape") {
                  setNoting(false);
                }
              }}
              style={{
                flex: 1,
                height: 24,
                padding: "0 8px",
                borderRadius: 6,
                border: `1px solid ${stroke(0.2)}`,
                background: inset,
                color: primary,
                fontSize: 11,
                fontFamily: "inherit",
                outline: 0,
              }}
            />
            <button
              type="button"
              disabled={disabled}
              onClick={() => setNoting(false)}
              style={{
                display: "inline-flex",
                alignItems: "center",
                height: 22,
                padding: "0 9px",
                borderRadius: 6,
                border: `1px solid ${stroke(0.18)}`,
                background: "transparent",
                color: secondary,
                fontSize: 11,
                fontWeight: 500,
                cursor: disabled ? "default" : "pointer",
              }}
            >
              {t("Back", "返回")}
            </button>
            <button
              type="button"
              disabled={disabled}
              onClick={() => sendBack()}
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 4,
                height: 22,
                padding: "0 9px",
                borderRadius: 6,
                border: 0,
                background: signal,
                color: onSignal,
                fontSize: 11,
                fontWeight: 500,
                cursor: disabled ? "default" : "pointer",
              }}
            >
              <ArrowUp size={9} strokeWidth={2.6} />
              {t("Send", "发送")}
            </button>
          </div>
        ) : (
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <button
              type="button"
              disabled={disabled}
              onClick={() => setNoting(true)}
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 4,
                height: 22,
                padding: "0 9px",
                borderRadius: 6,
                border: `1px solid ${stroke(0.18)}`,
                background: "transparent",
                color: secondary,
                fontSize: 11,
                fontWeight: 500,
                cursor: disabled ? "default" : "pointer",
              }}
            >
              <MessageSquare size={9} strokeWidth={2.6} />
              {t("Keep planning", "继续规划")}
            </button>
            <span style={{ flex: 1, minWidth: 8 }} />
            <button
              type="button"
              disabled={disabled}
              onClick={() =>
                void answer({ kind: "approve_plan", acceptEdits: true })
              }
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 4,
                height: 22,
                padding: "0 9px",
                borderRadius: 6,
                border: `1px solid ${allowEdge}`,
                background: "transparent",
                color: allowText,
                fontSize: 11,
                fontWeight: 500,
                cursor: disabled ? "default" : "pointer",
              }}
            >
              <CheckCheck size={9} strokeWidth={2.6} />
              {t("Approve, accept edits", "批准并自动接受编辑")}
            </button>
            <button
              type="button"
              disabled={disabled}
              onClick={() =>
                void answer({ kind: "approve_plan", acceptEdits: false })
              }
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 4,
                height: 22,
                padding: "0 9px",
                borderRadius: 6,
                border: 0,
                background: allow,
                color: onAllow,
                fontSize: 11,
                fontWeight: 500,
                cursor: disabled ? "default" : "pointer",
              }}
            >
              <Check size={9} strokeWidth={2.6} />
              {t("Approve", "批准")}
            </button>
          </div>
        )
      ) : (
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <span style={{ fontSize: 11, color: secondary }}>
            {t(
              `Review this plan in ${vendor}`,
              `请到 ${vendor} 里审批这份计划`,
            )}
          </span>
          <span style={{ flex: 1, minWidth: 8 }} />
          <button
            type="button"
            disabled={disabled}
            onClick={() => void answer({ kind: "leave" })}
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
              height: 22,
              padding: "0 9px",
              borderRadius: 6,
              border: `1px solid ${stroke(0.18)}`,
              background: "transparent",
              color: secondary,
              fontSize: 11,
              fontWeight: 500,
              cursor: disabled ? "default" : "pointer",
            }}
          >
            <X size={9} strokeWidth={2.6} />
            {t("Dismiss", "知道了")}
          </button>
        </div>
      )}
    </div>
  );
}
