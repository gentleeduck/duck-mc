"use client";

import { mdxComponents } from "./components";
import { useMDXComponent } from "./runtime";

export function Mdx({ code }: { code: string }) {
  const Component = useMDXComponent(code);
  return (
    <div className="mdx">
      <Component components={mdxComponents} />
    </div>
  );
}
