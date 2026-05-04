import { docs } from "../../../.velite";
import { notFound } from "next/navigation";

export function generateStaticParams() {
  return docs.map((d) => ({ slug: d.permalink }));
}

export default async function Page({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const doc = docs.find((d) => d.permalink === slug);
  if (!doc) notFound();
  return (
    <article>
      <h1>{doc.title}</h1>
      {doc.description && <p style={{ color: "var(--muted)" }}>{doc.description}</p>}
      <div dangerouslySetInnerHTML={{ __html: doc.html }} />
    </article>
  );
}
