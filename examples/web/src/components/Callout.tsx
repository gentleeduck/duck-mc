import type { PropsWithChildren } from "react";

type Type = "info" | "warn" | "error";

const palette: Record<Type, { bg: string; border: string }> = {
  info: { bg: "#eff6ff", border: "#3b82f6" },
  warn: { bg: "#fffbeb", border: "#f59e0b" },
  error: { bg: "#fef2f2", border: "#ef4444" },
};

export function Callout({
  type = "info",
  children,
}: PropsWithChildren<{ type?: Type }>) {
  const c = palette[type];
  return (
    <aside
      style={{
        background: c.bg,
        borderLeft: `4px solid ${c.border}`,
        padding: "12px 16px",
        margin: "16px 0",
        borderRadius: 4,
      }}
    >
      {children}
    </aside>
  );
}
