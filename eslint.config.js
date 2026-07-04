import js from '@eslint/js'
import globals from 'globals'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import tseslint from 'typescript-eslint'
import { defineConfig, globalIgnores } from 'eslint/config'

export default defineConfig([
  globalIgnores([
    'dist',
    'src-tauri/target',
    // shadcn-owned components — never manually edited (CLAUDE.md), so never linted
    'src/components/ui',
  ]),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      js.configs.recommended,
      tseslint.configs.recommended,
      reactHooks.configs.flat['recommended-latest'],
      reactRefresh.configs.vite,
    ],
    languageOptions: {
      ecmaVersion: 2020,
      globals: globals.browser,
    },
    rules: {
      // Pre-existing sync-on-open effects (SettingsModal, AppToolbar,
      // GlossaryPanel); refactoring them is tracked separately in tasks/todo.md.
      'react-hooks/set-state-in-effect': 'warn',
      // columns.tsx intentionally co-exports column builders and cell
      // components; the rule only affects HMR granularity, not correctness.
      'react-refresh/only-export-components': 'warn',
    },
  },
])
