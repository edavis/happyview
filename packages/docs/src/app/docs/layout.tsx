import { DocsLayout } from 'fumadocs-ui/layouts/docs';
import { source } from '@/lib/source';
import type { ReactNode } from 'react';

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <DocsLayout
      tree={source.getPageTree()}
      nav={{
        title: (
          <span className="flex items-center gap-2 text-sm tracking-tight">
            <img src="/img/logo.dark.png" alt="" className="h-5" />
          </span>
        ),
      }}
      links={[
        {
          text: 'Docs',
          url: '/docs',
          active: 'nested-url',
        },
        {
          text: 'Source',
          url: 'https://tangled.org/gamesgamesgamesgames.games/happyview',
          external: true,
        },
      ]}
    >
      {children}
    </DocsLayout>
  );
}
