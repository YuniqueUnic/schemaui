import type { Config } from 'tailwindcss';

const config: Config = {
  darkMode: 'class',
  safelist: [
    'text-cyan-400',
    'text-cyan-300',
    'text-emerald-300',
    'text-emerald-200',
    'text-orange-300',
    'text-orange-200',
    'text-rose-300',
    'text-rose-200',
    'text-sky-300',
    'text-sky-200',
  ],
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        brand: {
          50: '#f0f9ff',
          100: '#e0f2fe',
          200: '#bae6fd',
          300: '#7dd3fc',
          400: '#38bdf8',
          500: '#0ea5e9',
          600: '#0284c7',
          700: '#0369a1',
          800: '#075985',
          900: '#0c4a6e',
        },
      },
      boxShadow: {
        shell: '0 30px 80px rgba(3,7,18,0.55)',
      },
      fontFamily: {
        sans: ['InterVariable', 'Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'SFMono-Regular', 'Menlo', 'monospace'],
      },
      keyframes: {
        'fade-in': {
          from: { opacity: '0', transform: 'translateY(4px)' },
          to: { opacity: '1', transform: 'translateY(0px)' },
        },
      },
      animation: {
        'fade-in': 'fade-in 0.16s ease-out',
      },
    },
  },
  plugins: [],
};

export default config;
