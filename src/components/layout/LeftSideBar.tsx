import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ChevronRight, Globe, Mail, Network, Server } from 'lucide-react'
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  SidebarProvider,
} from '@/components/ui/sidebar'
import { cn } from '@/lib/utils'

interface LeftSideBarProps {
  className?: string
}

const websupportSubItems = [
  { key: 'domains', labelKey: 'sidebar.websupport.domains', icon: Globe },
  { key: 'email', labelKey: 'sidebar.websupport.email', icon: Mail },
  { key: 'dns', labelKey: 'sidebar.websupport.dns', icon: Network },
] as const

export function LeftSideBar({ className }: LeftSideBarProps) {
  const { t } = useTranslation()
  const [websupportOpen, setWebsupportOpen] = useState(true)

  return (
    <div
      className={cn('flex h-full flex-col border-r bg-background', className)}
    >
      <SidebarProvider className="min-h-0 flex-1 items-stretch">
        <Sidebar collapsible="none" className="h-full w-full">
          <SidebarContent>
            <SidebarGroup>
              <SidebarGroupContent>
                <SidebarMenu>
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      onClick={() => setWebsupportOpen(prev => !prev)}
                      aria-expanded={websupportOpen}
                    >
                      <Server />
                      <span>{t('sidebar.websupport')}</span>
                      <ChevronRight
                        className={cn(
                          'ml-auto transition-transform duration-200',
                          websupportOpen && 'rotate-90'
                        )}
                      />
                    </SidebarMenuButton>
                  </SidebarMenuItem>

                  {websupportOpen && (
                    <SidebarMenuSub>
                      {websupportSubItems.map(item => (
                        <SidebarMenuSubItem key={item.key}>
                          <SidebarMenuSubButton asChild>
                            <button type="button" className="w-full">
                              <item.icon />
                              <span>{t(item.labelKey)}</span>
                            </button>
                          </SidebarMenuSubButton>
                        </SidebarMenuSubItem>
                      ))}
                    </SidebarMenuSub>
                  )}
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>
          </SidebarContent>
        </Sidebar>
      </SidebarProvider>
    </div>
  )
}
