import { DocsLayout } from 'fumadocs-ui/layouts/docs';
import { source } from '@/lib/source';
import { SidebarFooter } from '@/components/sidebar-footer';
import type { ReactNode } from 'react';

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <DocsLayout
      tree={source.getPageTree()}
      sidebar={{ footer: <SidebarFooter key="sidebar-footer" /> }}
      searchToggle={{ enabled: false }}
      themeSwitch={{ enabled: false }}
    >
      {children}
    </DocsLayout>
  );
}
