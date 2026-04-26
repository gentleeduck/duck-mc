import docs from "../../../.gentleduck/doc.json";
import { notFound } from "next/navigation";

type Doc = {
  permalink: string;
  title?: string;
  description?: string;
  html?: string;
  body?: string;
};

export function generateStaticParams() {
  return (docs as Doc[]).map((d) => ({ slug: d.permalink.split("/") }));
}

export default async function Page({
  params,
}: {
  params: Promise<{ slug: string[] }>;
}) {
  const { slug } = await params;
  const path = slug.join("/");
  const doc = (docs as Doc[]).find((d) => d.permalink === path);
  if (!doc) notFound();
  return (
    <article>
      <header>
        <h1>{doc.title ?? doc.permalink}</h1>
        {doc.description && <p className="lede">{doc.description}</p>}
      </header>
      <div dangerouslySetInnerHTML={{ __html: doc.html ?? "" }} />
    </article>
  );
}
