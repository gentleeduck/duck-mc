import { notFound } from "next/navigation";
import duckUi from "../../.gentleduck/DuckUi.json";
import duckHooks from "../../.gentleduck/DuckHooks.json";
import { Mdx } from "../_mdx/mdx";

type Doc = {
  permalink: string;
  title?: string;
  description?: string;
  body?: string;
};

const all = [
  ...((duckUi as Doc[]).map((d) => ({ ...d, _src: "duck-ui" }))),
  ...((duckHooks as Doc[]).map((d) => ({ ...d, _src: "duck-hooks" }))),
];

const stripDir = (p: string) => p.replace(/^docs\//, "");

export function generateStaticParams() {
  return all.map((d) => ({ slug: stripDir(d.permalink).split("/") }));
}

export default async function Page({
  params,
}: {
  params: Promise<{ slug: string[] }>;
}) {
  const { slug } = await params;
  const path = slug.join("/");
  const doc = all.find((d) => stripDir(d.permalink) === path);
  if (!doc) notFound();
  return (
    <article>
      <h1>{doc.title ?? doc.permalink}</h1>
      {doc.description && (
        <p style={{ color: "#666" }}>{doc.description}</p>
      )}
      {doc.body ? <Mdx code={doc.body} /> : null}
    </article>
  );
}
