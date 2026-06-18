import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  FONT_OPTIONS,
  getAvailableFontOptions,
  getThemeStyle,
  resolveFontKey,
  resolveThemeKey,
} from './theme.dist/theme.js';

test('resolveThemeKey falls back to warm for unknown keys', () => {
  assert.equal(resolveThemeKey('blue'), 'blue');
  assert.equal(resolveThemeKey('dark'), 'dark');
  assert.equal(resolveThemeKey('missing'), 'warm');
  assert.equal(resolveThemeKey(undefined), 'warm');
});

test('getAvailableFontOptions hides unavailable built-in fonts but keeps system', () => {
  const fonts = getAvailableFontOptions((fontFamily) => ['Microsoft YaHei', 'FangSong'].includes(fontFamily));
  assert.deepEqual(fonts.map((font) => font.key), ['system', 'microsoft-yahei', 'fangsong']);
});

test('resolveFontKey allows only currently available font keys', () => {
  assert.equal(resolveFontKey('simhei', ['system', 'simhei']), 'simhei');
  assert.equal(resolveFontKey('kaiti', ['system', 'simhei']), 'system');
  assert.equal(resolveFontKey('missing', ['system', 'simhei']), 'system');
});

test('getThemeStyle returns css variables for selected theme and font', () => {
  const style = getThemeStyle('blue', 'simhei', 16, 600, FONT_OPTIONS);
  assert.equal(style['--color-primary'], '#2563eb');
  assert.match(style['--app-font-family'], /SimHei/);
  assert.match(style.fontFamily, /SimHei/);
  assert.equal(style.fontSize, '16px');
  assert.equal(style.fontWeight, '600');
  assert.equal(style['--app-font-size'], '16px');
  assert.equal(style['--app-font-size-xs'], '14px');
  assert.equal(style['--app-font-size-sm'], '16px');
  assert.equal(style['--app-font-size-base'], '18px');
  assert.equal(style['--app-font-weight-medium'], '600');
  assert.equal(style['--app-font-weight-semibold'], '700');
  assert.equal(style['--app-font-weight'], '600');
  assert.ok(style['--color-scrollbar-thumb']);
  assert.ok(style['--color-scrollbar-track']);
});

test('dark theme exposes dark surface colors', () => {
  const style = getThemeStyle('dark', 'system', 14, 400, FONT_OPTIONS);
  assert.equal(style['--color-panel'], '#111318');
  assert.equal(style['--color-card'], '#1f232b');
});

test('font size and weight resolve to safe ranges', () => {
  const small = getThemeStyle('warm', 'system', 8, 200, FONT_OPTIONS);
  assert.equal(small['--app-font-size'], '12px');
  assert.equal(small['--app-font-weight'], '400');

  const large = getThemeStyle('warm', 'system', 24, 900, FONT_OPTIONS);
  assert.equal(large['--app-font-size'], '18px');
  assert.equal(large['--app-font-weight'], '600');
});

test('global styles route utility text classes through appearance variables', () => {
  const css = readFileSync(new URL('./styles.css', import.meta.url), 'utf8');

  assert.match(css, /--app-font-size-sm/);
  assert.match(css, /\.text-sm[\s\S]*font-size:\s*var\(--app-font-size-sm\)/);
  assert.match(css, /\.text-base[\s\S]*font-size:\s*var\(--app-font-size-base\)/);
  assert.match(css, /\.font-medium[\s\S]*font-weight:\s*var\(--app-font-weight-medium\)/);
  assert.match(css, /\.font-semibold[\s\S]*font-weight:\s*var\(--app-font-weight-semibold\)/);
  assert.match(css, /\.rich-text-preview[\s\S]*font-size:\s*var\(--app-font-size-sm\)/);
});

test('scrollable views use theme-aware scrollbar styling', () => {
  const css = readFileSync(new URL('./styles.css', import.meta.url), 'utf8');
  const app = readFileSync(new URL('./App.vue', import.meta.url), 'utf8');
  const floating = readFileSync(new URL('./Floating.vue', import.meta.url), 'utf8');

  assert.match(css, /\[data-theme\][\s\S]*scrollbar-color:\s*var\(--color-scrollbar-thumb\)\s+var\(--color-scrollbar-track\)/);
  assert.match(css, /\[data-theme\][\s\S]*::-webkit-scrollbar-thumb[\s\S]*background:\s*var\(--color-scrollbar-thumb\)/);
  assert.match(css, /\[data-theme\][\s\S]*::-webkit-scrollbar-track[\s\S]*background:\s*var\(--color-scrollbar-track\)/);
  assert.match(app, /key="theme"[^>]*scroll-thin/);
  assert.match(app, /key="settings"[^>]*scroll-thin/);
  assert.match(app, /key="about"[^>]*scroll-thin/);
  assert.match(app, /overflow-auto bg-white p-4[^>]*scroll-thin|scroll-thin[^>]*overflow-auto bg-white p-4/);
  assert.match(app, /<dd class="font-medium">0\.19<\/dd>/);
  assert.match(floating, /ref="listRef"[^>]*scroll-thin/);
});

test('package metadata is updated to version 0.19.0', () => {
  const packageJson = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
  const packageLock = JSON.parse(readFileSync(new URL('../package-lock.json', import.meta.url), 'utf8'));
  const tauriConfig = JSON.parse(readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'));
  const cargoToml = readFileSync(new URL('../src-tauri/Cargo.toml', import.meta.url), 'utf8');
  const cargoLock = readFileSync(new URL('../src-tauri/Cargo.lock', import.meta.url), 'utf8');

  assert.equal(packageJson.version, '0.19.0');
  assert.equal(packageLock.version, '0.19.0');
  assert.equal(packageLock.packages[''].version, '0.19.0');
  assert.equal(tauriConfig.version, '0.19.0');
  assert.match(cargoToml, /^version = "0\.19\.0"$/m);
  assert.match(cargoLock, /name = "sheepclip"\r?\nversion = "0\.19\.0"/);
});
