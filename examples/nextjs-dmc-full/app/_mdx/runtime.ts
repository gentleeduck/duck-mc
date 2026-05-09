"use client";

import type { ComponentType, ReactNode } from "react";
import * as runtime from "react/jsx-runtime";

export type MdxComponentMap = Record<string, ComponentType<any>>;

export type CompiledMdx = ComponentType<{ components: MdxComponentMap }>;

/**
 * Compile a dmc-emitted MDX body string into a React component.
 *
 * The body the rust codegen emits is a self-contained function expression
 * that expects `arguments[0]` to look like `react/jsx-runtime` AND carry
 * a `.components` property (the MDX component map). We merge the
 * consumer's component map onto the runtime object on every render so
 * each `<Mdx>` invocation gets the right components and capital JSX
 * names resolve through `_components.Foo || _missingMdxReference("Foo")`.
 */
export function useMDXComponent(code: string): CompiledMdx {
  const fn = new Function(code) as (arg: unknown) => ReactNode;
  return ((props: { components: MdxComponentMap }) =>
    fn({ ...runtime, components: props.components })) as CompiledMdx;
}
