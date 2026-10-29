import { render, screen, fireEvent, within } from '@testing-library/react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import App from './App.jsx'
import { fakeFetch } from './test/fixtures.js'

beforeEach(() => {
  window.location.hash = '#secret-token'
  vi.stubGlobal('fetch', fakeFetch())
})
afterEach(() => vi.unstubAllGlobals())

const nav = () => screen.getByRole('navigation', { name: 'Views' })

it('opens on Usage and switches in-page between the views without changing the URL fragment', async () => {
  render(<App />)
  expect(await screen.findByRole('heading', { name: 'Totals' })).toBeInTheDocument()
  expect(within(nav()).getByRole('button', { name: 'Usage' })).toHaveAttribute('aria-current', 'page')
  fireEvent.click(within(nav()).getByRole('button', { name: 'Warnings & plan' }))
  expect(await screen.findByRole('heading', { name: 'Warnings' })).toBeInTheDocument()
  fireEvent.click(within(nav()).getByRole('button', { name: 'Kit' }))
  expect(await screen.findByRole('heading', { name: 'Kit by project' })).toBeInTheDocument()
  expect(window.location.hash).toBe('#secret-token')
})

it('lists the 5 views in the sidebar and marks only the active one', () => {
  render(<App />)
  const buttons = within(nav()).getAllByRole('button')
  expect(buttons.map((b) => b.textContent)).toEqual(['Usage', 'Warnings & plan', 'Kit', 'Workspaces', 'Agents'])
  fireEvent.click(within(nav()).getByRole('button', { name: 'Kit' }))
  expect(within(nav()).getByRole('button', { name: 'Kit' })).toHaveAttribute('aria-current', 'page')
  expect(within(nav()).getByRole('button', { name: 'Usage' })).not.toHaveAttribute('aria-current')
})

it('shows the active view title in the top bar', async () => {
  render(<App />)
  const bar = screen.getByRole('banner')
  expect(within(bar).getByText('Usage')).toBeInTheDocument()
  fireEvent.click(within(nav()).getByRole('button', { name: 'Kit' }))
  expect(within(bar).getByText('Kit')).toBeInTheDocument()
  expect(await screen.findByRole('heading', { name: 'Kit by project' })).toBeInTheDocument()
})

it('has a menu button that toggles aria-expanded and closes after choosing a view', () => {
  render(<App />)
  const menu = screen.getByRole('button', { name: 'Menu' })
  expect(menu).toHaveAttribute('aria-expanded', 'false')
  fireEvent.click(menu)
  expect(menu).toHaveAttribute('aria-expanded', 'true')
  fireEvent.click(within(nav()).getByRole('button', { name: 'Agents' }))
  expect(menu).toHaveAttribute('aria-expanded', 'false')
})

it('puts the Usage filters in the top bar', async () => {
  render(<App />)
  await screen.findByRole('heading', { name: 'Totals' })
  const bar = screen.getByRole('banner')
  expect(await within(bar).findByLabelText('Workspace')).toBeInTheDocument()
  expect(within(bar).getByLabelText('Account')).toBeInTheDocument()
})
