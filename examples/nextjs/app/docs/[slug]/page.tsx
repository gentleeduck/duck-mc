import docs from "../../../.gentleduck/doc.json";
import { notFound } from "next/navigation";

type Doc = (typeof docs)[number] & {
  title?: string;
  description?: string;
};

export function generateStaticParams() {
  return docs.map((d) => ({ slug: d.permalink }));
}

export default async function Page({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const doc = docs.find((d) => d.permalink === slug) as Doc | undefined;
  if (!doc) notFound();
  return (
    <article>
      <h1>{doc.title ?? doc.permalink}</h1>
      {doc.description && <p style={{ color: "#666" }}>{doc.description}</p>}
      <div dangerouslySetInnerHTML={{ __html: htmlOf(doc) }} />
    </article>
  );
}

function htmlOf(doc: Doc & { __compiled?: { html?: string } }) {
  return doc.__compiled?.html ?? "";
}
