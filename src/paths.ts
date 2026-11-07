import { homeDir } from '@tauri-apps/api/path'

let home: string | null = null

/// called once at startup so displayPath can abbreviate the home prefix as `~`
export async function initPaths(): Promise<void> {
  try {
    home = await homeDir()
  } catch {
    home = null // abbreviation simply won't kick in
  }
}

/// paths shown in the UI shouldn't leak the account name (screenshots, bug
/// reports): /Users/me/Pictures/cat.png renders as ~/Pictures/cat.png
export function displayPath(path: string): string {
  if (!home) return path
  const prefix = home.endsWith('/') ? home : home + '/'
  if (path === home) return '~'
  if (path.startsWith(prefix)) return '~/' + path.slice(prefix.length)
  return path
}
