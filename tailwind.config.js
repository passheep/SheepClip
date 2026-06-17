/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{vue,ts}'],
  theme: {
    extend: {
      colors: {
        ink: '#16181d',
        panel: '#f7f7f4',
        line: '#dedbd2',
        mint: '#2f8f71',
        rust: '#b25f3a',
      },
      boxShadow: {
        soft: '0 18px 48px rgba(22, 24, 29, 0.12)',
      },
    },
  },
  plugins: [],
};
