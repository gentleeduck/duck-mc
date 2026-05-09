import type { ReactNode } from "react";
import Link from "next/link";
import "./globals.css";

export const metadata = {
	title: "dmc x Next.js - duck-ui style",
	description: "Per-package collections + React MDX runtime — velite drop-in.",
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
					<span className="badge">dmc</span>
					<Link href="/">home</Link>
					<Link href="/duck-ui/introduction">duck-ui</Link>
					<Link href="/duck-ui/components/accordion">accordion</Link>
					<Link href="/duck-hooks/use-toggle">use-toggle</Link>
					<span
						style={{ marginLeft: "auto", color: "var(--muted)", fontSize: 13 }}
					>
						per-package collections + React MDX runtime
					</span>
				</nav>
				{children}
			</body>
		</html>
	);
}
