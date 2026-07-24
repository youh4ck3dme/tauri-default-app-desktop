import { describe, it, expect } from 'vitest'
import { cn } from './utils'

describe('cn', () => {
  it('joins multiple class strings', () => {
    expect(cn('flex', 'items-center')).toBe('flex items-center')
  })

  it('drops falsy values', () => {
    expect(cn('flex', false, null, undefined, '', 'gap-2')).toBe('flex gap-2')
  })

  it('applies conditional classes from objects', () => {
    expect(cn('base', { active: true, disabled: false })).toBe('base active')
  })

  it('resolves conflicting Tailwind classes to the last one', () => {
    // twMerge should keep only the last conflicting utility
    expect(cn('p-2', 'p-4')).toBe('p-4')
    expect(cn('text-red-500', 'text-blue-500')).toBe('text-blue-500')
  })

  it('preserves non-conflicting classes alongside merged ones', () => {
    expect(cn('flex p-2', 'p-4 gap-2')).toBe('flex p-4 gap-2')
  })

  it('returns an empty string for no input', () => {
    expect(cn()).toBe('')
  })
})
