---
"@gentleduck/md": patch
---

`CompileOptions.mdxOutputFormat` is typed `"function-body" | "module"` instead of `string`, matching `content.outputFormat` on the build config. The compiler treats anything but `"module"` as function-body, so a misspelt value used to compile silently in the wrong format; now it is a type error.
