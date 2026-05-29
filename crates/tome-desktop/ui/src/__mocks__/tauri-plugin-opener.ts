// Mock for `@tauri-apps/plugin-opener` (axe-core gate).

export async function openPath(_path: string): Promise<void> {
  return undefined;
}

export async function openUrl(_url: string): Promise<void> {
  return undefined;
}

export async function revealItemInDir(_path: string): Promise<void> {
  return undefined;
}
