import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@/test/test-utils'
import { commands } from '@/lib/tauri-bindings'
import { IntegrationsPane } from './IntegrationsPane'
import { SECRET_KEYS } from '@/services/secrets'

const mockCommands = vi.mocked(commands)

const ALL_SECRET_KEYS = Object.values(SECRET_KEYS)

describe('IntegrationsPane', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockCommands.getSecret.mockResolvedValue({ status: 'ok', data: null })
    mockCommands.saveSecret.mockResolvedValue({ status: 'ok', data: null })
  })

  it('loads and displays all five secret fields', async () => {
    render(<IntegrationsPane />)

    await waitFor(() => {
      for (const key of ALL_SECRET_KEYS) {
        expect(screen.getByTestId(`secret-input-${key}`)).toBeInTheDocument()
      }
    })

    expect(screen.getByText('Websupport Identifier')).toBeInTheDocument()
    expect(screen.getByText('Websupport Secret Key')).toBeInTheDocument()
    expect(screen.getByText('Websupport DynDNS Identifier')).toBeInTheDocument()
    expect(screen.getByText('Websupport DynDNS Secret Key')).toBeInTheDocument()
    expect(screen.getByText('Mistral API Key')).toBeInTheDocument()
  })

  it('calls getSecret for each secret key on mount', async () => {
    render(<IntegrationsPane />)

    await waitFor(() => {
      expect(mockCommands.getSecret).toHaveBeenCalledTimes(
        ALL_SECRET_KEYS.length
      )
    })

    for (const key of ALL_SECRET_KEYS) {
      expect(mockCommands.getSecret).toHaveBeenCalledWith(key)
    }
  })

  it('displays loaded secret values in the fields', async () => {
    mockCommands.getSecret.mockImplementation(async (key: string) => {
      if (key === SECRET_KEYS.websupportIdentifier) {
        return { status: 'ok', data: 'my-identifier' }
      }
      return { status: 'ok', data: null }
    })

    render(<IntegrationsPane />)

    await waitFor(() => {
      const input = screen.getByTestId(
        `secret-input-${SECRET_KEYS.websupportIdentifier}`
      ) as HTMLInputElement
      expect(input.value).toBe('my-identifier')
    })
  })

  it('saves on blur when the value has changed', async () => {
    render(<IntegrationsPane />)

    await waitFor(() => {
      expect(mockCommands.getSecret).toHaveBeenCalled()
    })

    const input = screen.getByTestId(
      `secret-input-${SECRET_KEYS.mistralApiKey}`
    ) as HTMLInputElement

    fireEvent.change(input, { target: { value: 'sk-test-key-123' } })
    fireEvent.blur(input)

    await waitFor(() => {
      expect(mockCommands.saveSecret).toHaveBeenCalledWith(
        SECRET_KEYS.mistralApiKey,
        'sk-test-key-123'
      )
    })
  })

  it('does not save on blur when the value is unchanged', async () => {
    mockCommands.getSecret.mockImplementation(async (key: string) => {
      if (key === SECRET_KEYS.mistralApiKey) {
        return { status: 'ok', data: 'existing-key' }
      }
      return { status: 'ok', data: null }
    })

    render(<IntegrationsPane />)

    await waitFor(() => {
      const input = screen.getByTestId(
        `secret-input-${SECRET_KEYS.mistralApiKey}`
      ) as HTMLInputElement
      expect(input.value).toBe('existing-key')
    })

    const input = screen.getByTestId(
      `secret-input-${SECRET_KEYS.mistralApiKey}`
    )

    fireEvent.blur(input)

    // Allow any microtasks to flush
    await waitFor(() => {
      expect(mockCommands.getSecret).toHaveBeenCalled()
    })

    expect(mockCommands.saveSecret).not.toHaveBeenCalled()
  })

  it('toggles password visibility', async () => {
    render(<IntegrationsPane />)

    const input = await screen.findByTestId(
      `secret-input-${SECRET_KEYS.websupportSecret}`
    )
    expect(input).toHaveAttribute('type', 'password')

    const toggle = screen.getByTestId(
      `secret-toggle-${SECRET_KEYS.websupportSecret}`
    )
    fireEvent.click(toggle)

    expect(input).toHaveAttribute('type', 'text')

    fireEvent.click(toggle)
    expect(input).toHaveAttribute('type', 'password')
  })
})
