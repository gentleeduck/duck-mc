# `code-import`

Resolves `<CodeImport src="...">` JSX nodes to fenced code blocks with the
referenced file's contents inlined. Lets MDX docs pull live source from
the repo without manual copy-paste.

- **Source:** `dmc-transform/src/builtin/code_import.rs`
- **Feature flag:** none
- **Config:** none

## Input

```mdx
<CodeImport
  src="examples/hello.rs"
  lang="rust"
  title="hello.rs"
  lines="5-12"
/>
```

## Output

A fenced code block with the resolved range, ready for
[`pretty-code`](./pretty-code.md) to highlight:

````text
```rust title="hello.rs"
fn greet(name: &str) {
    println!("hi, {name}");
}
```
````

## Attribute reference

| Attr | Required | Effect |
|---|---|---|
| `src` | yes | File path relative to the source `.mdx`. |
| `lang` | no | Override fence language. Defaults to file extension. |
| `title` | no | Forwarded to the resulting fence as `title="..."` meta. |
| `lines` | no | Range syntax `"5-12"` or `"5,9-12"` - slices the file before inlining. |

## Failure modes

- Missing file -> diagnostic; original `<CodeImport>` left in place.
- Invalid `lines` range -> diagnostic; full file inlined.
