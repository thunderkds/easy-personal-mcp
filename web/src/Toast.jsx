import { createContext, useCallback, useContext, useEffect, useRef, useState } from 'react'

const MAX = 3
const HIDE_AFTER_MS = 5000
const Notify = createContext(null)

/** `notify('success' | 'error', text)`: a floating message. Success hides by itself; an error stays until it is closed. */
export function useNotify() {
  const notify = useContext(Notify)
  if (!notify) throw new Error('useNotify needs a ToastProvider above it')
  return notify
}

function Toast({ toast, onClose }) {
  const { id, kind, text } = toast
  useEffect(() => {
    if (kind === 'error') return undefined
    const timer = setTimeout(() => onClose(id), HIDE_AFTER_MS)
    return () => clearTimeout(timer)
  }, [id, kind, onClose])
  return (
    <div className={`toast toast-${kind}`} role={kind === 'error' ? 'alert' : 'status'}>
      <p className="toast-text">{text}</p>
      <button type="button" className="toast-close" aria-label="Close notification" onClick={() => onClose(id)}>
        ×
      </button>
    </div>
  )
}

/** One toast host for the whole page: bottom-right, at most 3 (the oldest goes first), newest last. */
export function ToastProvider({ children }) {
  const [toasts, setToasts] = useState([])
  const next = useRef(0)
  const notify = useCallback((kind, text) => {
    const toast = { id: ++next.current, kind, text }
    setToasts((all) => [...all, toast].slice(-MAX))
  }, [])
  const close = useCallback((id) => setToasts((all) => all.filter((t) => t.id !== id)), [])
  return (
    <Notify.Provider value={notify}>
      {children}
      <div className="toast-host" data-testid="toasts">
        {toasts.map((t) => (
          <Toast key={t.id} toast={t} onClose={close} />
        ))}
      </div>
    </Notify.Provider>
  )
}
