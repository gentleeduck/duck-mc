---
"@gentleduck/md": minor
---

Validate and extract embedded components at build time.

A collection may now declare `components`: a map of component name to schema. JSX in a body
matching one of those names has its props validated and emitted onto the record as
`{ name, props, line, column, parent }`, in document order and including nested elements — so a
consumer can read the interactive parts of a document as data without parsing MDX or running a
browser.

A braced expression is read as JSON first, so `node={1}` arrives as the number `1`. A component
that fails its schema is reported against its own line and kept out of the record.
