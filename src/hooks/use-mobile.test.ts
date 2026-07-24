import { describe, it, expect } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useIsMobile } from './use-mobile'

function setInnerWidth(width: number) {
  Object.defineProperty(window, 'innerWidth', {
    writable: true,
    configurable: true,
    value: width,
  })
}

/** Installs a controllable matchMedia mock and returns handles to drive it. */
function mockMatchMedia() {
  let changeListener: (() => void) | undefined
  const mql = {
    matches: false,
    media: '',
    addEventListener: (_event: string, cb: () => void) => {
      changeListener = cb
    },
    removeEventListener: (_event: string, _cb: () => void) => {
      changeListener = undefined
    },
  }
  const matchMedia = (query: string) => ({ ...mql, media: query })

  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: matchMedia,
  })

  return { triggerChange: () => changeListener?.() }
}

describe('useIsMobile', () => {
  it('returns false when the viewport is above the breakpoint', () => {
    setInnerWidth(1024)
    mockMatchMedia()

    const { result } = renderHook(() => useIsMobile())

    expect(result.current).toBe(false)
  })

  it('returns true when the viewport is below the breakpoint', () => {
    setInnerWidth(500)
    mockMatchMedia()

    const { result } = renderHook(() => useIsMobile())

    expect(result.current).toBe(true)
  })

  it('updates when the media query change event fires', () => {
    setInnerWidth(1024)
    const { triggerChange } = mockMatchMedia()

    const { result } = renderHook(() => useIsMobile())
    expect(result.current).toBe(false)

    setInnerWidth(400)
    act(() => {
      triggerChange()
    })

    expect(result.current).toBe(true)
  })

  it('cleans up the media query listener on unmount', () => {
    setInnerWidth(1024)
    const { triggerChange } = mockMatchMedia()

    const { result, unmount } = renderHook(() => useIsMobile())
    unmount()

    setInnerWidth(300)
    act(() => {
      triggerChange()
    })

    // Listener was detached on unmount, so no state change should apply
    expect(result.current).toBe(false)
  })
})
