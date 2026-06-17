export type ThemeKey = 'warm' | 'blue' | 'mint' | 'graphite' | 'violet';
export type FontKey = 'system' | 'microsoft-yahei' | 'simhei' | 'simsun' | 'kaiti';

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
];

const DEFAULT_THEME_KEY: ThemeKey = 'warm';
const DEFAULT_FONT_KEY: FontKey = 'system';

export function resolveThemeKey(key?: string): ThemeKey {
  return THEME_OPTIONS.some((theme) => theme.key === key) ? key as ThemeKey : DEFAULT_THEME_KEY;
}

export function resolveFontKey(key?: string, availableKeys: string[] = FONT_OPTIONS.map((font) => font.key)): FontKey {
  if (!key) return DEFAULT_FONT_KEY;
  return FONT_OPTIONS.some((font) => font.key === key) && availableKeys.includes(key) ? key as FontKey : DEFAULT_FONT_KEY;
}

export function getThemeStyle(
  themeKey: string,
  fontKey: string,
  availableFonts: FontOption[] = getAvailableFontOptions(),
): Record<string, string> {
  const theme = THEME_OPTIONS.find((item) => item.key === resolveThemeKey(themeKey)) ?? THEME_OPTIONS[0];
  const font = availableFonts.find((item) => item.key === resolveFontKey(fontKey, availableFonts.map((item) => item.key))) ?? FONT_OPTIONS[0];

  return {
    '--app-font-family': font.family,
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
