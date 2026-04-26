# Lexer error samples

Each `.mdx` file at the top of this directory triggers ONE diagnostic from
`duck-md-lexer/src/diagnostic.rs`. The `dead-codes/` subdir contains samples
for codes that are *defined* but not yet *emitted* by the current lexer —
they exist so we can wire them up later without re-deriving the inputs.

## Run

```sh
cargo run -p duck-md-lexer --bin error                                 # all samples in this dir
cargo run -p duck-md-lexer --bin error -- E010-unterminated-code-fence.mdx
cargo run -p duck-md-lexer --bin error -- --tokens E009-bare-jsx-attribute.mdx
echo '`oops' | cargo run -p duck-md-lexer --bin error
```

## Codes that fire today

| File                                   | Code | Message                                       |
| -------------------------------------- | ---- | --------------------------------------------- |
| `E004-unclosed-link.mdx`               | E004 | unterminated link target                      |
| `E004-unclosed-jsx-expr.mdx`           | E004 | unterminated expression                       |
| `E009-bare-jsx-attribute.mdx`          | E009 | invalid jsx attribute                         |
| `E010-unterminated-code-fence.mdx`     | E010 | unterminated code block                       |
| `E010-unterminated-inline-code.mdx`    | E010 | unterminated inline code                      |

## Codes defined but never emitted (`dead-codes/`)

`E001` `InvalidCharacter`, `E002` `InvalidFrontMatter`, `E003` `UnterminatedString`,
`E005` `UnexpectedEof`, `E006` `InvalidJsxSelfClosingTag`, `E007` `UnterminatedJsxTag`,
`E008` `InvalidJsxClosingTag`, parser warning for unterminated JSX open tag.
The samples in `dead-codes/` are inputs that *should* trigger these codes once
the corresponding emit sites are added.
