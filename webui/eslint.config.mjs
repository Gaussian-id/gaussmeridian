import { defineConfig, globalIgnores } from "eslint/config";
import nextVitals from "eslint-config-next/core-web-vitals";
import nextTs from "eslint-config-next/typescript";
import importPlugin from "eslint-plugin-import";
import jsxA11y from "eslint-plugin-jsx-a11y";

/**
 * Gaussian front-end standards.
 * - Next.js core-web-vitals + TypeScript baseline
 * - jsx-a11y for accessibility (enterprise / auditable surfaces)
 * - import/order for a predictable, reviewable import block
 * Tailwind class sorting is handled by prettier-plugin-tailwindcss
 * (eslint-plugin-tailwindcss does not yet support Tailwind v4).
 */
const eslintConfig = defineConfig([
  ...nextVitals,
  ...nextTs,
  {
    // jsx-a11y plugin is already registered by eslint-config-next; reuse it and
    // enable the full recommended rule set without redefining the plugin.
    plugins: { import: importPlugin },
    rules: {
      ...jsxA11y.flatConfigs.recommended.rules,
      "import/order": [
        "warn",
        {
          groups: ["builtin", "external", "internal", "parent", "sibling", "index", "type"],
          pathGroups: [
            { pattern: "@core/**", group: "internal", position: "before" },
            { pattern: "@theme/**", group: "internal", position: "before" },
            { pattern: "@/**", group: "internal" },
          ],
          "newlines-between": "always",
          alphabetize: { order: "asc", caseInsensitive: true },
        },
      ],
      "no-console": ["warn", { allow: ["warn", "error"] }],
      "@typescript-eslint/no-unused-vars": [
        "warn",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
    },
  },
  {
    // The logger is the single sanctioned console-caller; everywhere else `no-console` stays strict
    // so code routes through it. It logs arbitrary `data`, so `any` is intentional here.
    files: ["src/utils/logger.ts"],
    rules: {
      "no-console": "off",
      "@typescript-eslint/no-explicit-any": "off",
    },
  },
  globalIgnores([".next/**", "out/**", "build/**", "docs/dogfood-output/**", "next-env.d.ts"]),
]);

export default eslintConfig;
