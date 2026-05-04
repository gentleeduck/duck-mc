import type { ReactNode } from "react";
import Link from "next/link";
import "./globals.css";
// KaTeX needs its own stylesheet for glyph spacing.
// Imported via CDN to keep the example install lean.

export const metadata = {
	title: "velite x Next.js - kitchen sink",
	description:
		"JS plugin chain (remark-gfm + remark-math + rehype-katex + ...) rendering MDX server-side.",
};

export default function RootLayout({ children }: { children: ReactNode }) {
	return (
		<html lang="en">
			<head>
				<link
					rel="stylesheet"
					href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css"
					integrity="sha384-nB0miv6/jRmo5UMMR1wu3Gz6NLsoTkbqJghGIsx//Rlm+ZU03BU6SQNC66uf4l5+"
					crossOrigin="anonymous"
				/>
			</head>
			<body>
				<nav className="site-nav">
					<span className="badge">velite</span>
					<Link href="/">home</Link>
					<Link href="/docs/kitchen-sink">kitchen-sink</Link>
					<span
						style={{ marginLeft: "auto", color: "var(--muted)", fontSize: 13 }}
					>
						same MDX - compare with the dmc app on :3000
					</span>
				</nav>
				{children}
			</body>
		</html>
	);
}
