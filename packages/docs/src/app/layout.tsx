import './global.css';
import { RootProvider } from 'fumadocs-ui/provider/next';
import { fontVariables } from '@/lib/fonts';
import type { ReactNode } from 'react';

export const metadata = {
  metadataBase: new URL(
    `https://${process.env.VERCEL_PROJECT_PRODUCTION_URL ?? 'localhost:3000'}`,
  ),
  title: {
    template: '%s | HappyView',
    default: 'HappyView',
  },
  description: 'Lexicon-driven ATProto AppView',
  openGraph: {
    images: [{ url: '/img/og.png' }],
  },
  icons: [{ rel: 'icon', url: '/img/favicon.png' }],
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" className={`${fontVariables} dark`} suppressHydrationWarning>
      <body>
        <RootProvider>{children}</RootProvider>
      </body>
    </html>
  );
}
