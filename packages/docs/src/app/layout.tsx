import './global.css';
import { RootProvider } from 'fumadocs-ui/provider/next';
import { HomeLayout } from 'fumadocs-ui/layouts/home';
import { fontVariables } from '@/lib/fonts';
import { ReducedMotionProvider } from '@/lib/reduced-motion';
import { navTitle, navLinks } from '@/lib/nav';
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
    <html lang="en" className={fontVariables} suppressHydrationWarning>
      <body>
        <RootProvider theme={{ defaultTheme: 'dark' }}>
          <ReducedMotionProvider>
            <HomeLayout
              nav={{ title: navTitle }}
              themeSwitch={{ enabled: false }}
              links={navLinks}
            >
              {children}
            </HomeLayout>
          </ReducedMotionProvider>
        </RootProvider>
      </body>
    </html>
  );
}
