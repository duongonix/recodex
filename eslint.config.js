import js from "@eslint/js";
import { defineConfig } from "eslint/config";
import globals from "globals";
import svelte from "eslint-plugin-svelte";
import ts from "typescript-eslint";
import { loadConfig } from '@sveltejs/load-config';

const svelteConfig = (await loadConfig('./', { traverse: false }))?.config;

export default defineConfig(
  { ignores: ["**/.svelte-kit/**", "**/build/**", "**/src-tauri/target/**"] },
  js.configs.recommended,
  ts.configs.recommended,
  svelte.configs.recommended,
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
      parserOptions: {
        extraFileExtensions: [".svelte"],
      },
    },
  },
  {
    files: ["**/*.svelte"],
    languageOptions: {
      parserOptions: {
        parser: ts.parser,
        svelteConfig,
      },
    },
  },
  {
    files: ["**/*.svelte.ts"],
    languageOptions: {
      parser: ts.parser,
    },
  },
);
