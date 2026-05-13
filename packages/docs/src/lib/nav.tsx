import type { LinkItemType } from 'fumadocs-ui/layouts/shared';
import type { ReactNode } from 'react';

export const navTitle: ReactNode = (
  <span className="flex items-center gap-2 text-sm tracking-tight mr-4">
    <img
      src="/img/logo.dark.png"
      alt="HappyView"
      className="h-14 hidden dark:block"
    />
    <img
      src="/img/logo.light.png"
      alt="HappyView"
      className="h-14 block dark:hidden"
    />
  </span>
);

export const navLinks: LinkItemType[] = [
  {
    text: 'Docs',
    url: '/',
    active: 'nested-url',
  },
  {
    text: 'Blog',
    url: '/blog',
    active: 'nested-url',
  },
  {
    text: 'Source',
    url: 'https://tangled.org/gamesgamesgamesgames.games/happyview',
    external: true,
  },
];
