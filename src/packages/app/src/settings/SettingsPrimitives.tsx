import type { CSSProperties, ReactNode } from "react";
import type { NativeMethod, Snapshot } from "@agent-hud/protocol";
import { Button } from "@agent-hud/ui/components/button";
import { Slider } from "@agent-hud/ui/components/slider";
import { Switch } from "@agent-hud/ui/components/switch";
import { Tabs, TabsList, TabsTrigger } from "@agent-hud/ui/components/tabs";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@agent-hud/ui/components/select";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@agent-hud/ui/components/tooltip";
import type { HUDConfig } from "@agent-hud/hud/model";
import { snapshotLanguage } from "@agent-hud/ui/format";

export interface SettingsPaneProps {
  snapshot: Snapshot;
  active?: boolean;
  config: HUDConfig;
  update: (patch: Partial<HUDConfig>) => void;
  action: (
    method: NativeMethod,
    params?: Record<string, unknown>,
  ) => Promise<unknown>;
}

export type Text = (en: string, zh: string) => string;

export const secondary: CSSProperties = {
  color: "var(--muted-foreground)",
  lineHeight: 1.4,
};

const nativeUnavailable =
  "This native integration is not available in this version.";

export function useText(snapshot: Snapshot): Text {
  const chinese = snapshotLanguage(snapshot) === "zh";
  return (en, zh) => (chinese ? zh : en);
}

export function Section({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle?: string;
  children: ReactNode;
}) {
  return (
    <section
      className="source-settings-section"
      style={{ display: "grid", gap: 12 }}
    >
      <header style={{ display: "grid", gap: 5 }}>
        <h2
          style={{
            margin: 0,
            fontSize: 15,
            fontWeight: 600,
            lineHeight: "18px",
          }}
        >
          {title}
        </h2>
        {subtitle && (
          <p
            style={{
              ...secondary,
              margin: 0,
              fontSize: 12,
              whiteSpace: "pre-line",
            }}
          >
            {subtitle}
          </p>
        )}
      </header>
      <div
        className="source-settings-card"
        style={{
          borderRadius: 12,
          overflow: "hidden",
          background: "var(--settings-card, var(--card))",
          border: "1px solid var(--settings-card-border, var(--border))",
        }}
      >
        {children}
      </div>
    </section>
  );
}

export function Divider() {
  return (
    <div
      className="source-settings-divider"
      style={{ height: 1, marginInline: 16, background: "var(--border)" }}
    />
  );
}

export function Row({
  title,
  subtitle,
  tag,
  children,
}: {
  title: string;
  subtitle?: string;
  tag?: string;
  children: ReactNode;
}) {
  return (
    <div
      className="source-setting-row"
      style={{
        display: "flex",
        alignItems: "center",
        gap: 24,
        padding: "11px 16px",
        minHeight: 52,
      }}
    >
      <div style={{ flex: 1, minWidth: 0, display: "grid", gap: 4 }}>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 7,
            fontSize: 13,
            lineHeight: "17px",
          }}
        >
          {title}
          {tag && (
            <span
              style={{
                fontSize: 10,
                padding: "1px 5px",
                borderRadius: 4,
                color: "var(--muted-foreground)",
                background: "var(--muted)",
              }}
            >
              {tag}
            </span>
          )}
        </div>
        {subtitle && (
          <div style={{ ...secondary, fontSize: 11 }}>{subtitle}</div>
        )}
      </div>
      <div
        style={{ flexShrink: 0, display: "flex", alignItems: "center", gap: 8 }}
      >
        {children}
      </div>
    </div>
  );
}

export function Unavailable({
  children,
  reason = nativeUnavailable,
}: {
  children: ReactNode;
  reason?: string;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span tabIndex={0} style={{ display: "inline-flex" }}>
          {children}
        </span>
      </TooltipTrigger>
      <TooltipContent style={{ maxWidth: 280 }}>{reason}</TooltipContent>
    </Tooltip>
  );
}

export function ToggleRow({
  title,
  subtitle,
  checked,
  change,
  disabled,
  tag,
  reason,
}: {
  title: string;
  subtitle?: string;
  checked: boolean;
  change?: (value: boolean) => void;
  disabled?: boolean;
  tag?: string;
  reason?: string;
}) {
  const control = (
    <Switch
      aria-label={title}
      checked={checked}
      onCheckedChange={change}
      disabled={disabled}
    />
  );
  return (
    <Row title={title} subtitle={subtitle} tag={tag}>
      {disabled ? (
        <Unavailable reason={reason}>{control}</Unavailable>
      ) : (
        control
      )}
    </Row>
  );
}

export function Segments({
  label,
  value,
  options,
  change,
  disabled = false,
}: {
  label: string;
  value: string;
  options: [string, string][];
  change: (value: string) => void;
  disabled?: boolean;
}) {
  return (
    <Tabs
      value={value}
      onValueChange={change}
      className="source-settings-segments"
    >
      <TabsList
        aria-label={label}
        style={{ height: 26, padding: 2, gap: 0, borderRadius: 7 }}
      >
        {options.map(([key, name]) => (
          <TabsTrigger
            key={key}
            value={key}
            disabled={disabled}
            style={{
              height: 22,
              minHeight: 22,
              padding: "0 11px",
              fontSize: 12,
              borderRadius: 5,
              lineHeight: "20px",
              fontWeight: 400,
            }}
          >
            {name}
          </TabsTrigger>
        ))}
      </TabsList>
    </Tabs>
  );
}

export function Menu({
  label,
  value,
  options,
  change,
  disabled = false,
  width = 124,
}: {
  label: string;
  value: string;
  options: [string, string][];
  change?: (value: string) => void;
  disabled?: boolean;
  width?: number;
}) {
  const control = (
    <Select value={value} onValueChange={change} disabled={disabled}>
      <SelectTrigger
        aria-label={label}
        className="source-settings-select"
        size="sm"
        style={{
          width,
          height: 26,
          minHeight: 26,
          padding: "0 8px",
          fontSize: 12,
          borderRadius: 6,
        }}
      >
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {options.map(([key, name]) => (
          <SelectItem key={key} value={key} style={{ fontSize: 12 }}>
            {name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
  return disabled ? <Unavailable>{control}</Unavailable> : control;
}

export function Slide({
  title,
  value,
  min,
  max,
  step,
  change,
  format,
}: {
  title: string;
  value: number;
  min: number;
  max: number;
  step: number;
  change: (value: number) => void;
  format: (value: number) => string;
}) {
  return (
    <div
      className="source-slider-row"
      style={{
        display: "flex",
        alignItems: "center",
        gap: 12,
        minHeight: 52,
        padding: "11px 16px",
      }}
    >
      <label style={{ width: 150, flex: "0 0 150px", fontSize: 13 }}>
        {title}
      </label>
      <Slider
        aria-label={title}
        min={min}
        max={max}
        step={step}
        value={[value]}
        onValueChange={(values) => change(values[0])}
        style={{ flex: 1 }}
      />
      <output
        style={{
          width: 60,
          flex: "0 0 60px",
          textAlign: "right",
          fontSize: 13,
          fontVariantNumeric: "tabular-nums",
          color: "var(--muted-foreground)",
        }}
      >
        {format(value)}
      </output>
    </div>
  );
}

export function SmallButton({
  children,
  onClick,
  disabled = false,
}: {
  children: ReactNode;
  onClick?: () => void;
  disabled?: boolean;
}) {
  return (
    <Button
      variant="outline"
      size="sm"
      disabled={disabled}
      onClick={onClick}
      style={{
        height: 24,
        minHeight: 24,
        padding: "0 9px",
        fontSize: 12,
        fontWeight: 400,
        borderRadius: 6,
      }}
    >
      {children}
    </Button>
  );
}
