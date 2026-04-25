import type { ReactNode } from 'react'
import './globals.css'

export const metadata = {
  title: 'acme/ui — docs',
  description: 'Component library docs powered by duck-md',
}

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  )
}
