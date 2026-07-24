import { describe, it, expect, beforeEach, vi } from 'vitest'
import { commands } from '@/lib/tauri-bindings'
import {
  saveEmergencyData,
  loadEmergencyData,
  cleanupOldFiles,
  saveCrashState,
} from './recovery'

const mockCommands = vi.mocked(commands)

describe('saveEmergencyData', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('resolves without throwing on success', async () => {
    mockCommands.saveEmergencyData.mockResolvedValue({
      status: 'ok',
      data: null,
    })

    await expect(
      saveEmergencyData('draft', { content: 'hello' })
    ).resolves.toBeUndefined()

    expect(mockCommands.saveEmergencyData).toHaveBeenCalledWith('draft', {
      content: 'hello',
    })
  })

  it('throws a human-readable error on ValidationError', async () => {
    mockCommands.saveEmergencyData.mockResolvedValue({
      status: 'error',
      error: { type: 'ValidationError', message: 'bad filename' },
    })

    await expect(saveEmergencyData('bad/name', {})).rejects.toThrow(
      'Validation error: bad filename'
    )
  })

  it('throws a human-readable error on DataTooLarge', async () => {
    mockCommands.saveEmergencyData.mockResolvedValue({
      status: 'error',
      error: { type: 'DataTooLarge', max_bytes: 1024 },
    })

    await expect(saveEmergencyData('draft', {})).rejects.toThrow(
      'Data too large (max 1024 bytes)'
    )
  })
})

describe('loadEmergencyData', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns the parsed data on success', async () => {
    mockCommands.loadEmergencyData.mockResolvedValue({
      status: 'ok',
      data: { content: 'restored' },
    })

    const result = await loadEmergencyData<{ content: string }>('draft')

    expect(result).toEqual({ content: 'restored' })
  })

  it('returns null instead of throwing when the file is not found', async () => {
    mockCommands.loadEmergencyData.mockResolvedValue({
      status: 'error',
      error: { type: 'FileNotFound' },
    })

    const result = await loadEmergencyData('missing')

    expect(result).toBeNull()
  })

  it('throws for non-FileNotFound errors', async () => {
    mockCommands.loadEmergencyData.mockResolvedValue({
      status: 'error',
      error: { type: 'ParseError', message: 'invalid JSON' },
    })

    await expect(loadEmergencyData('corrupt')).rejects.toThrow(
      'Parse error: invalid JSON'
    )
  })
})

describe('cleanupOldFiles', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns the number of removed files', async () => {
    mockCommands.cleanupOldRecoveryFiles.mockResolvedValue({
      status: 'ok',
      data: 3,
    })

    await expect(cleanupOldFiles()).resolves.toBe(3)
  })

  it('throws a human-readable error on failure', async () => {
    mockCommands.cleanupOldRecoveryFiles.mockResolvedValue({
      status: 'error',
      error: { type: 'IoError', message: 'disk full' },
    })

    await expect(cleanupOldFiles()).rejects.toThrow('IO error: disk full')
  })
})

describe('saveCrashState', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('saves crash data without throwing, even on failure', async () => {
    mockCommands.saveEmergencyData.mockResolvedValue({
      status: 'error',
      error: { type: 'IoError', message: 'disk full' },
    })

    // Crash handlers must never throw - this would mask the original crash
    await expect(
      saveCrashState({ page: '/dashboard' }, { error: 'boom' })
    ).resolves.toBeUndefined()
  })

  it('saves crash data silently on success', async () => {
    mockCommands.saveEmergencyData.mockResolvedValue({
      status: 'ok',
      data: null,
    })

    await saveCrashState({ page: '/dashboard' })

    expect(mockCommands.saveEmergencyData).toHaveBeenCalledWith(
      expect.stringMatching(/^crash-\d+$/),
      expect.objectContaining({ state: { page: '/dashboard' } })
    )
  })
})
