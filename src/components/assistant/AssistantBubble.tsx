import { Sparkles } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { useUIStore } from '@/store/ui-store'

export function AssistantBubble() {
  const { t } = useTranslation()
  const toggleAssistant = useUIStore(state => state.toggleAssistant)

  return (
    <Button
      type="button"
      size="icon-lg"
      onClick={() => toggleAssistant()}
      aria-label={t('assistant.openAriaLabel')}
      data-testid="assistant-bubble"
      className="fixed bottom-4 end-4 z-50 size-12 rounded-full shadow-lg"
    >
      <Sparkles className="size-5" />
    </Button>
  )
}
