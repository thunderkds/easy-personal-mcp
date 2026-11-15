import '@testing-library/jest-dom/vitest'
import { cleanup, configure } from '@testing-library/react'
import { afterEach } from 'vitest'

// Full-suite load can push a first render past the 1 s default wait (HUN-121: 3 s; T082: 10 s at load avg ~40).
configure({ asyncUtilTimeout: 10000 })

afterEach(cleanup)
