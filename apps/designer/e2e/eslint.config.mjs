import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import prettier from 'eslint-config-prettier';

export default tseslint.config(
  { ignores: ['target/', 'dist/', 'dist-web/', 'node_modules/', '.claude/'] },
  {
    files: [
      'tests/**/*.ts',
      'tests-web/**/*.ts',
      'playwright.config.ts',
      'playwright.web.config.ts',
    ],
    extends: [js.configs.recommended, ...tseslint.configs.recommended, prettier],
  },
);
