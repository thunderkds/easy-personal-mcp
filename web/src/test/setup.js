import '@testing-library/jest-dom/vitest'
import { cleanup, configure } from '@testing-library/react'
import { afterEach } from 'vitest'

// Full-suite load can push a first render past the 1 s default wait.
configure({ asyncUtilTimeout: 3000 })

afterEach(cleanup)
