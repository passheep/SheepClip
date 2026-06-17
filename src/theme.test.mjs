import test from 'node:test';
import assert from 'node:assert/strict';
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
