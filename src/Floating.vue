<template>
  <main class="relative flex h-screen select-none flex-col overflow-hidden rounded-lg border border-line bg-panel text-ink shadow-soft" :data-theme="currentThemeKey" :style="floatingThemeStyle" @contextmenu.prevent @mousemove="resetIdleTimer" @mousedown="resetIdleTimer">
    <header data-tauri-drag-region class="shrink-0 border-b border-line bg-header px-3 py-2" @pointerdown="startWindowDrag">
      <div class="flex items-center justify-between gap-3">
        <div class="min-w-0">
          <span class="text-sm font-semibold">{{ activeSource === 'quick' ? '快捷输入' : '剪贴板历史' }}</span>
          <span class="ml-2 text-xs text-muted">{{ triggerText || settings.inline_trigger }}</span>
        </div>
        <span class="shrink-0 text-xs text-muted">Tab 切换 · Enter 粘贴</span>
      </div>
      <div class="mt-2 grid grid-cols-2 gap-1 rounded-md bg-sidebar p-1">
        <button type="button" class="h-7 rounded text-xs transition" :class="activeSource === 'quick' ? 'bg-card text-ink shadow-sm' : 'text-muted'" @click="switchSource('quick')">快捷短语</button>
        <button type="button" class="h-7 rounded text-xs transition" :class="activeSource === 'clipboard' ? 'bg-card text-ink shadow-sm' : 'text-muted'" @click="switchSource('clipboard')">剪贴板</button>
      </div>
      <input
        ref="queryInputRef"
        v-model="searchQuery"
        class="mt-2 h-8 w-full rounded-md border border-line bg-card px-3 text-sm outline-none transition focus:border-mint"
        placeholder="输入标签或关键字"
        autocomplete="off"
        @keydown="onKeydown"
        @input="onInput"
        @compositionupdate="onCompositionUpdate"
        @compositionend="onCompositionEnd"
      />
    </header>

    <section ref="listRef" class="scroll-thin min-h-0 flex-1 overscroll-contain overflow-y-auto p-2">
      <TransitionGroup tag="div" name="list-motion">
        <button
          v-for="(item, index) in items"
          :key="`${activeSource}-${item.id}`"
          type="button"
          :data-floating-index="index"
          class="mb-1 grid w-full grid-cols-[1fr_auto] items-center gap-2 rounded-md border px-3 py-2 text-left transition"
          :class="selectedIndex === index ? 'border-primaryMuted bg-primarySoft text-ink shadow-sm ring-2 ring-primaryMuted' : 'border-transparent bg-card hover:bg-primarySoft'"
          @mouseenter="selectedIndex = index; resetIdleTimer()"
          @click="pasteItem(item)"
        >
          <span class="min-w-0">
            <span class="block truncate text-sm">{{ item.display }}</span>
            <span v-if="activeSource === 'clipboard'" class="mt-0.5 block truncate text-xs text-muted">{{ kindLabel(item.kind) }} · {{ item.source_app || '未知来源' }}</span>
          </span>
          <span class="flex flex-wrap justify-end gap-1 text-xs text-muted">
            <template v-if="activeSource === 'quick'">
              <span v-for="tag in item.tags" :key="tag" class="rounded bg-sidebar px-1.5 py-0.5">{{ tag }}</span>
            </template>
            <span v-else class="rounded bg-sidebar px-1.5 py-0.5">{{ formatTime(item.created_at) }}</span>
          </span>
        </button>
        <div v-if="items.length === 0" key="empty" class="flex h-32 items-center justify-center text-sm text-muted">
          没有匹配内容
        </div>
      </TransitionGroup>
    </section>
    <div class="absolute bottom-0 right-0 h-4 w-4 cursor-nwse-resize" title="调整大小" @pointerdown.stop.prevent="startResize">
      <div class="absolute bottom-1 right-1 h-2 w-2 border-b border-r border-muted" />
    </div>
  </main>
</template>

<script setup lang="ts">
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue';
import type { AppSettings, ClipboardItem, QuickInput } from './types';
import { getSettings, listClipboardItems, listQuickInputs, pasteQuickInputFromFloating } from './lib/commands';
import { restoreAndTrackWindowSize } from './lib/windowSize';
import { FONT_OPTIONS, getAvailableFontOptions, getThemeStyle, resolveFontKey, resolveThemeKey, type FontOption } from './theme';

type FloatingSource = 'quick' | 'clipboard';
interface FloatingItem {
  id: number;
  display: string;
  tags: string[];
  kind?: ClipboardItem['kind'];
  source_app?: string | null;
  created_at?: string;
}

const items = ref<FloatingItem[]>([]);
const activeSource = ref<FloatingSource>('quick');
const selectedIndex = ref(0);
const triggerText = ref('');
const searchQuery = ref('');
const queryInputRef = ref<HTMLInputElement | null>(null);
const listRef = ref<HTMLElement | null>(null);
const availableFontOptions = ref<FontOption[]>([FONT_OPTIONS[0]]);
const settings = ref<AppSettings>({
  history_limit: 2000,
  theme_key: 'warm',
  font_key: 'system',
  main_hotkey: 'Alt',
  main_hotkey_enabled: true,
  inline_trigger: '//',
  inline_trigger_enabled: true,
  launch_at_startup: false,
  launch_as_admin: false,
  auto_hide_to_tray: true,
  confirm_close_to_tray: true,
  enter_paste_to_active: false,
  hide_on_blur: true,
  confirm_exit: true,
  move_activated_to_top: true,
  close_after_activation: true,
  focus_previous_after_activation: true,
  paste_after_activation: false,
  onboarding_completed: false,
});

const selectedItem = computed(() => items.value[selectedIndex.value]);
const currentThemeKey = computed(() => resolveThemeKey(settings.value.theme_key));
const currentFontKey = computed(() => resolveFontKey(settings.value.font_key, availableFontOptions.value.map((font) => font.key)));
const floatingThemeStyle = computed(() => getThemeStyle(currentThemeKey.value, currentFontKey.value, availableFontOptions.value));
let idleTimer = 0;
let focusHideTimer = 0;
let pointerOperationTimer = 0;
let nativePointerOperationActive = false;
let ignoreSlashUntil = 0;

async function refresh(query = '', trigger = settings.value.inline_trigger, nextSelectedIndex = 0) {
  triggerText.value = trigger;
  searchQuery.value = query;
  const rawItems = activeSource.value === 'quick'
    ? await listQuickInputs(query)
    : await listClipboardItems(query);
  items.value = activeSource.value === 'quick'
    ? (rawItems as QuickInput[]).map(mapQuickItem)
    : (rawItems as ClipboardItem[]).map(mapClipboardItem);
  selectedIndex.value = items.value.length === 0 ? 0 : Math.min(nextSelectedIndex, items.value.length - 1);
  await nextTick();
  if (listRef.value) {
    listRef.value.scrollTop = 0;
  }
  window.setTimeout(() => {
    queryInputRef.value?.focus({ preventScroll: true });
    queryInputRef.value?.select();
  }, 20);
  if (nextSelectedIndex > 0) {
    scrollSelectedIntoView();
  }
  resetIdleTimer();
}

async function pasteItem(item = selectedItem.value) {
  if (!item) return;
  window.clearTimeout(idleTimer);
  await pasteQuickInputFromFloating(item.id, triggerText.value.length, activeSource.value);
}

async function switchSource(source: FloatingSource) {
  activeSource.value = source;
  await refresh('', triggerText.value, 0);
}

function moveSelection(delta: number) {
  resetIdleTimer();
  if (items.value.length === 0) {
    selectedIndex.value = 0;
    return;
  }
  selectedIndex.value = (selectedIndex.value + delta + items.value.length) % items.value.length;
  scrollSelectedIntoView();
}

function scrollSelectedIntoView() {
  nextTick(() => {
    const element = listRef.value?.querySelector<HTMLElement>(`[data-floating-index="${selectedIndex.value}"]`);
    element?.scrollIntoView({ block: 'nearest' });
  });
}

async function updateSearch(value: string, commitValue = true) {
  resetIdleTimer();
  if (commitValue) {
    searchQuery.value = value;
  }
  const rawItems = activeSource.value === 'quick'
    ? await listQuickInputs(value)
    : await listClipboardItems(value);
  items.value = activeSource.value === 'quick'
    ? (rawItems as QuickInput[]).map(mapQuickItem)
    : (rawItems as ClipboardItem[]).map(mapClipboardItem);
  selectedIndex.value = 0;
  nextTick(() => {
    if (listRef.value) {
      listRef.value.scrollTop = 0;
    }
  });
  scrollSelectedIntoView();
}

function onInput() {
  if (Date.now() < ignoreSlashUntil && searchQuery.value.includes('/')) {
    searchQuery.value = searchQuery.value.replace(/\//g, '');
  }
  updateSearch(searchQuery.value);
}

function onCompositionUpdate(event: CompositionEvent) {
  if (event.data) {
    updateSearch(event.data, false);
  }
}

function onCompositionEnd() {
  updateSearch(searchQuery.value);
}

function onKeydown(event: KeyboardEvent) {
  resetIdleTimer();
  if (event.key === '/' && Date.now() < ignoreSlashUntil) {
    event.preventDefault();
    event.stopPropagation();
    return;
  }
  if (event.key === 'ArrowDown') {
    event.preventDefault();
    event.stopPropagation();
    moveSelection(1);
  } else if (event.key === 'ArrowUp') {
    event.preventDefault();
    event.stopPropagation();
    moveSelection(-1);
  } else if (event.key === 'Enter') {
    event.preventDefault();
    event.stopPropagation();
    pasteItem();
  } else if (event.key === 'Escape') {
    event.preventDefault();
    event.stopPropagation();
    pasteQuickInputFromFloating(0, 0);
  } else if (event.key === 'Tab') {
    event.preventDefault();
    event.stopPropagation();
    void switchSource(activeSource.value === 'quick' ? 'clipboard' : 'quick');
  }
}

function startWindowDrag(event: PointerEvent) {
  if (event.button !== 0 || !('__TAURI_INTERNALS__' in window)) return;
  const target = event.target as HTMLElement | null;
  if (target?.closest('button,input,textarea,select,a')) return;
  event.preventDefault();
  markNativePointerOperation();
  void getCurrentWindow().startDragging();
}

function startResize(event?: PointerEvent) {
  if (!('__TAURI_INTERNALS__' in window)) return;
  event?.preventDefault();
  markNativePointerOperation();
  void getCurrentWindow().startResizeDragging('SouthEast');
}

function mapQuickItem(item: QuickInput): FloatingItem {
  return {
    id: item.id,
    display: item.content,
    tags: item.tags,
  };
}

function mapClipboardItem(item: ClipboardItem): FloatingItem {
  return {
    id: item.id,
    display: clipboardDisplayText(item),
    tags: [],
    kind: item.kind,
    source_app: item.source_app,
    created_at: item.created_at,
  };
}

function clipboardDisplayText(item: ClipboardItem) {
  if (item.kind === 'image') return item.title || item.preview || '图片剪贴板';
  if (item.kind === 'file') return item.preview || item.content.split(/\r?\n/).filter(Boolean).join('，');
  if (item.kind === 'rich_text') return item.preview || stripHtmlTags(item.content);
  return item.preview || item.content;
}

function stripHtmlTags(value: string) {
  return value.replace(/<[^>]*>/g, ' ').replace(/\s+/g, ' ').trim();
}

function kindLabel(kind?: string) {
  if (kind === 'image') return '图片';
  if (kind === 'file') return '文件';
  if (kind === 'rich_text') return '富文本';
  return '文本';
}

function formatTime(value?: string) {
  if (!value) return '';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '';
  return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
}

function resetIdleTimer() {
  window.clearTimeout(idleTimer);
  idleTimer = window.setTimeout(() => {
    void pasteQuickInputFromFloating(0, 0);
  }, 5000);
}

function markNativePointerOperation() {
  nativePointerOperationActive = true;
  window.clearTimeout(pointerOperationTimer);
  pointerOperationTimer = window.setTimeout(() => {
    nativePointerOperationActive = false;
  }, 1200);
}

function scheduleFocusHide(focused: boolean) {
  window.clearTimeout(focusHideTimer);
  if (focused) {
    return;
  }
  const hide = async () => {
    if (nativePointerOperationActive) {
      focusHideTimer = window.setTimeout(() => scheduleFocusHide(false), 80);
      return;
    }
    const current = getCurrentWindow();
    if (await current.isFocused()) return;
    await pasteQuickInputFromFloating(0, 0);
  };
  focusHideTimer = window.setTimeout(hide, 0);
}

let unlistenTrigger: (() => void) | null = null;
let unlistenFocus: (() => void) | null = null;
let unlistenQuickInputsUpdated: (() => void) | null = null;
let unlistenWindowSize: (() => void) | null = null;

function handlePointerUp() {
  nativePointerOperationActive = false;
}

onMounted(async () => {
  availableFontOptions.value = getAvailableFontOptions();
  settings.value = await getSettings();
  await refresh();
  window.addEventListener('keydown', onKeydown);
  window.addEventListener('pointerup', handlePointerUp);

  if ('__TAURI_INTERNALS__' in window) {
    unlistenWindowSize = await restoreAndTrackWindowSize('sheepclip:floating-window-size', {
      minWidth: 320,
      minHeight: 180,
      maxWidth: 1600,
      maxHeight: 1200,
    });
    unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload }) => scheduleFocusHide(payload));
    unlistenQuickInputsUpdated = await listen('quick-inputs-updated', async () => {
      if (activeSource.value === 'quick') {
        await refresh(searchQuery.value, triggerText.value, selectedIndex.value);
      }
    });
    unlistenTrigger = await listen<{ query: string; trigger: string; selectedIndex: number }>('floating-triggered', async (event) => {
      settings.value = await getSettings();
      activeSource.value = 'quick';
      ignoreSlashUntil = Date.now() + 300;
      await refresh('', event.payload.trigger, 0);
    });
  }
});

onUnmounted(() => {
  window.clearTimeout(idleTimer);
  window.clearTimeout(focusHideTimer);
  window.clearTimeout(pointerOperationTimer);
  window.removeEventListener('keydown', onKeydown);
  window.removeEventListener('pointerup', handlePointerUp);
  unlistenTrigger?.();
  unlistenFocus?.();
  unlistenQuickInputsUpdated?.();
  unlistenWindowSize?.();
});
</script>
