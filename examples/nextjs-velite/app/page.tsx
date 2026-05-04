import { docs } from "../.velite";
import Link from "next/link";

export default function Home() {
  return (
    <main>
      <h1>velite x Next.js demo</h1>
      <p>
        These pages were compiled by <strong>velite</strong> running the JS
        plugin chain: <code>remark-gfm</code>, <code>remark-math</code>,
        <code>rehype-katex</code>, <code>remark-emoji</code>,
        <code>rehype-pretty-code</code> (shiki),
        <code>rehype-slug</code> + <code>rehype-autolink-headings</code>.
        It renders the same MDX as the dmc kitchen-sink app. Run the dmc
        example on :3000 to compare.
      </p>
      <h2>Pages</h2>
      <ul>
        {docs.map((d) => (
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
