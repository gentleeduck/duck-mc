"use client";

import type { ReactNode, HTMLAttributes, ComponentProps } from "react";
import type { MdxComponentMap } from "./runtime";

// Minimal duck-ui-shaped component set. The compiled MDX body resolves
// every capital JSX name through this map (`_components.Foo`), so the
// docs renderer doesn't need a per-tag `<switch>` — adding a new
// component is just registering it here.

function Callout({
  title,
  tone = "info",
  icon,
  children,
}: {
  title?: string;
  tone?: "info" | "warning" | "danger";
  icon?: ReactNode;
  children?: ReactNode;
}) {
  const palette = {
    info: { bg: "#eaf3ff", border: "#1e66f5" },
    warning: { bg: "#fff7d6", border: "#df8e1d" },
    danger: { bg: "#fde2e4", border: "#d20f39" },
  }[tone];
  return (
    <aside
      style={{
        margin: "1rem 0",
        padding: "0.75rem 1rem",
        borderLeft: `4px solid ${palette.border}`,
        background: palette.bg,
        borderRadius: 6,
      }}
    >
      <div
        style={{
          display: "flex",
          gap: "0.5rem",
          alignItems: "center",
          fontWeight: 600,
        }}
      >
        {icon}
        <span>{title ?? tone}</span>
      </div>
      <div>{children}</div>
    </aside>
  );
}

function Tabs({
  defaultValue,
  children,
}: {
  defaultValue?: string;
  children?: ReactNode;
}) {
  return (
    <div data-tabs data-default-value={defaultValue} style={{ margin: "1rem 0" }}>
      {children}
    </div>
  );
}

function TabsList({ children }: { children?: ReactNode }) {
  return (
    <div
      role="tablist"
      style={{
        display: "flex",
        gap: "0.5rem",
        borderBottom: "1px solid #e4e4e7",
        paddingBottom: "0.25rem",
        marginBottom: "0.75rem",
      }}
    >
      {children}
    </div>
  );
}

function TabsTrigger({
  value,
  children,
}: {
  value?: string;
  children?: ReactNode;
}) {
  return (
    <button
      role="tab"
      data-value={value}
      style={{
        padding: "0.25rem 0.5rem",
        border: "1px solid #e4e4e7",
        borderRadius: 4,
        background: "white",
        cursor: "pointer",
      }}
    >
      {children}
    </button>
  );
}

function TabsContent({
  value,
  children,
}: {
  value?: string;
  children?: ReactNode;
}) {
  return (
    <div role="tabpanel" data-value={value}>
      {children}
    </div>
  );
}

function Steps({ children }: { children?: ReactNode }) {
  return (
    <ol
      data-steps
      style={{
        counterReset: "step",
        listStyle: "none",
        paddingLeft: 0,
        borderLeft: "2px solid #e4e4e7",
        margin: "1rem 0",
      }}
    >
      {children}
    </ol>
  );
}

function Step({ children }: { children?: ReactNode }) {
  return (
    <li
      data-step
      style={{
        position: "relative",
        marginBottom: "0.75rem",
        paddingLeft: "1.5rem",
      }}
    >
      <span
        style={{
          position: "absolute",
          left: "-0.6rem",
          top: 0,
          width: "1.25rem",
          height: "1.25rem",
          borderRadius: "50%",
          background: "#1e66f5",
          color: "white",
          fontSize: "0.75rem",
          display: "inline-flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        ●
      </span>
      {children}
    </li>
  );
}

function ComponentPreview({
  name,
  description,
}: {
  name?: string;
  description?: string;
  className?: string;
}) {
  return (
    <div
      data-component-preview={name}
      style={{
        border: "1px solid #e4e4e7",
        borderRadius: 6,
        padding: "1rem",
        margin: "1rem 0",
        background: "#fafafa",
      }}
    >
      <strong>preview</strong>
      <code style={{ marginLeft: "0.5rem" }}>{name}</code>
      {description && (
        <p style={{ color: "#666", margin: "0.25rem 0 0" }}>{description}</p>
      )}
    </div>
  );
}

function MermaidDiagram({ chart }: { chart?: string }) {
  return (
    <pre
      data-mermaid
      style={{
        background: "#f4f4f5",
        padding: "0.75rem",
        borderRadius: 6,
        whiteSpace: "pre",
        fontSize: 12,
      }}
    >
      {chart}
    </pre>
  );
}

function PackageManagerTabs({
  npm,
  yarn,
  pnpm,
  bun,
}: {
  npm?: string;
  yarn?: string;
  pnpm?: string;
  bun?: string;
}) {
  return (
    <div data-pm-tabs style={{ margin: "1rem 0" }}>
      {[
        ["npm", npm],
        ["pnpm", pnpm],
        ["yarn", yarn],
        ["bun", bun],
      ]
        .filter(([, v]) => Boolean(v))
        .map(([k, v]) => (
          <pre
            key={k}
            data-pm={k}
            style={{
              background: "#1e1e2e",
              color: "#cdd6f4",
              padding: "0.5rem 0.75rem",
              borderRadius: 4,
              margin: "0.25rem 0",
              fontSize: 13,
            }}
          >
            <span style={{ color: "#7c7f93" }}>{k}$ </span>
            {v}
          </pre>
        ))}
    </div>
  );
}

function Zap(props: HTMLAttributes<SVGElement>) {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      {...props}
    >
      <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
    </svg>
  );
}

// Native HTML overrides — match the velite/rehype-pretty-code envelope
// so consumer CSS already targeting [data-rehype-pretty-code-figure]
// keeps working. The codegen already emits these attribute names; the
// component map only needs to forward children through.
function Figure(props: ComponentProps<"figure">) {
  return <figure {...props} />;
}
function Figcaption(props: ComponentProps<"figcaption">) {
  return <figcaption {...props} />;
}
function Pre(props: ComponentProps<"pre">) {
  return <pre {...props} />;
}
function Code(props: ComponentProps<"code">) {
  return <code {...props} />;
}

export const mdxComponents: MdxComponentMap = {
  Callout,
  Tabs,
  TabsList,
  TabsTrigger,
  TabsContent,
  Steps,
  Step,
  ComponentPreview,
  MermaidDiagram,
  PackageManagerTabs,
  Zap,
  // Native tag overrides used by the pretty-code envelope.
  figure: Figure,
  figcaption: Figcaption,
  pre: Pre,
  code: Code,
};
