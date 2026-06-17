export type ClipboardKind = 'text' | 'rich_text' | 'image' | 'file';

export interface ClipboardItem {
  id: number;
  kind: ClipboardKind;
  title: string;
  content: string;
  preview: string;
  meta: string;
  source_app?: string | null;
  created_at: string;
  used_at?: string | null;
}

export interface QuickInput {
  id: number;
  content: string;
  tags: string[];
  prefix: string;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

export interface AppSettings {
  history_limit: number;
  theme_key: string;
  font_key: string;
  main_hotkey: 'Alt' | 'Ctrl';
  main_hotkey_enabled: boolean;
  inline_trigger: string;
  inline_trigger_enabled: boolean;
  launch_at_startup: boolean;
  launch_as_admin: boolean;
  auto_hide_to_tray: boolean;
  confirm_close_to_tray: boolean;
  enter_paste_to_active: boolean;
  hide_on_blur: boolean;
  confirm_exit: boolean;
  move_activated_to_top: boolean;
  close_after_activation: boolean;
  focus_previous_after_activation: boolean;
  paste_after_activation: boolean;
  onboarding_completed: boolean;
}

export interface SaveQuickInputPayload {
  id?: number | null;
  content: string;
  tags: string[];
}
