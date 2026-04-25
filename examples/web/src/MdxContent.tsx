import { Fragment, useMemo, type ComponentType } from 'react'
import * as runtime from 'react/jsx-runtime'

type RuntimeArg = {
  Fragment: typeof Fragment
  jsx: typeof runtime.jsx
  jsxs: typeof runtime.jsxs
}

const IMPORT_RE = /^\s*import\s+(?:\{([^}]+)\}|([A-Za-z_$][\w$]*))\s+from\s+['"][^'"]+['"];?\s*$/gm

function compileMdx(code: string) {
  const names: string[] = []
  const stripped = code.replace(IMPORT_RE, (_full, named, def) => {
    if (named) {
      named
        .split(',')
        .map((s: string) => s.trim().split(/\s+as\s+/)[1] ?? s.trim().split(/\s+as\s+/)[0])
        .filter(Boolean)
        .forEach((n: string) => names.push(n))
    } else if (def) {
      names.push(def)
    }
    return ''
  })

  const header = names.length
    ? `const __c = arguments[1] || {}; const { ${names.join(', ')} } = __c;\n`
    : ''

  // body declares `function _createMdxContent` and ends with `return _createMdxContent(arguments[0])`
  // we wrap it so `new Function(...)` returns the React tree.
  return new Function(header + stripped) as (
    rt: RuntimeArg,
    components: Record<string, ComponentType<unknown>>,
  ) => unknown
}

export function MdxContent({
  code,
  components,
}: {
  code: string
  components: Record<string, ComponentType<any>>
}) {
  const tree = useMemo(() => {
    const fn = compileMdx(code)
    return fn(
      { Fragment, jsx: runtime.jsx, jsxs: runtime.jsxs },
      components,
    )
  }, [code, components])

  return <>{tree as React.ReactNode}</>
}
