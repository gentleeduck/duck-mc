import docs from "../.gentleduck/doc.json";
import Link from "next/link";

type Doc = { permalink: string; title?: string; description?: string };

export default function Home() {
  const list = docs as Doc[];
  return (
    <main>
      <h1>dmc x Next.js demo</h1>
      <p>
        These pages were compiled by <strong>dmc</strong> in process. Rust
        handles GFM, code highlighting, math, emoji, slug generation, and
        autolink before any JS plugin runs. The matching{" "}
        <code>nextjs-velite</code> app exists for comparison.
      </p>
      <h2>Pages</h2>
      <ul>
        {list.map((d) => (
          <li key={d.permalink}>
            <Link href={`/docs/${d.permalink}`}>{d.title ?? d.permalink}</Link>
            {d.description && (
              <span style={{ color: "var(--muted)", marginLeft: "0.5rem" }}>
                - {d.description}
              </span>
            )}
          </li>
        ))}
      </ul>
    </main>
  );
}
