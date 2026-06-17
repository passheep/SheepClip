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
  assert.equal(resolveThemeKey('missing'), 'warm');
  assert.equal(resolveThemeKey(undefined), 'warm');
});

test('getAvailableFontOptions hides unavailable built-in fonts but keeps system', () => {
  const fonts = getAvailableFontOptions((fontFamily) => fontFamily === 'Microsoft YaHei');
  assert.deepEqual(fonts.map((font) => font.key), ['system', 'microsoft-yahei']);
});

test('resolveFontKey allows only currently available font keys', () => {
  assert.equal(resolveFontKey('simhei', ['system', 'simhei']), 'simhei');
  assert.equal(resolveFontKey('kaiti', ['system', 'simhei']), 'system');
  assert.equal(resolveFontKey('missing', ['system', 'simhei']), 'system');
});

test('getThemeStyle returns css variables for selected theme and font', () => {
  const style = getThemeStyle('blue', 'simhei', FONT_OPTIONS);
  assert.equal(style['--color-primary'], '#2563eb');
  assert.match(style['--app-font-family'], /SimHei/);
});
