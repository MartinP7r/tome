// Mock for `@tauri-apps/plugin-clipboard-manager` (axe-core gate).
// The Skills view calls `writeText` after `copy_path` returns; the
// a11y gate doesn't exercise the action, but the import must resolve.

export async function writeText(_text: string): Promise<void> {
  return undefined;
}

export async function readText(): Promise<string> {
  return "";
}
