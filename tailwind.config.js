/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{vue,ts}'],
  theme: {
    extend: {
      colors: {
        ink: 'var(--color-ink)',
        panel: 'var(--color-panel)',
        line: 'var(--color-line)',
        mint: 'var(--color-primary)',
        rust: 'var(--color-danger)',
        sidebar: 'var(--color-sidebar)',
        header: 'var(--color-header)',
        card: 'var(--color-card)',
        muted: 'var(--color-muted)',
        primarySoft: 'var(--color-primary-soft)',
        primaryMuted: 'var(--color-primary-muted)',
      },
      boxShadow: {
        soft: '0 18px 48px rgba(22, 24, 29, 0.12)',
      },
    },
  },
  plugins: [],
};
