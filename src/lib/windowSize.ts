import {
  availableMonitors,
  getCurrentWindow,
  PhysicalPosition,
  PhysicalSize,
} from '@tauri-apps/api/window';

interface WindowBoundsOptions {
  minWidth: number;
  minHeight: number;
  maxWidth?: number;
  maxHeight?: number;
}

interface SavedWindowBounds {
  width: number;
  height: number;
  x?: number;
  y?: number;
}

function isTauriRuntime() {
  return Boolean('__TAURI_INTERNALS__' in window);
}

function normalizeBounds(value: unknown, options: WindowBoundsOptions): SavedWindowBounds | null {
  if (!value || typeof value !== 'object') return null;
  const raw = value as Partial<SavedWindowBounds>;
  const width = Math.round(Number(raw.width));
  const height = Math.round(Number(raw.height));
  const maxWidth = options.maxWidth ?? 3600;
  const maxHeight = options.maxHeight ?? 2400;
  if (!Number.isFinite(width) || !Number.isFinite(height)) return null;
  if (width < options.minWidth || height < options.minHeight) return null;
  return {
    width: Math.min(width, maxWidth),
    height: Math.min(height, maxHeight),
    x: Number.isFinite(raw.x) ? Math.round(Number(raw.x)) : undefined,
    y: Number.isFinite(raw.y) ? Math.round(Number(raw.y)) : undefined,
  };
}

function readSavedBounds(storageKey: string, options: WindowBoundsOptions) {
  try {
    return normalizeBounds(JSON.parse(localStorage.getItem(storageKey) || 'null'), options);
  } catch {
    return null;
  }
}

function saveBounds(storageKey: string, bounds: SavedWindowBounds, options: WindowBoundsOptions) {
  const normalized = normalizeBounds(bounds, options);
  if (!normalized) return;
  localStorage.setItem(storageKey, JSON.stringify(normalized));
}

async function isPositionVisible(bounds: SavedWindowBounds) {
  if (!Number.isFinite(bounds.x) || !Number.isFinite(bounds.y)) return false;
  const monitors = await availableMonitors();
  return monitors.some((monitor) => {
    const area = monitor.workArea || monitor;
    const centerX = Number(bounds.x) + bounds.width / 2;
    const centerY = Number(bounds.y) + bounds.height / 2;
    const left = area.position.x;
    const top = area.position.y;
    const right = left + area.size.width;
    const bottom = top + area.size.height;
    return centerX >= left && centerX <= right && centerY >= top && centerY <= bottom;
  });
}

export async function restoreAndTrackWindowBounds(storageKey: string, options: WindowBoundsOptions) {
  if (!isTauriRuntime()) return null;

  const appWindow = getCurrentWindow();
  const saved = readSavedBounds(storageKey, options);
  if (saved) {
    try {
      await appWindow.setSize(new PhysicalSize(saved.width, saved.height));
      if (await isPositionVisible(saved)) {
        await appWindow.setPosition(new PhysicalPosition(saved.x || 0, saved.y || 0));
      }
    } catch {
      // 权限不足或系统拒绝时不影响主流程。
    }
  }

  let resizeTimer = 0;
  let moveTimer = 0;
  let lastSize: SavedWindowBounds | null = saved;
  let lastPosition: Pick<SavedWindowBounds, 'x' | 'y'> = saved ? { x: saved.x, y: saved.y } : {};
  const persist = () => {
    if (!lastSize) return;
    saveBounds(storageKey, { ...lastSize, ...lastPosition }, options);
  };
  try {
    const [size, position] = await Promise.all([appWindow.innerSize(), appWindow.outerPosition()]);
    lastSize = { width: size.width, height: size.height };
    lastPosition = { x: position.x, y: position.y };
    persist();
  } catch {
    // 读取当前窗口信息失败时继续监听后续变化。
  }

  const unlistenResize = await appWindow.onResized(({ payload }) => {
    window.clearTimeout(resizeTimer);
    resizeTimer = window.setTimeout(() => {
      lastSize = { width: payload.width, height: payload.height };
      persist();
    }, 160);
  });
  const unlistenMove = await appWindow.onMoved(({ payload }) => {
    window.clearTimeout(moveTimer);
    moveTimer = window.setTimeout(() => {
      lastPosition = { x: payload.x, y: payload.y };
      persist();
    }, 160);
  });

  return () => {
    window.clearTimeout(resizeTimer);
    window.clearTimeout(moveTimer);
    unlistenResize();
    unlistenMove();
  };
}

export const restoreAndTrackWindowSize = restoreAndTrackWindowBounds;
