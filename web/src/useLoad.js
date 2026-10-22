import { useEffect, useState } from 'react'

/** Run `load()` once on mount; returns {status: 'loading'|'ready'|'error', data, message}. */
export function useLoad(load) {
  const [state, setState] = useState({ status: 'loading' })
  useEffect(() => {
    let current = true
    load().then(
      (data) => current && setState({ status: 'ready', data }),
      (error) => current && setState({ status: 'error', message: error.message }),
    )
    return () => {
      current = false
    }
  }, [])
  return state
}
