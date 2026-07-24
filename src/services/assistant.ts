import { useMutation } from '@tanstack/react-query'
import { toast } from 'sonner'
import i18n from '@/i18n/config'
import { logger } from '@/lib/logger'
import {
  commands,
  type ChatMessage,
  type MistralTurnResult,
  type PendingAction,
} from '@/lib/tauri-bindings'

/**
 * Sends the full conversation to Mistral and returns the assistant turn
 * (text reply + any pending mutating actions awaiting confirmation).
 */
export function useSendAssistantMessage() {
  return useMutation({
    mutationFn: async (
      conversation: ChatMessage[]
    ): Promise<MistralTurnResult> => {
      logger.debug('Sending assistant message', {
        messageCount: conversation.length,
      })
      const result = await commands.mistralSendMessage(conversation)

      if (result.status === 'error') {
        logger.error('Assistant send failed', { error: result.error })
        toast.error(i18n.t('toast.error.assistantSendFailed'), {
          description: result.error,
        })
        throw new Error(result.error)
      }

      logger.info('Assistant reply received', {
        pendingCount: result.data.pending_actions.length,
      })
      return result.data
    },
  })
}

/**
 * Confirms a pending mutating Websupport action from the assistant.
 */
export function useConfirmAssistantAction() {
  return useMutation({
    mutationFn: async (action: PendingAction): Promise<string> => {
      logger.debug('Confirming assistant action', {
        id: action.id,
        tool: action.tool_name,
      })
      const result = await commands.mistralConfirmAction(action)

      if (result.status === 'error') {
        logger.error('Assistant confirm failed', {
          id: action.id,
          error: result.error,
        })
        toast.error(i18n.t('toast.error.assistantConfirmFailed'), {
          description: result.error,
        })
        throw new Error(result.error)
      }

      logger.info('Assistant action confirmed', { id: action.id })
      return result.data
    },
  })
}
