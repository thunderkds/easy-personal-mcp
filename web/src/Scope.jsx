/** The global Workspace/Account scope: shared helpers for the views that filter by it. */
export const NO_SCOPE = { workspace: '', account: '' }

/** `workspace=...&account=...` for the set parts of `scope`; '' when nothing is chosen. */
export function scopeQuery({ workspace, account }) {
  const parts = []
  if (workspace) parts.push(`workspace=${encodeURIComponent(workspace)}`)
  if (account) parts.push(`account=${encodeURIComponent(account)}`)
  return parts.join('&')
}

/** `path` with the scope query appended. */
export const scopedPath = (path, scope) => [path, scopeQuery(scope)].filter(Boolean).join('?')

/** The chosen names: the workspace name, the account label (id until the options are known), All when empty. */
export function scopeNames({ workspace, account }, options) {
  const label = options?.accounts.find((a) => String(a.id) === account)?.label ?? account
  return [workspace || 'All', account ? label : 'All']
}

export function ScopeLine({ scope, options }) {
  return (
    <p className="muted" data-testid="scope-line">
      Showing: {scopeNames(scope, options).join(' · ')}
    </p>
  )
}

/** The sidebar block: two labelled selects. */
export function ScopeSelects({ options, scope, onChange }) {
  return (
    <section className="scope" aria-label="Scope">
      <p className="scope-title" aria-hidden="true">Scope</p>
      <label>
        Workspace
        <select value={scope.workspace} onChange={(e) => onChange({ ...scope, workspace: e.target.value })}>
          <option value="">All</option>
          {options.workspaces.map((w) => (
            <option key={w.id} value={w.name}>
              {w.name}
            </option>
          ))}
        </select>
      </label>
      <label>
        Account
        <select value={scope.account} onChange={(e) => onChange({ ...scope, account: e.target.value })}>
          <option value="">All</option>
          {options.accounts.map((a) => (
            <option key={a.id} value={String(a.id)}>
              {a.label}
            </option>
          ))}
        </select>
      </label>
    </section>
  )
}
