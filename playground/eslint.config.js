import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import prettier from 'eslint-config-prettier';

/** @type {import("eslint").Linter.Config[]} */
const config = [
  js.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  ...svelte.configs['flat/recommended'],
  prettier,
  ...svelte.configs['flat/prettier'],
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
        extraFileExtensions: ['.svelte'],
      },
    },
  },
  {
    files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
  },
  {
    files: ['src/**/*.{ts,svelte}'],
    rules: {
      // TS/svelte-check ya reportan identificadores no definidos; no-undef no
      // conoce los globals del browser/worker y daría falsos positivos.
      'no-undef': 'off',
      // Handlers de eventos Svelte: `onclick={() => fn()}` es idiomático.
      '@typescript-eslint/no-confusing-void-expression': ['error', { ignoreArrowShorthand: true }],
      // Números en template literals (tamaños, dimensiones) son legítimos.
      '@typescript-eslint/restrict-template-expressions': ['error', { allowNumber: true }],
    },
  },
  // Config/tooling JS files: disable type-aware rules (not in tsconfig project)
  {
    files: ['*.config.js', '*.config.ts', 'eslint.config.js'],
    ...tseslint.configs.disableTypeChecked,
  },
  {
    ignores: ['dist/**', 'node_modules/**', 'src/lib/wasm/**', 'src/lib/components/ui/**'],
  },
];

export default config;
