import { defineConfig, mergeConfig } from 'vitest/config'
import viteConfig from './vite.config.js'

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: 'jsdom',
      setupFiles: ['./src/test/setup.js'],
      globals: false,
      // A loaded machine (load avg ~40) pushed whole-view tests past the 5 s default (T082); a real hang still fails.
      testTimeout: 20000,
    },
  }),
)
