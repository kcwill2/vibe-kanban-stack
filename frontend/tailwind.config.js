/** @type {import('tailwindcss').Config}
 * Default config so Tailwind always has content when no @config is in scope.
 * Actual styling uses tailwind.new.config.js and tailwind.legacy.config.js via @config in CSS.
 */
module.exports = {
  content: [
    './index.html',
    './src/**/*.{js,ts,jsx,tsx}',
    './pages/**/*.{ts,tsx}',
    './components/**/*.{ts,tsx}',
    './app/**/*.{ts,tsx}',
    'node_modules/@rjsf/shadcn/src/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: { extend: {} },
  plugins: [],
};
