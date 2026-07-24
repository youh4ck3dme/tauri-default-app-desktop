import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { useUIStore } from '@/store/ui-store'

interface MainWindowContentProps {
  children?: React.ReactNode
  className?: string
}

export function MainWindowContent({
  children,
  className,
}: MainWindowContentProps) {
  const { t } = useTranslation()
  const lastQuickPaneEntry = useUIStore(state => state.lastQuickPaneEntry)

  return (
    <div className={cn('flex h-full flex-col bg-background', className)}>
      {children || (
        <div className="flex flex-1 flex-col items-center justify-center">
          <h1 className="text-4xl font-bold text-foreground">
            {lastQuickPaneEntry
              ? t('mainWindow.lastEntry', { entry: lastQuickPaneEntry })
              : t('mainWindow.placeholder')}
          </h1>
        </div>
      )}
    </div>
  )
}
