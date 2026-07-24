import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Eye, EyeOff } from 'lucide-react'
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
} from '@/components/ui/input-group'
import { SettingsField, SettingsSection } from '../shared/SettingsComponents'
import {
  SECRET_KEYS,
  useSecret,
  useSaveSecret,
  type SecretKey,
} from '@/services/secrets'
import { logger } from '@/lib/logger'

interface SecretFieldProps {
  secretKey: SecretKey
  label: string
  description: string
  placeholder: string
  showLabel: string
  hideLabel: string
}

function SecretField({
  secretKey,
  label,
  description,
  placeholder,
  showLabel,
  hideLabel,
}: SecretFieldProps) {
  const { data: storedValue = '', isLoading } = useSecret(secretKey)
  const saveSecret = useSaveSecret()
  // null means "use stored value"; non-null means user has edited
  const [draft, setDraft] = useState<string | null>(null)
  const [visible, setVisible] = useState(false)

  const value = draft ?? storedValue

  const handleBlur = async () => {
    if (draft === null || draft === storedValue) {
      setDraft(null)
      return
    }

    const valueToSave = draft
    logger.info('Saving secret on blur', { key: secretKey })
    try {
      await saveSecret.mutateAsync({ key: secretKey, value: valueToSave })
      setDraft(null)
    } catch {
      // Toast already shown by useSaveSecret; keep draft so user can retry
      logger.warn('Secret save failed, keeping draft', { key: secretKey })
    }
  }

  return (
    <SettingsField label={label} description={description}>
      <InputGroup>
        <InputGroupInput
          type={visible ? 'text' : 'password'}
          value={value}
          onChange={e => setDraft(e.target.value)}
          onBlur={handleBlur}
          disabled={isLoading || saveSecret.isPending}
          placeholder={placeholder}
          autoComplete="off"
          spellCheck={false}
          data-testid={`secret-input-${secretKey}`}
        />
        <InputGroupAddon align="inline-end">
          <InputGroupButton
            size="icon-xs"
            onClick={() => setVisible(v => !v)}
            aria-label={visible ? hideLabel : showLabel}
            data-testid={`secret-toggle-${secretKey}`}
          >
            {visible ? <EyeOff /> : <Eye />}
          </InputGroupButton>
        </InputGroupAddon>
      </InputGroup>
    </SettingsField>
  )
}

export function IntegrationsPane() {
  const { t } = useTranslation()

  const showLabel = t('preferences.integrations.showSecret')
  const hideLabel = t('preferences.integrations.hideSecret')
  const placeholder = t('preferences.integrations.placeholder')

  return (
    <div className="space-y-6">
      <SettingsSection title={t('preferences.integrations.websupport')}>
        <SecretField
          secretKey={SECRET_KEYS.websupportIdentifier}
          label={t('preferences.integrations.websupportIdentifier')}
          description={t(
            'preferences.integrations.websupportIdentifierDescription'
          )}
          placeholder={placeholder}
          showLabel={showLabel}
          hideLabel={hideLabel}
        />
        <SecretField
          secretKey={SECRET_KEYS.websupportSecret}
          label={t('preferences.integrations.websupportSecret')}
          description={t(
            'preferences.integrations.websupportSecretDescription'
          )}
          placeholder={placeholder}
          showLabel={showLabel}
          hideLabel={hideLabel}
        />
      </SettingsSection>

      <SettingsSection title={t('preferences.integrations.websupportDyndns')}>
        <SecretField
          secretKey={SECRET_KEYS.websupportDyndnsIdentifier}
          label={t('preferences.integrations.websupportDyndnsIdentifier')}
          description={t(
            'preferences.integrations.websupportDyndnsIdentifierDescription'
          )}
          placeholder={placeholder}
          showLabel={showLabel}
          hideLabel={hideLabel}
        />
        <SecretField
          secretKey={SECRET_KEYS.websupportDyndnsSecret}
          label={t('preferences.integrations.websupportDyndnsSecret')}
          description={t(
            'preferences.integrations.websupportDyndnsSecretDescription'
          )}
          placeholder={placeholder}
          showLabel={showLabel}
          hideLabel={hideLabel}
        />
      </SettingsSection>

      <SettingsSection title={t('preferences.integrations.mistral')}>
        <SecretField
          secretKey={SECRET_KEYS.mistralApiKey}
          label={t('preferences.integrations.mistralApiKey')}
          description={t('preferences.integrations.mistralApiKeyDescription')}
          placeholder={placeholder}
          showLabel={showLabel}
          hideLabel={hideLabel}
        />
      </SettingsSection>
    </div>
  )
}
