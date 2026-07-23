import { isTauri as checkIsTauri } from '@tauri-apps/api/core'

/** True when running inside the Tauri desktop shell (not a plain browser). */
export function isTauri(): boolean {
  return checkIsTauri()
}
