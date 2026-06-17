# Theme Menu Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a left-side Theme menu where users can switch built-in color themes and choose only locally available built-in fonts.

**Architecture:** Keep theme definitions and font detection in focused frontend utility modules, then wire selected `theme_key` and `font_key` through the existing Tauri `AppSettings` persistence path. `App.vue` applies theme CSS variables to the root view and renders a new theme page with CSS-only preview cards.

**Tech Stack:** Vue 3, TypeScript, Tauri 2, Rust, SQLite, Tailwind CSS, Node built-in test runner.

## Global Constraints

- Add a left-side `主题` menu, not a custom landing page.
- Color choices are built-in themes only; do not add a custom color picker.
- Theme cards use simple `div` and CSS previews, not image assets.
- Font choices are built-in candidates; unavailable local fonts must not display or save.
- Preserve existing clipboard, quick input, tray, hotkey, and onboarding behavior.
- Use existing `AppSettings` persistence; do not add a new state-management library.

---

### Task 1: Theme and Font Utility Rules

**Files:**
- Create: `src/theme.ts`
- Create: `src/theme.test.mjs`
- Modify: `package.json`

**Interfaces:**
- Produces: `THEME_OPTIONS`, `FONT_OPTIONS`, `resolveThemeKey(key?: string): ThemeKey`, `resolveFontKey(key?: string, availableKeys?: string[]): FontKey`, `getThemeStyle(themeKey: string, fontKey: string, availableFonts?: FontOption[]): Record<string, string>`, `getAvailableFontOptions(detector?: (fontFamily: string) => boolean): FontOption[]`
- Consumes: no application runtime state.

- [ ] **Step 1: Write failing tests**

```js
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm run test:theme`

Expected: FAIL because `src/theme.ts`, `build:theme-test`, or `test:theme` does not exist yet.

- [ ] **Step 3: Implement utilities and test scripts**

Create `src/theme.ts` with built-in theme definitions, built-in font definitions, key fallback helpers, CSS variable generation, and `isFontAvailable` canvas detection.

Update `package.json` scripts:

```json
"build:theme-test": "tsc src/theme.ts --target ES2020 --module ES2020 --moduleResolution Node --outDir src/theme.dist --skipLibCheck",
"test:theme": "npm run build:theme-test && node src/theme.test.mjs"
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm run test:theme`

Expected: PASS with 4 passing tests.

- [ ] **Step 5: Commit**

```bash
git add package.json src/theme.ts src/theme.test.mjs
git commit -m "test: cover theme option rules"
```

### Task 2: Persist Theme Settings Through Tauri

**Files:**
- Modify: `src/types.ts`
- Modify: `src/lib/commands.ts`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: frontend settings use `theme_key: string` and `font_key: string`.
- Produces: Rust `AppSettings` reads, sanitizes, and writes `theme_key` and `font_key`.

- [ ] **Step 1: Write failing tests**

Add Rust unit tests in `src-tauri/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_settings_falls_back_unknown_theme_and_font() {
        let mut settings = recommended_settings();
        settings.theme_key = "unknown".into();
        settings.font_key = "unknown".into();

        let sanitized = sanitize_settings(settings);

        assert_eq!(sanitized.theme_key, "warm");
        assert_eq!(sanitized.font_key, "system");
    }

    #[test]
    fn settings_entries_include_theme_fields() {
        let settings = recommended_settings();
        let entries = settings_entries(&settings);

        assert!(entries.iter().any(|(key, value)| *key == "theme_key" && value == "warm"));
        assert!(entries.iter().any(|(key, value)| *key == "font_key" && value == "system"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test`

Expected: FAIL because `theme_key` and `font_key` do not exist yet.

- [ ] **Step 3: Implement settings persistence**

Add `theme_key` and `font_key` to TypeScript `AppSettings`, frontend fallback settings, Rust `AppSettings`, `read_settings`, `recommended_settings`, `sanitize_settings`, and `settings_entries`.

Allowed Rust values:

```rust
const THEME_KEYS: [&str; 5] = ["warm", "blue", "mint", "graphite", "violet"];
const FONT_KEYS: [&str; 5] = ["system", "microsoft-yahei", "simhei", "simsun", "kaiti"];
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/types.ts src/lib/commands.ts src-tauri/src/lib.rs
git commit -m "feat: persist theme settings"
```

### Task 3: Theme Page UI and Runtime Styling

**Files:**
- Modify: `src/App.vue`
- Modify: `src/styles.css`
- Modify: `tailwind.config.js`

**Interfaces:**
- Consumes: `THEME_OPTIONS`, `FONT_OPTIONS`, `getAvailableFontOptions`, `getThemeStyle`, `resolveFontKey`, `resolveThemeKey` from `src/theme.ts`.
- Produces: a new `theme` primary view and CSS variable driven runtime styling.

- [ ] **Step 1: Write failing checks**

Run: `npm run build`

Expected before implementation: PASS currently, then after the next step starts imports and view wiring, TypeScript should enforce all references. No separate component test exists in the project.

- [ ] **Step 2: Implement UI**

Update `App.vue`:

- Add a lucide icon import for the theme menu.
- Add `{ key: 'theme', label: '主题', icon: Palette }` to nav items.
- Add a `theme` view rendering color theme cards and available font cards.
- Apply `:style="appThemeStyle"` to the root `<main>`.
- Use `getAvailableFontOptions()` on mount.
- Resolve unknown or unavailable saved settings to defaults.
- Replace major hard-coded theme colors with CSS-variable backed classes or inline styles.

Update `styles.css` so `:root` and `body` use CSS variable defaults, including `font-family: var(--app-font-family)`.

Update `tailwind.config.js` extended colors to map app tokens to CSS variables.

- [ ] **Step 3: Run frontend tests and build**

Run: `npm run test:theme`

Expected: PASS.

Run: `npm --cache C:\000Program\SheepClip\.npm-cache run build`

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/App.vue src/styles.css tailwind.config.js src/theme.ts
git commit -m "feat: add theme menu"
```

### Task 4: Final Verification

**Files:**
- No planned source edits.

**Interfaces:**
- Consumes all previous tasks.
- Produces verified final state.

- [ ] **Step 1: Run focused theme tests**

Run: `npm run test:theme`

Expected: PASS.

- [ ] **Step 2: Run frontend build**

Run: `npm --cache C:\000Program\SheepClip\.npm-cache run build`

Expected: PASS.

- [ ] **Step 3: Run Rust check**

Run: `cargo check`

Expected: PASS.

- [ ] **Step 4: Inspect git diff/status**

Run: `git status --short`

Expected: clean or only intentional uncommitted files if final polish is needed.
