import { describe, it, expect, afterEach, vi } from 'vitest'
import { renderHook } from '@testing-library/react'
import type { CommandContext } from '@/lib/commands/types'

const mockUIStore = {
  getState: vi.fn(() => ({
    leftSidebarVisible: true,
    rightSidebarVisible: true,
    setLeftSidebarVisible: vi.fn(),
    setRightSidebarVisible: vi.fn(),
  })),
}

vi.mock('@/store/ui-store', () => ({
  useUIStore: mockUIStore,
}))

const { useKeyboardShortcuts } = await import('./use-keyboard-shortcuts')

function createContext(): CommandContext {
  return {
    openPreferences: vi.fn(),
    showToast: vi.fn(),
  }
}

function dispatchKeydown(
  key: string,
  { meta = false, ctrl = false }: { meta?: boolean; ctrl?: boolean } = {}
) {
  const event = new KeyboardEvent('keydown', {
    key,
    metaKey: meta,
    ctrlKey: ctrl,
    bubbles: true,
    cancelable: true,
  })
  document.dispatchEvent(event)
  return event
}

describe('useKeyboardShortcuts', () => {
  afterEach(() => {
    vi.clearAllMocks()
  })

  it('opens preferences on Cmd/Ctrl+,', () => {
    const context = createContext()
    renderHook(() => useKeyboardShortcuts(context))

    dispatchKeydown(',', { meta: true })

    expect(context.openPreferences).toHaveBeenCalledTimes(1)
  })

  it('toggles the left sidebar on Cmd/Ctrl+1', () => {
    const setLeftSidebarVisible = vi.fn()
    mockUIStore.getState.mockReturnValue({
      leftSidebarVisible: true,
      rightSidebarVisible: true,
      setLeftSidebarVisible,
      setRightSidebarVisible: vi.fn(),
    })
    renderHook(() => useKeyboardShortcuts(createContext()))

    dispatchKeydown('1', { ctrl: true })

    expect(setLeftSidebarVisible).toHaveBeenCalledWith(false)
  })

  it('toggles the right sidebar on Cmd/Ctrl+2', () => {
    const setRightSidebarVisible = vi.fn()
    mockUIStore.getState.mockReturnValue({
      leftSidebarVisible: true,
      rightSidebarVisible: false,
      setLeftSidebarVisible: vi.fn(),
      setRightSidebarVisible,
    })
    renderHook(() => useKeyboardShortcuts(createContext()))

    dispatchKeydown('2', { meta: true })

    expect(setRightSidebarVisible).toHaveBeenCalledWith(true)
  })

  it('ignores keydowns without a modifier key', () => {
    const context = createContext()
    renderHook(() => useKeyboardShortcuts(context))

    dispatchKeydown(',')

    expect(context.openPreferences).not.toHaveBeenCalled()
  })

  it('prevents the default browser action for handled shortcuts', () => {
    renderHook(() => useKeyboardShortcuts(createContext()))

    const event = dispatchKeydown(',', { meta: true })

    expect(event.defaultPrevented).toBe(true)
  })

  it('removes the listener on unmount', () => {
    const context = createContext()
    const { unmount } = renderHook(() => useKeyboardShortcuts(context))
    unmount()

    dispatchKeydown(',', { meta: true })

    expect(context.openPreferences).not.toHaveBeenCalled()
  })
})
