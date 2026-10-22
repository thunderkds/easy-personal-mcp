import { render, screen, fireEvent } from '@testing-library/react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import App from './App.jsx'
import { fakeFetch } from './test/fixtures.js'

beforeEach(() => {
  window.location.hash = '#secret-token'
  vi.stubGlobal('fetch', fakeFetch())
})
afterEach(() => vi.unstubAllGlobals())

it('opens on Usage and switches in-page between the tabs without changing the URL fragment', async () => {
  render(<App />)
  expect(await screen.findByRole('heading', { name: 'Totals' })).toBeInTheDocument()
  expect(screen.getByRole('tab', { name: 'Usage' })).toHaveAttribute('aria-selected', 'true')
  fireEvent.click(screen.getByRole('tab', { name: 'Warnings & plan' }))
  expect(await screen.findByRole('heading', { name: 'Warnings' })).toBeInTheDocument()
  fireEvent.click(screen.getByRole('tab', { name: 'Kit' }))
  expect(await screen.findByRole('heading', { name: 'Kit by project' })).toBeInTheDocument()
  expect(window.location.hash).toBe('#secret-token')
})
