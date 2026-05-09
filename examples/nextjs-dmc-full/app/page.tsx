import duckUi from "../.gentleduck/DuckUi.json";
import duckHooks from "../.gentleduck/DuckHooks.json";
import Link from "next/link";

type Doc = { permalink: string; title?: string; description?: string };

const stripDir = (p: string) => p.replace(/^docs\//, "");

export default function Home() {
  const sections: Array<{ name: string; docs: Doc[] }> = [
    { name: "duck-ui", docs: duckUi as Doc[] },
    { name: "duck-hooks", docs: duckHooks as Doc[] },
  ];
  return (
    <main>
      <h1>dmc x Next.js — duck-ui style</h1>
      <p>
        Per-package collections, MDX bodies rendered through a React
        component map (compound <code>Tabs</code>, <code>Steps</code>,
        <code>Callout</code>, <code>ComponentPreview</code>, etc.). Drop-in
        replacement for a velite-driven docs site.
      </p>
      {sections.map((s) => (
        <section key={s.name}>
          <h2>{s.name}</h2>
          <ul>
            {s.docs.map((d) => (
              <li key={d.permalink}>
                <Link href={`/${stripDir(d.permalink)}`}>
                  {d.title ?? d.permalink}
                </Link>
                {d.description && (
                  <span
                    style={{ color: "var(--muted)", marginLeft: "0.5rem" }}
                  >
                    - {d.description}
                  </span>
                )}
              </li>
            ))}
          </ul>
        </section>
      ))}
    </main>
  );
}
