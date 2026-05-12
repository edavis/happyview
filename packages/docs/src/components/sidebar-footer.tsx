'use client';

import { SidebarCollapseTrigger } from 'fumadocs-ui/components/sidebar/base';
import { ThemeSwitch } from 'fumadocs-ui/layouts/shared/slots/theme-switch';
import { SidebarIcon } from 'lucide-react';

export function SidebarFooter() {
  return (
    <div className="flex items-center">
      <ThemeSwitch className="p-0" />
      <div className="flex-1" />
      <SidebarCollapseTrigger className="inline-flex items-center justify-center rounded-md p-1.5 text-fd-muted-foreground hover:bg-fd-accent hover:text-fd-accent-foreground transition-colors max-md:hidden">
        <SidebarIcon className="size-4" />
      </SidebarCollapseTrigger>
    </div>
  );
}
