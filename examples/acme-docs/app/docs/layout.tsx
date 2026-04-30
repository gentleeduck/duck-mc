import Link from "next/link";
import type { ReactNode } from "react";
import docs from "../../.gentleduck/doc.json";

type Doc = { permalink: string; title?: string; description?: string };

export default function DocsLayout({ children }: { children: ReactNode }) {
	const list = docs as Doc[];
	return (
		<div className="layout">
			<nav className="sidebar">
				<div className="brand">
					<span className="brand-mark" />
					acme/ui
				</div>
				<h2>docs</h2>
				<ul>
					{list.map((d) => (
						<li key={d.permalink}>
							<Link href={`/docs/${d.permalink}`}>
								{d.title ?? d.permalink}
							</Link>
						</li>
					))}
				</ul>
				<h2 style={{ marginTop: 32 }}>powered by</h2>
				<ul>
					<li>
						<Link href="https://github.com/gentleduck">dmc</Link>
					</li>
				</ul>
			</nav>
			<main className="docs">{children}</main>
		</div>
	);
}
