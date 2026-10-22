import { useState } from 'react'
import { getJson, sendJson } from './api.js'
import { useLoad } from './useLoad.js'

const load = () => getJson('/api/connectors')

function groupByCategory(connectors) {
  const groups = []
  for (const c of connectors) {
    const last = groups[groups.length - 1]
    if (last && last.category === c.category) last.items.push(c)
    else groups.push({ category: c.category, items: [c] })
  }
  return groups
}

/** One provider: status, claude.ai link first, a form built from its catalog fields, a confirmation, then write. */
function Provider({ connector, onChanged }) {
  const { id, provider, configured, claude_ai_connector: official, claude_ai_link: link, fields } = connector
  // mode: 'idle' | 'form' | 'plan'; action: 'add' | 'remove'
  const [mode, setMode] = useState('idle')
  const [action, setAction] = useState('add')
  const [values, setValues] = useState({})
  const [lines, setLines] = useState([])
  const [result, setResult] = useState(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState(null)

  const request = (confirm) =>
    action === 'add'
      ? sendJson('POST', '/api/connectors', { provider: id, values, confirm })
      : sendJson('DELETE', `/api/connectors/${encodeURIComponent(id)}`, { confirm })

  async function run(confirm) {
    if (busy) return
    setBusy(true)
    setError(null)
    try {
      const body = await request(confirm)
      if (!confirm && body.pending) {
        setLines(body.lines)
        setMode('plan')
        return
      }
      // A final answer (applied, refused or failed): never keep typed values.
      setValues({})
      setLines([])
      setMode('idle')
      setResult({ ok: body.status === 0, lines: body.lines })
      if (confirm) onChanged()
    } catch (e) {
      setError(e.message)
    } finally {
      setBusy(false)
    }
  }

  function start(next) {
    setAction(next)
    setResult(null)
    setError(null)
    setValues({})
    if (next === 'remove') setMode('plan-remove')
    else setMode('form')
  }

  function cancel() {
    setValues({})
    setLines([])
    setMode('idle')
  }

  const planning = mode === 'plan' || mode === 'plan-remove'
  return (
    <section aria-labelledby={`p-${id}`}>
      <div className="entry-head">
        <h3 id={`p-${id}`}>{provider}</h3>
        <span className={`label ${configured ? 'label-improve' : 'label-aware'}`}>
          {configured ? 'Configured: yes' : 'Configured: no'}
        </span>
      </div>
      {official && (
        <p>
          Official claude.ai connector available:{' '}
          <a href={link} target="_blank" rel="noopener noreferrer">
            add it in claude.ai
          </a>
          . Use the form below only if you want this launcher instead.
        </p>
      )}

      {mode === 'idle' && (
        <button type="button" onClick={() => start(configured ? 'remove' : 'add')}>
          {configured ? `Remove ${provider}` : `Set up ${provider}`}
        </button>
      )}

      {mode === 'form' && (
        <form
          onSubmit={(e) => {
            e.preventDefault()
            run(false)
          }}
        >
          {fields.map((f) => (
            <div className="field" key={f.env}>
              <label htmlFor={`f-${id}-${f.env}`}>{f.label}</label>
              <input
                id={`f-${id}-${f.env}`}
                name={f.env}
                type={f.secret ? 'password' : 'text'}
                autoComplete="off"
                spellCheck={false}
                value={values[f.env] ?? ''}
                onChange={(e) => setValues({ ...values, [f.env]: e.target.value })}
              />
            </div>
          ))}
          <div className="actions">
            <button type="submit" disabled={busy}>
              Review changes
            </button>
            <button type="button" onClick={cancel} disabled={busy}>
              Cancel
            </button>
          </div>
        </form>
      )}

      {mode === 'plan-remove' && (
        <div className="actions">
          <button type="button" onClick={() => run(false)} disabled={busy}>
            Review changes
          </button>
          <button type="button" onClick={cancel} disabled={busy}>
            Cancel
          </button>
        </div>
      )}

      {planning && lines.length > 0 && (
        <div role="group" aria-label={`Changes for ${provider}`}>
          <p>This will change:</p>
          <pre className="plan">{lines.join('\n')}</pre>
          <div className="actions">
            <button type="button" onClick={() => run(true)} disabled={busy}>
              {action === 'add' ? 'Apply changes' : 'Apply removal'}
            </button>
            <button type="button" onClick={cancel} disabled={busy}>
              Cancel
            </button>
          </div>
        </div>
      )}

      {error && (
        <p role="alert" className="error">
          Could not complete: {error}
        </p>
      )}
      {result && (
        <div role="status" className={result.ok ? undefined : 'error'}>
          <pre className="plan">{result.lines.join('\n')}</pre>
        </div>
      )}
    </section>
  )
}

function Catalog() {
  const state = useLoad(load)
  const [fresh, setFresh] = useState(null)
  // After a write, reload the list in place so each provider keeps its result message.
  const refresh = () => getJson('/api/connectors').then(setFresh, () => {})
  if (state.status === 'loading') return <p role="status">Loading connectors</p>
  if (state.status === 'error')
    return (
      <p role="alert" className="error">
        Could not load connectors: {state.message}
      </p>
    )
  return groupByCategory((fresh ?? state.data).connectors).map((g) => (
    <div key={g.category}>
      <h2>{g.category}</h2>
      {g.items.map((c) => (
        <Provider key={c.id} connector={c} onChanged={refresh} />
      ))}
    </div>
  ))
}

export default function ConnectorsView() {
  return (
    <main>
      <h1>Connectors</h1>
      <p className="muted">
        Secrets go to your OS keychain and are never shown again. Every change is shown before it is written.
      </p>
      <Catalog />
    </main>
  )
}
