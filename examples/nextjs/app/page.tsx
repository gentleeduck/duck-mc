import docs from '../.gentleduck/doc.json'
import Link from 'next/link'

export default function Home() {
  return (
    <main>
      <h1>duck-md × Next.js demo</h1>
      <ul>
        {docs.map((d: { permalink: string; title?: string }) => (
          <li key={d.permalink}>
            <Link href={`/docs/${d.permalink}`}>{d.title ?? d.permalink}</Link>
          </li>
        ))}
      </ul>
    </main>
  )
}
