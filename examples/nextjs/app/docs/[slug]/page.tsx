import docs from "../../../.gentleduck/doc.json";
import { notFound } from "next/navigation";

type Doc = (typeof docs)[number] & {
  title?: string;
  description?: string;
};

// Strip the `docs/` directory prefix so URLs are `/docs/hello` rather
// than `/docs/docs%2Fhello`.
const stripDir = (p: string) => p.replace(/^docs\//, "");

export function generateStaticParams() {
  return docs.map((d) => ({ slug: stripDir(d.permalink) }));
}

export default async function Page({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const doc = docs.find((d) => stripDir(d.permalink) === slug) as Doc | undefined;
  if (!doc) notFound();
  return (
    <article>
      <h1>{doc.title ?? doc.permalink}</h1>
      {doc.description && <p style={{ color: "#666" }}>{doc.description}</p>}
      <div dangerouslySetInnerHTML={{ __html: htmlOf(doc) }} />
    </article>
  );
}

function htmlOf(doc: Doc & { html?: string }) {
  return doc.html ?? "";
}
