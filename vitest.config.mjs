import { defineConfig } from 'vitest/config'

// The include keeps vitest out of target/ and node_modules; JS and TS test files
// both run, transpiled by vitest itself with no tsconfig needed.
export default defineConfig({
  test: {
    include: ['npm/**/*.test.{js,mjs,ts,mts}'],
  },
})
