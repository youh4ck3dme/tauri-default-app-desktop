import { describe, it, expect, beforeEach, vi } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { commands } from '@/lib/tauri-bindings'
import { usePreferences, useSavePreferences } from './preferences'

const mockCommands = vi.mocked(commands)

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  })
  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    )
  }
  return Wrapper
}

describe('usePreferences', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns preferences loaded from the backend', async () => {
    mockCommands.loadPreferences.mockResolvedValue({
      status: 'ok',
      data: { theme: 'dark', quick_pane_shortcut: null, language: 'en' },
    })

    const { result } = renderHook(() => usePreferences(), {
      wrapper: createWrapper(),
    })

    await waitFor(() => expect(result.current.isSuccess).toBe(true))

    expect(result.current.data).toEqual({
      theme: 'dark',
      quick_pane_shortcut: null,
      language: 'en',
    })
  })

  it('falls back to defaults when loading fails', async () => {
    mockCommands.loadPreferences.mockResolvedValue({
      status: 'error',
      error: 'not found',
    })

    const { result } = renderHook(() => usePreferences(), {
      wrapper: createWrapper(),
    })

    await waitFor(() => expect(result.current.isSuccess).toBe(true))

    expect(result.current.data).toEqual({
      theme: 'system',
      quick_pane_shortcut: null,
      language: null,
    })
  })
})

describe('useSavePreferences', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('saves preferences and updates the query cache on success', async () => {
    mockCommands.savePreferences.mockResolvedValue({
      status: 'ok',
      data: null,
    })

    const { result } = renderHook(() => useSavePreferences(), {
      wrapper: createWrapper(),
    })

    result.current.mutate({
      theme: 'dark',
      quick_pane_shortcut: null,
      language: null,
    })

    await waitFor(() => expect(result.current.isSuccess).toBe(true))

    expect(mockCommands.savePreferences).toHaveBeenCalledWith({
      theme: 'dark',
      quick_pane_shortcut: null,
      language: null,
    })
  })

  it('surfaces an error when saving fails', async () => {
    mockCommands.savePreferences.mockResolvedValue({
      status: 'error',
      error: 'disk full',
    })

    const { result } = renderHook(() => useSavePreferences(), {
      wrapper: createWrapper(),
    })

    result.current.mutate({
      theme: 'dark',
      quick_pane_shortcut: null,
      language: null,
    })

    await waitFor(() => expect(result.current.isError).toBe(true))

    expect(result.current.error).toBeInstanceOf(Error)
    expect(result.current.error?.message).toBe('disk full')
  })
})
