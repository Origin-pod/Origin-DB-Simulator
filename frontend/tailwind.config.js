/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        primary: {
          50: '#eff6ff',
          100: '#dbeafe',
          200: '#bfdbfe',
          300: '#93c5fd',
          400: '#60a5fa',
          500: '#3B82F6',
          600: '#2563eb',
          700: '#1d4ed8',
          800: '#1e40af',
          900: '#1e3a8a',
        },
        storage: '#8B5CF6',
        index: '#3B82F6',
        buffer: '#14B8A6',
        concurrency: '#F59E0B',
        execution: '#EC4899',
        transaction: '#6366F1',
        compression: '#84CC16',
        partitioning: '#F97316',
        optimization: '#06B6D4',
        distribution: '#A855F7',
        success: '#10B981',
        error: '#EF4444',
        warning: '#F59E0B',
        canvas: {
          bg: '#FAFAFA',
          grid: '#E5E7EB',
        }
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
    },
  },
  plugins: [],
}
