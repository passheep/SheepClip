import { invoke } from '@tauri-apps/api/core';
import type { AppSettings, ClipboardItem, QuickInput, SaveQuickInputPayload } from '../types';

const fallbackClipboardItems: ClipboardItem[] = [
  {
    id: 1,
    kind: 'text',
    title: '这里会自动记录你复制过的文本、图片、文件和富文本',
    content: '这里会自动记录你复制过的文本、图片、文件和富文本。双击条目或按 Enter 可以快速复制。',
    preview: '这里会自动记录你复制过的文本、图片、文件和富文本。双击条目或按 Enter 可以快速复制。',
    meta: '36 字',
    source_app: '演示数据',
    created_at: new Date().toISOString(),
    used_at: null,
    copy_count: 1,
  },
];

const fallbackQuickInputs: QuickInput[] = [
  {
    id: 1,
    content: '13333333333',
    tags: ['contact'],
    prefix: '133',
    sort_order: 1,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: 2,
    content: 'xxxxxxxx@qq.com',
    tags: ['email'],
    prefix: 'qq',
    sort_order: 2,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: 3,
    content: '北京市朝阳区xxxxxxxx102',
    tags: ['address'],
    prefix: '北京',
    sort_order: 3,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
];

const fallbackTags = ['contact', 'email', 'address', 'common'];

const fallbackSettings: AppSettings = {
  history_limit: 2000,
  theme_key: 'warm',
  font_key: 'system',
  font_size: 14,
  font_weight: 400,
  main_hotkey: 'Alt',
  main_hotkey_enabled: true,
  inline_trigger: '//',
  inline_trigger_enabled: true,
  launch_at_startup: false,
  launch_as_admin: false,
  auto_hide_to_tray: false,
  confirm_close_to_tray: true,
  enter_paste_to_active: false,
  hide_on_blur: false,
  confirm_exit: true,
  move_activated_to_top: true,
  close_after_activation: true,
  focus_previous_after_activation: true,
  paste_after_activation: false,
  onboarding_completed: false,
};

function isTauriRuntime() {
  return Boolean('__TAURI_INTERNALS__' in window);
}

async function call<T>(command: string, args?: Record<string, unknown>, fallback?: T): Promise<T> {
  if (!isTauriRuntime()) {
    if (fallback === undefined) {
      throw new Error(`Command ${command} needs Tauri runtime`);
    }
    return fallback;
  }

  return invoke<T>(command, args);
}

export async function listClipboardItems(query: string, limit = 100) {
  return call<ClipboardItem[]>('list_clipboard_items', { query, limit }, fallbackClipboardItems);
}

export async function listQuickInputs(query: string) {
  return call<QuickInput[]>('list_quick_inputs', { query, limit: 100 }, fallbackQuickInputs);
}

export async function listTags() {
  return call<string[]>('list_tags', undefined, fallbackTags);
}

export async function addTag(name: string) {
  return call<string>('add_tag', { name });
}

export async function deleteTag(name: string) {
  return call<void>('delete_tag', { name });
}

export async function saveQuickInput(payload: SaveQuickInputPayload) {
  return call<QuickInput>('save_quick_input', { payload });
}

export async function deleteQuickInput(id: number) {
  return call<void>('delete_quick_input', { id });
}

export async function reorderQuickInputs(ids: number[]) {
  return call<void>('reorder_quick_inputs', { ids });
}

export async function getSettings() {
  return call<AppSettings>('get_settings', undefined, fallbackSettings);
}

export async function saveSettings(settings: AppSettings) {
  return call<AppSettings>('save_settings', { settings });
}

export async function resetSettings() {
  return call<AppSettings>('reset_settings', undefined, fallbackSettings);
}

export async function markMainPointerOperation() {
  return call<void>('mark_main_pointer_operation');
}

export async function openExternalUrl(url: string) {
  return call<void>('open_external_url', { url });
}

export async function copyClipboardItem(id: number, paste = false) {
  return call<void>('copy_item_to_clipboard', { source: 'clipboard', id, paste });
}

export async function copyQuickInput(id: number, paste = false) {
  return call<void>('copy_item_to_clipboard', { source: 'quick', id, paste });
}

export async function deleteClipboardItem(id: number) {
  return call<void>('delete_clipboard_item', { id });
}

export async function pasteItemFromMainWindow(source: 'clipboard' | 'quick', id: number) {
  return call<void>('paste_item_from_main_window', { source, id });
}

export async function captureCurrentClipboard() {
  return call<void>('capture_current_clipboard');
}

export async function clearClipboardHistory() {
  return call<void>('clear_clipboard_history');
}

export async function hideMainWindow() {
  return call<void>('hide_main_window');
}

export async function minimizeMainWindow() {
  return call<void>('minimize_main_window');
}

export async function setMainWindowAlwaysOnTop(pinned: boolean) {
  return call<void>('set_main_window_always_on_top', { pinned });
}

export async function exitApp() {
  return call<void>('exit_app');
}

export async function addClipboardItemToQuickInput(id: number) {
  return call<QuickInput>('add_clipboard_item_to_quick_input', { id });
}

export async function pasteQuickInputFromFloating(id: number, backspaceCount: number, source = 'quick') {
  return call<void>('paste_quick_input_from_floating', { id, backspaceCount, source });
}
