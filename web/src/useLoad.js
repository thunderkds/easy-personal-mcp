import { useEffect, useState } from 'react'

/** Run `load()` on mount and again when `deps` change (the older answer is dropped); returns {status: 'loading'|'ready'|'error', data, message}. */
export function useLoad(load, deps = []) {
  const [state, setState] = useState({ status: 'loading' })
  useEffect(() => {
    let current = true
    setState((s) => (s.status === 'loading' ? s : { status: 'loading' }))
    load().then(
      (data) => current && setState({ status: 'ready', data }),
      (error) => current && setState({ status: 'error', message: error.message }),
    )
    return () => {
      current = false
    }
  }, deps)
  return state
}
