import { describe, it, expect } from 'vitest'
import { getPlatformStrings, formatShortcut } from './platform-strings'

describe('getPlatformStrings', () => {
  it('returns macOS strings for macos', () => {
    const strings = getPlatformStrings('macos')
    expect(strings.modifierKeySymbol).toBe('⌘')
    expect(strings.revealInFileManager).toBe('Reveal in Finder')
    expect(strings.trashName).toBe('Trash')
  })

  it('returns Windows strings for windows', () => {
    const strings = getPlatformStrings('windows')
    expect(strings.modifierKeySymbol).toBe('Ctrl')
    expect(strings.revealInFileManager).toBe('Show in Explorer')
    expect(strings.trashName).toBe('Recycle Bin')
  })

  it('returns Linux strings for linux', () => {
    const strings = getPlatformStrings('linux')
    expect(strings.modifierKeySymbol).toBe('Ctrl')
    expect(strings.revealInFileManager).toBe('Show in Files')
    expect(strings.trashName).toBe('Trash')
  })

  it('defaults to macOS strings when platform is undefined', () => {
    expect(getPlatformStrings(undefined)).toEqual(getPlatformStrings('macos'))
  })
})

describe('formatShortcut', () => {
  it('formats a mod-only shortcut on macOS with the symbol and no separator', () => {
    expect(formatShortcut('macos', 'K')).toBe('⌘K')
  })

  it('formats a mod-only shortcut on Windows with a plus separator', () => {
    expect(formatShortcut('windows', 'K')).toBe('Ctrl+K')
  })

  it('formats a mod-only shortcut on Linux with a plus separator', () => {
    expect(formatShortcut('linux', 'K')).toBe('Ctrl+K')
  })

  it('combines shift and mod on macOS using symbols', () => {
    expect(formatShortcut('macos', 'K', ['shift', 'mod'])).toBe('⇧⌘K')
  })

  it('combines shift and mod on Windows using words', () => {
    expect(formatShortcut('windows', 'K', ['shift', 'mod'])).toBe(
      'Shift+Ctrl+K'
    )
  })

  it('combines alt and mod on macOS', () => {
    expect(formatShortcut('macos', 'K', ['alt', 'mod'])).toBe('⌥⌘K')
  })

  it('renders a bare key with no modifiers', () => {
    expect(formatShortcut('macos', 'F1', [])).toBe('F1')
    expect(formatShortcut('windows', 'Escape', [])).toBe('Escape')
  })

  it('defaults to the mod modifier when none is passed', () => {
    expect(formatShortcut('macos', 'S')).toBe(
      formatShortcut('macos', 'S', ['mod'])
    )
  })

  it('treats an undefined platform as macOS', () => {
    expect(formatShortcut(undefined, 'K')).toBe('⌘K')
  })
})
