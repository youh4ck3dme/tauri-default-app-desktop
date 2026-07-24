/**
 * Re-export generated Tauri bindings with project conventions
 *
 * This file provides type-safe access to all Tauri commands.
 * Types are auto-generated from Rust by tauri-specta.
 *
 * @see docs/developer/tauri-commands.md for full documentation
 */

export { commands, type Result } from './bindings'
export type {
  AppPreferences,
  JsonValue,
  RecoveryError,
  DnsRecord,
  CreateDnsRecordInput,
  DnsZone,
  Hosting,
  VHost,
  Mailbox,
  CreateMailboxInput,
  UpdateMailboxInput,
} from './bindings'

/**
 * Helper to unwrap a Result type, throwing on error
 */
export function unwrapResult<T, E>(
  result: { status: 'ok'; data: T } | { status: 'error'; error: E }
): T {
  if (result.status === 'ok') {
    return result.data
  }
  throw result.error
}
