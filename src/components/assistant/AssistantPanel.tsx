import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Bot, Send } from 'lucide-react'
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { useUIStore } from '@/store/ui-store'
import {
  useConfirmAssistantAction,
  useSendAssistantMessage,
} from '@/services/assistant'
import type { ChatMessage, PendingAction } from '@/lib/tauri-bindings'
import { cn } from '@/lib/utils'

interface DisplayMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  pendingActions?: PendingAction[]
}

function newId(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
}

export function AssistantPanel() {
  const { t } = useTranslation()
  const assistantOpen = useUIStore(state => state.assistantOpen)
  const setAssistantOpen = useUIStore(state => state.setAssistantOpen)

  const [messages, setMessages] = useState<DisplayMessage[]>([])
  const [draft, setDraft] = useState('')
  const scrollRef = useRef<HTMLDivElement>(null)
  const sendMutation = useSendAssistantMessage()
  const confirmMutation = useConfirmAssistantAction()

  const isBusy = sendMutation.isPending || confirmMutation.isPending

  // Auto-scroll to latest message
  useEffect(() => {
    const el = scrollRef.current
    if (el) {
      el.scrollTop = el.scrollHeight
    }
  }, [messages, isBusy])

  const handleSend = async () => {
    const text = draft.trim()
    if (!text || isBusy) return

    const userMessage: DisplayMessage = {
      id: newId(),
      role: 'user',
      content: text,
    }
    const nextMessages = [...messages, userMessage]
    setMessages(nextMessages)
    setDraft('')

    const conversation: ChatMessage[] = nextMessages.map(m => ({
      role: m.role,
      content: m.content,
    }))

    try {
      const turn = await sendMutation.mutateAsync(conversation)
      setMessages(prev => [
        ...prev,
        {
          id: newId(),
          role: 'assistant',
          content: turn.reply || t('assistant.emptyReply'),
          pendingActions:
            turn.pending_actions.length > 0 ? turn.pending_actions : undefined,
        },
      ])
    } catch {
      // Error toast already shown by the service
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      void handleSend()
    }
  }

  const handleConfirm = async (messageId: string, action: PendingAction) => {
    try {
      const resultText = await confirmMutation.mutateAsync(action)
      // Remove the pending card from its parent message
      setMessages(prev =>
        prev.map(m =>
          m.id === messageId
            ? {
                ...m,
                pendingActions: m.pendingActions?.filter(
                  a => a.id !== action.id
                ),
              }
            : m
        )
      )
      // Append confirmation result as an assistant message
      setMessages(prev => [
        ...prev,
        {
          id: newId(),
          role: 'assistant',
          content: resultText,
        },
      ])
    } catch {
      // Error toast already shown by the service
    }
  }

  const handleCancel = (messageId: string, actionId: string) => {
    setMessages(prev =>
      prev.map(m =>
        m.id === messageId
          ? {
              ...m,
              pendingActions: m.pendingActions?.filter(a => a.id !== actionId),
            }
          : m
      )
    )
  }

  return (
    <Sheet open={assistantOpen} onOpenChange={setAssistantOpen}>
      <SheetContent
        side="right"
        className="w-full sm:max-w-md gap-0 border-zinc-800 bg-black p-0 text-zinc-100 [&>button]:text-zinc-400 [&>button]:hover:text-zinc-100"
        data-testid="assistant-panel"
      >
        <SheetHeader className="border-b border-zinc-800 px-4 py-3">
          <div className="flex items-center gap-3">
            <div
              className="flex size-9 shrink-0 items-center justify-center rounded-full bg-gradient-to-br from-violet-500 to-orange-400 text-white"
              aria-hidden
            >
              <Bot className="size-5" />
            </div>
            <div className="min-w-0">
              <SheetTitle className="text-zinc-100">
                {t('assistant.title')}
              </SheetTitle>
              <SheetDescription className="text-zinc-400">
                {t('assistant.subtitle')}
              </SheetDescription>
            </div>
          </div>
        </SheetHeader>

        <div
          ref={scrollRef}
          className="flex flex-1 flex-col gap-3 overflow-y-auto px-4 py-4"
          data-testid="assistant-messages"
        >
          {messages.length === 0 ? (
            <p
              className="text-center text-sm text-zinc-400 px-4 py-8"
              data-testid="assistant-empty-state"
            >
              {t('assistant.emptyState')}
            </p>
          ) : (
            messages.map(message => (
              <div key={message.id} className="flex flex-col gap-2">
                <div
                  className={cn(
                    'flex gap-2',
                    message.role === 'user' ? 'justify-end' : 'justify-start'
                  )}
                >
                  {message.role === 'assistant' && (
                    <div
                      className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-full bg-gradient-to-br from-violet-500 to-orange-400 text-white"
                      aria-hidden
                    >
                      <Bot className="size-3.5" />
                    </div>
                  )}
                  <div
                    className={cn(
                      'max-w-[85%] rounded-2xl px-3 py-2 text-sm whitespace-pre-wrap break-words',
                      message.role === 'user'
                        ? 'bg-violet-600 text-white'
                        : 'bg-zinc-900 text-zinc-100 border border-zinc-800'
                    )}
                    data-testid={`assistant-message-${message.role}`}
                  >
                    {message.content}
                  </div>
                </div>

                {message.pendingActions?.map(action => (
                  <div
                    key={action.id}
                    className="ms-9 rounded-xl border border-amber-500/40 bg-zinc-900 p-3"
                    data-testid={`pending-action-${action.id}`}
                  >
                    <p className="text-sm text-zinc-100">
                      {action.description}
                    </p>
                    <p className="mt-1 text-xs text-zinc-400">
                      {t('assistant.pendingHint')}
                    </p>
                    <div className="mt-3 flex gap-2">
                      <Button
                        type="button"
                        size="sm"
                        disabled={isBusy}
                        onClick={() => void handleConfirm(message.id, action)}
                        data-testid={`confirm-action-${action.id}`}
                      >
                        {t('assistant.confirm')}
                      </Button>
                      <Button
                        type="button"
                        size="sm"
                        variant="outline"
                        disabled={isBusy}
                        onClick={() => handleCancel(message.id, action.id)}
                        className="border-zinc-700 bg-transparent text-zinc-200 hover:bg-zinc-800 hover:text-zinc-50"
                        data-testid={`cancel-action-${action.id}`}
                      >
                        {t('assistant.cancel')}
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            ))
          )}

          {sendMutation.isPending && (
            <p
              className="text-xs text-zinc-500 ps-9"
              data-testid="assistant-thinking"
            >
              {t('assistant.thinking')}
            </p>
          )}
        </div>

        <div className="border-t border-zinc-800 p-3">
          <div className="flex items-end gap-2">
            <Textarea
              value={draft}
              onChange={e => setDraft(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={t('assistant.placeholder')}
              disabled={isBusy}
              rows={2}
              className="min-h-[44px] resize-none border-zinc-700 bg-zinc-950 text-zinc-100 placeholder:text-zinc-500 focus-visible:border-violet-500 focus-visible:ring-violet-500/30"
              data-testid="assistant-input"
              aria-label={t('assistant.inputAriaLabel')}
            />
            <Button
              type="button"
              size="icon"
              onClick={() => void handleSend()}
              disabled={isBusy || !draft.trim()}
              aria-label={t('assistant.sendAriaLabel')}
              data-testid="assistant-send"
              className="shrink-0"
            >
              <Send className="size-4" />
            </Button>
          </div>
        </div>
      </SheetContent>
    </Sheet>
  )
}
