import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import i18n from '@/i18n/config'
import { logger } from '@/lib/logger'
import { commands } from '@/lib/tauri-bindings'

/** Known secret key identifiers stored in the OS keychain. */
export const SECRET_KEYS = {
  websupportIdentifier: 'websupport_identifier',
  websupportSecret: 'websupport_secret',
  websupportDyndnsIdentifier: 'websupport_dyndns_identifier',
  websupportDyndnsSecret: 'websupport_dyndns_secret',
  mistralApiKey: 'mistral_api_key',
} as const

export type SecretKey = (typeof SECRET_KEYS)[keyof typeof SECRET_KEYS]

export const secretsQueryKeys = {
  all: ['secrets'] as const,
  secret: (key: string) => [...secretsQueryKeys.all, key] as const,
}

export function useSecret(key: string) {
  return useQuery({
    queryKey: secretsQueryKeys.secret(key),
    queryFn: async (): Promise<string> => {
      logger.debug('Loading secret from backend', { key })
      const result = await commands.getSecret(key)

      if (result.status === 'error') {
        logger.warn('Failed to load secret', { key, error: result.error })
        // Treat load failures as empty so the field remains usable
        return ''
      }

      logger.info('Secret loaded successfully', {
        key,
        hasValue: result.data != null && result.data.length > 0,
      })
      return result.data ?? ''
    },
    staleTime: 1000 * 60 * 5, // 5 minutes
    gcTime: 1000 * 60 * 10, // 10 minutes
  })
}

export function useSaveSecret() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ key, value }: { key: string; value: string }) => {
      logger.debug('Saving secret to backend', { key })
      const result = await commands.saveSecret(key, value)

      if (result.status === 'error') {
        logger.error('Failed to save secret', { key, error: result.error })
        toast.error(i18n.t('toast.error.secretSaveFailed'), {
          description: result.error,
        })
        throw new Error(result.error)
      }

      logger.info('Secret saved successfully', { key })
    },
    onSuccess: (_, { key, value }) => {
      queryClient.setQueryData(secretsQueryKeys.secret(key), value)
      logger.info('Secret cache updated', { key })
      toast.success(i18n.t('toast.success.secretSaved'))
    },
  })
}
