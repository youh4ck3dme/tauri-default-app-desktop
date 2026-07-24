import '@testing-library/jest-dom'
import { vi } from 'vitest'

// Mock matchMedia for tests
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(), // deprecated
    removeListener: vi.fn(), // deprecated
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
})

// Mock Tauri APIs for tests
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {
    // Mock unlisten function
  }),
}))

vi.mock('@tauri-apps/plugin-updater', () => ({
  check: vi.fn().mockResolvedValue(null),
}))

// Mock typed Tauri bindings (tauri-specta generated)
vi.mock('@/lib/tauri-bindings', () => ({
  commands: {
    greet: vi.fn().mockResolvedValue('Hello, test!'),
    loadPreferences: vi
      .fn()
      .mockResolvedValue({ status: 'ok', data: { theme: 'system' } }),
    savePreferences: vi.fn().mockResolvedValue({ status: 'ok', data: null }),
    sendNativeNotification: vi
      .fn()
      .mockResolvedValue({ status: 'ok', data: null }),
    saveEmergencyData: vi.fn().mockResolvedValue({ status: 'ok', data: null }),
    loadEmergencyData: vi.fn().mockResolvedValue({ status: 'ok', data: null }),
    cleanupOldRecoveryFiles: vi
      .fn()
      .mockResolvedValue({ status: 'ok', data: 0 }),
    getDefaultQuickPaneShortcut: vi
      .fn()
      .mockResolvedValue('CommandOrControl+Shift+.'),
    getSecret: vi.fn().mockResolvedValue({ status: 'ok', data: null }),
    saveSecret: vi.fn().mockResolvedValue({ status: 'ok', data: null }),
    deleteSecret: vi.fn().mockResolvedValue({ status: 'ok', data: null }),
    websupportTestConnection: vi
      .fn()
      .mockResolvedValue({ status: 'ok', data: true }),
    websupportGetDnsZone: vi.fn().mockResolvedValue({
      status: 'ok',
      data: { name: 'example.com', lastCheck: null, dnssecSigning: null },
    }),
    websupportListDnsRecords: vi
      .fn()
      .mockResolvedValue({ status: 'ok', data: [] }),
    websupportCreateDnsRecord: vi
      .fn()
      .mockResolvedValue({ status: 'ok', data: null }),
    websupportUpdateDnsRecord: vi
      .fn()
      .mockResolvedValue({ status: 'ok', data: null }),
    websupportDeleteDnsRecord: vi
      .fn()
      .mockResolvedValue({ status: 'ok', data: null }),
    websupportListHostings: vi
      .fn()
      .mockResolvedValue({ status: 'ok', data: [] }),
    websupportListDomains: vi
      .fn()
      .mockResolvedValue({ status: 'ok', data: [] }),
    websupportListMailboxes: vi
      .fn()
      .mockResolvedValue({ status: 'ok', data: [] }),
    websupportCreateMailbox: vi
      .fn()
      .mockResolvedValue({ status: 'ok', data: null }),
    websupportUpdateMailbox: vi
      .fn()
      .mockResolvedValue({ status: 'ok', data: null }),
    websupportDeleteMailbox: vi
      .fn()
      .mockResolvedValue({ status: 'ok', data: null }),
    mistralSendMessage: vi.fn().mockResolvedValue({
      status: 'ok',
      data: { reply: '', pending_actions: [] },
    }),
    mistralConfirmAction: vi
      .fn()
      .mockResolvedValue({ status: 'ok', data: 'ok' }),
  },
  unwrapResult: vi.fn((result: { status: string; data?: unknown }) => {
    if (result.status === 'ok') return result.data
    throw result
  }),
}))
