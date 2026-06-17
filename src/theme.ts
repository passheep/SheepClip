export type ThemeKey = 'warm' | 'blue' | 'mint' | 'graphite' | 'violet' | 'dark';
export type FontKey = 'system' | 'microsoft-yahei' | 'simhei' | 'simsun' | 'kaiti' | 'fangsong';

export interface ThemeOption {
  key: ThemeKey;
  name: string;
  description: string;
  colors: {
    ink: string;
    panel: string;
    sidebar: string;
    header: string;
    line: string;
    primary: string;
    primarySoft: string;
    primaryMuted: string;
    danger: string;
    card: string;
    muted: string;
    scrollbarThumb: string;
    scrollbarTrack: string;
  };
}

export interface FontOption {
  key: FontKey;
  name: string;
  sample: string;
  family: string;
  detectFamily?: string;
}

export const THEME_OPTIONS: ThemeOption[] = [
  {
    key: 'warm',
    name: '默认暖灰',
    description: '延续 SheepClip 当前的温和浅色观感。',
    colors: {
      ink: '#16181d',
      panel: '#f7f7f4',
      sidebar: '#eceae2',
      header: '#fbfaf6',
      line: '#dedbd2',
      primary: '#2f8f71',
      primarySoft: '#edf3f5',
      primaryMuted: '#8bb4c5',
      danger: '#b25f3a',
      card: '#ffffff',
      muted: '#57534e',
      scrollbarThumb: '#bcb5aa',
      scrollbarTrack: '#eeeae2',
    },
  },
  {
    key: 'blue',
    name: '清爽蓝',
    description: '更偏办公工具感，界面清透利落。',
    colors: {
      ink: '#111827',
      panel: '#f4f8ff',
      sidebar: '#e8f0fb',
      header: '#f8fbff',
      line: '#d4e0ef',
      primary: '#2563eb',
      primarySoft: '#e8f1ff',
      primaryMuted: '#8bb8f7',
      danger: '#dc633a',
      card: '#ffffff',
      muted: '#526174',
      scrollbarThumb: '#a8bfdc',
      scrollbarTrack: '#e3edf8',
    },
  },
  {
    key: 'mint',
    name: '薄荷绿',
    description: '更轻快，适合长时间整理剪贴板。',
    colors: {
      ink: '#13201b',
      panel: '#f3fbf6',
      sidebar: '#e4f4eb',
      header: '#f8fdf9',
      line: '#cde6d8',
      primary: '#14845f',
      primarySoft: '#e3f5ed',
      primaryMuted: '#7fc8a6',
      danger: '#c05f3d',
      card: '#ffffff',
      muted: '#4f635b',
      scrollbarThumb: '#9acdb4',
      scrollbarTrack: '#dcefe5',
    },
  },
  {
    key: 'graphite',
    name: '石墨灰',
    description: '更克制，减少界面色彩干扰。',
    colors: {
      ink: '#14171a',
      panel: '#f3f4f5',
      sidebar: '#e5e7eb',
      header: '#fafafa',
      line: '#d6d9de',
      primary: '#374151',
      primarySoft: '#eceff3',
      primaryMuted: '#9aa3af',
      danger: '#b4533a',
      card: '#ffffff',
      muted: '#565d66',
      scrollbarThumb: '#b5bac2',
      scrollbarTrack: '#e5e8ec',
    },
  },
  {
    key: 'violet',
    name: '柔和紫',
    description: '少量紫色点缀，整体仍保持轻盈。',
    colors: {
      ink: '#1d1724',
      panel: '#faf7fd',
      sidebar: '#f0e8f8',
      header: '#fdfaff',
      line: '#e2d5ec',
      primary: '#7c3aed',
      primarySoft: '#f1e9ff',
      primaryMuted: '#b69af1',
      danger: '#bd5a4a',
      card: '#ffffff',
      muted: '#63586e',
      scrollbarThumb: '#c6afd9',
      scrollbarTrack: '#f0e6f7',
    },
  },
  {
    key: 'dark',
    name: '黑暗模式',
    description: '深色背景和低亮度边界，适合夜间使用。',
    colors: {
      ink: '#f2f4f8',
      panel: '#111318',
      sidebar: '#171a21',
      header: '#1a1e26',
      line: '#323844',
      primary: '#60a5fa',
      primarySoft: '#1f2f45',
      primaryMuted: '#375f8f',
      danger: '#f9735b',
      card: '#1f232b',
      muted: '#a8b0bd',
      scrollbarThumb: '#4b5567',
      scrollbarTrack: '#171a21',
    },
  },
];

export const FONT_OPTIONS: FontOption[] = [
  {
    key: 'system',
    name: '系统默认',
    sample: '系统默认字体',
    family: 'Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", "Microsoft YaHei", sans-serif',
  },
  {
    key: 'microsoft-yahei',
    name: '微软雅黑',
    sample: '微软雅黑',
    family: '"Microsoft YaHei", "微软雅黑", sans-serif',
    detectFamily: 'Microsoft YaHei',
  },
  {
    key: 'simhei',
    name: '黑体',
    sample: '黑体',
    family: 'SimHei, "黑体", sans-serif',
    detectFamily: 'SimHei',
  },
  {
    key: 'simsun',
    name: '宋体',
    sample: '宋体',
    family: 'SimSun, "宋体", serif',
    detectFamily: 'SimSun',
  },
  {
    key: 'kaiti',
    name: '楷体',
    sample: '楷体',
    family: 'KaiTi, "楷体", serif',
    detectFamily: 'KaiTi',
  },
  {
    key: 'fangsong',
    name: '仿宋',
    sample: '仿宋',
    family: 'FangSong, "FangSong_GB2312", "仿宋", serif',
    detectFamily: 'FangSong',
  },
];

const DEFAULT_THEME_KEY: ThemeKey = 'warm';
const DEFAULT_FONT_KEY: FontKey = 'system';
export const DEFAULT_FONT_SIZE = 14;
export const MIN_FONT_SIZE = 12;
export const MAX_FONT_SIZE = 18;
export const DEFAULT_FONT_WEIGHT = 400;
export const FONT_WEIGHT_OPTIONS = [400, 500, 600] as const;
export type FontWeightValue = typeof FONT_WEIGHT_OPTIONS[number];

export function resolveThemeKey(key?: string): ThemeKey {
  return THEME_OPTIONS.some((theme) => theme.key === key) ? key as ThemeKey : DEFAULT_THEME_KEY;
}

export function resolveFontKey(key?: string, availableKeys: string[] = FONT_OPTIONS.map((font) => font.key)): FontKey {
  if (!key) return DEFAULT_FONT_KEY;
  return FONT_OPTIONS.some((font) => font.key === key) && availableKeys.includes(key) ? key as FontKey : DEFAULT_FONT_KEY;
}

export function resolveFontSize(value?: number): number {
  if (!Number.isFinite(value)) return DEFAULT_FONT_SIZE;
  return Math.min(Math.max(Math.round(value as number), MIN_FONT_SIZE), MAX_FONT_SIZE);
}

export function resolveFontWeight(value?: number): FontWeightValue {
  const normalized = Number(value);
  if (!Number.isFinite(normalized)) return DEFAULT_FONT_WEIGHT;
  return FONT_WEIGHT_OPTIONS.reduce((closest, option) => {
    return Math.abs(option - normalized) < Math.abs(closest - normalized) ? option : closest;
  }, DEFAULT_FONT_WEIGHT as FontWeightValue);
}

function clampFontSize(value: number): number {
  return Math.min(Math.max(value, MIN_FONT_SIZE), MAX_FONT_SIZE);
}

function resolveFontScale(fontSize: number) {
  return {
    xs: clampFontSize(fontSize - 2),
    sm: clampFontSize(fontSize),
    base: clampFontSize(fontSize + 2),
    lg: clampFontSize(fontSize + 4),
  };
}

function resolveFontWeightScale(fontWeight: FontWeightValue) {
  return {
    normal: fontWeight,
    medium: fontWeight === 400 ? 500 : 600,
    semibold: fontWeight === 600 ? 700 : 600,
    bold: fontWeight === 600 ? 700 : 600,
  };
}

export function getThemeStyle(
  themeKey: string,
  fontKey: string,
  fontSize = DEFAULT_FONT_SIZE,
  fontWeight = DEFAULT_FONT_WEIGHT,
  availableFonts: FontOption[] = getAvailableFontOptions(),
): Record<string, string> {
  const theme = THEME_OPTIONS.find((item) => item.key === resolveThemeKey(themeKey)) ?? THEME_OPTIONS[0];
  const font = availableFonts.find((item) => item.key === resolveFontKey(fontKey, availableFonts.map((item) => item.key))) ?? FONT_OPTIONS[0];
  const resolvedFontSize = resolveFontSize(fontSize);
  const resolvedFontWeight = resolveFontWeight(fontWeight);
  const fontScale = resolveFontScale(resolvedFontSize);
  const fontWeightScale = resolveFontWeightScale(resolvedFontWeight);

  return {
    fontFamily: font.family,
    fontSize: `${resolvedFontSize}px`,
    fontWeight: String(resolvedFontWeight),
    '--app-font-family': font.family,
    '--app-font-size': `${resolvedFontSize}px`,
    '--app-font-size-xs': `${fontScale.xs}px`,
    '--app-font-size-sm': `${fontScale.sm}px`,
    '--app-font-size-base': `${fontScale.base}px`,
    '--app-font-size-lg': `${fontScale.lg}px`,
    '--app-font-weight': String(resolvedFontWeight),
    '--app-font-weight-normal': String(fontWeightScale.normal),
    '--app-font-weight-medium': String(fontWeightScale.medium),
    '--app-font-weight-semibold': String(fontWeightScale.semibold),
    '--app-font-weight-bold': String(fontWeightScale.bold),
    '--color-ink': theme.colors.ink,
    '--color-panel': theme.colors.panel,
    '--color-sidebar': theme.colors.sidebar,
    '--color-header': theme.colors.header,
    '--color-line': theme.colors.line,
    '--color-primary': theme.colors.primary,
    '--color-primary-soft': theme.colors.primarySoft,
    '--color-primary-muted': theme.colors.primaryMuted,
    '--color-danger': theme.colors.danger,
    '--color-card': theme.colors.card,
    '--color-muted': theme.colors.muted,
    '--color-scrollbar-thumb': theme.colors.scrollbarThumb,
    '--color-scrollbar-track': theme.colors.scrollbarTrack,
  };
}

export function getAvailableFontOptions(detector: (fontFamily: string) => boolean = isFontAvailable): FontOption[] {
  return FONT_OPTIONS.filter((font) => !font.detectFamily || detector(font.detectFamily));
}

export function isFontAvailable(fontFamily: string): boolean {
  if (typeof document === 'undefined') return false;

  const sample = 'SheepClip 字体检测 0123456789';
  const testFonts = ['monospace', 'serif', 'sans-serif'];
  const canvas = document.createElement('canvas');
  const context = canvas.getContext('2d');
  if (!context) return false;

  return testFonts.some((fallback) => {
    context.font = `72px ${fallback}`;
    const fallbackWidth = context.measureText(sample).width;
    context.font = `72px "${fontFamily}", ${fallback}`;
    return context.measureText(sample).width !== fallbackWidth;
  });
}
