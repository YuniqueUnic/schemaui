import type { Config } from "tailwindcss";

/**
 * Tailwind CSS 4 Configuration
 *
 * Note: In Tailwind 4, theme configuration is done in CSS using @theme directive.
 * This config file is minimal - colors and other design tokens are defined in globals.css
 */
const config: Partial<Config> = {
  darkMode: "class",
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
};

export default config;
