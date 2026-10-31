// API-shaped fixtures. `sessions` follows the HUN-53 response contract exactly.
const cost = (usd, abstained = []) => ({
  usd,
  price_table_date: '2026-10-01',
  abstained_models: abstained,
})
const group = (key, i, o, cc, cr, messages, usd, abstained) => ({
  key,
  messages,
  input_tokens: i,
  output_tokens: o,
  cache_creation_input_tokens: cc,
  cache_read_input_tokens: cr,
  api_equivalent_cost: cost(usd, abstained),
})
const period = { since_utc: null, until_utc: null }

export const summary = {
  period,
  sessions: 3,
  messages: 20,
  input_tokens: 1200,
  output_tokens: 3400,
  cache_creation_input_tokens: 5600,
  cache_read_input_tokens: 78900,
  cache_hit_ratio: 0.9184,
  abstained: { total: 4, by_version: { '9.9.9': 4 } },
}

const total = group('', 1200, 3400, 5600, 78900, 20, 12.5, [])
const { key: _k, ...totalRow } = total
export const breakdowns = {
  day: {
    by: 'day',
    period,
    groups: [
      group('2026-10-19', 100, 200, 300, 400, 5, 1.25, []),
      group('2026-10-20', 1100, 3200, 5300, 78500, 15, 11.25, []),
    ],
    total: totalRow,
  },
  model: {
    by: 'model',
    period,
    groups: [
      group('claude-opus-5-5', 1000, 3000, 5000, 78000, 15, 11, []),
      group('mystery-model', 200, 400, 600, 900, 5, null, ['mystery-model']),
    ],
    total: totalRow,
  },
  project: {
    by: 'project',
    period,
    groups: [
      group('app', 600, 1700, 2800, 39000, 10, 6, []),
      group('app', 600, 1700, 2800, 39900, 10, 6.5, []),
    ],
    total: totalRow,
  },
  tool: {
    by: 'tool',
    period,
    groups: [group('Bash', 900, 2000, 4000, 60000, 12, 8, []), group('(none)', 300, 1400, 1600, 18900, 8, 4.5, [])],
    total: totalRow,
  },
}

// HUN-83: previous equal period (request carries `until`). Current day total: 89,100 tokens, 20 messages, $12.50.
const prevTotal = { ...totalRow, input_tokens: 600, output_tokens: 1700, cache_creation_input_tokens: 2800, cache_read_input_tokens: 39450, messages: 16, api_equivalent_cost: cost(20) }
export const previousDay = { by: 'day', period, groups: [], total: prevTotal }

export const outliers = {
  period,
  thresholds: { tool_calls: 100, context_tokens: 150000 },
  sessions: [
    { session_id: 'sess-outlier-1', project: 'app', tool_calls: 140, max_context_tokens: 180000, reasons: ['tool_calls', 'context_tokens'] },
  ],
}

const point = (ts, i, o, cc, cr) => ({
  ts_utc: ts,
  model: 'claude-opus-5-5',
  input_tokens: i,
  output_tokens: o,
  cache_creation_input_tokens: cc,
  cache_read_input_tokens: cr,
})
export const sessions = {
  period,
  gap_minutes: 30,
  sessions: [
    {
      session_id: 'abc12345-aaaa',
      project: 'demo-app',
      start_utc: '2026-10-20T08:00:00.000000Z',
      end_utc: '2026-10-20T10:00:00.000000Z',
      models: ['claude-haiku-5-5', 'claude-opus-5-5'],
      messages: 3,
      input_tokens: 30,
      output_tokens: 60,
      cache_creation_input_tokens: 90,
      cache_read_input_tokens: 120,
      points: [
        point('2026-10-20T08:00:00.000000Z', 10, 20, 30, 40),
        point('2026-10-20T08:10:00.000000Z', 10, 20, 30, 40),
        point('2026-10-20T10:00:00.000000Z', 10, 20, 30, 40),
      ],
      gaps: [{ start_utc: '2026-10-20T08:10:00.000000Z', end_utc: '2026-10-20T10:00:00.000000Z', minutes: 110.0 }],
    },
    {
      session_id: 'single-point',
      project: 'app',
      start_utc: '2026-10-19T08:00:00.000000Z',
      end_utc: '2026-10-19T08:00:00.000000Z',
      models: ['claude-opus-5-5'],
      messages: 1,
      input_tokens: 1,
      output_tokens: 2,
      cache_creation_input_tokens: 3,
      cache_read_input_tokens: 4,
      points: [point('2026-10-19T08:00:00.000000Z', 1, 2, 3, 4)],
      gaps: [],
    },
  ],
}

export const emptySummary = {
  ...summary,
  sessions: 0,
  messages: 0,
  input_tokens: 0,
  output_tokens: 0,
  cache_creation_input_tokens: 0,
  cache_read_input_tokens: 0,
  cache_hit_ratio: null,
  abstained: { total: 0, by_version: {} },
}
export const emptyBreakdown = (by) => ({
  by,
  period,
  groups: [],
  total: { ...totalRow, messages: 0, input_tokens: 0, output_tokens: 0, cache_creation_input_tokens: 0, cache_read_input_tokens: 0, api_equivalent_cost: cost(0) },
})
export const emptyOutliers = { ...outliers, sessions: [] }
export const emptySessions = { period, gap_minutes: 30, sessions: [] }

/** A fetch stub answering the five read routes; `overrides` replaces a route's body. */
// Shapes copied from core/rules.evaluate, core/plan.improvement_plan and core/kit.kit_summary.
export const warnings = {
  as_of_utc: '2026-10-22T04:00:00Z',
  hits: [
    {
      rule_id: 'cache_hit_low', label: 'warning', priority: 1, measured: 0.31, threshold: 0.5, window_days: 7,
      title: 'Low cache reuse',
      why: 'A prompt read from cache costs far less.',
      steps: ['Keep the start of your prompts the same.', 'Avoid editing CLAUDE.md mid-session.'],
      summary: '31% of prompt tokens came from cache; target 50% or more, last 7 days',
      fix: 'Keep the start of your prompts the same. Avoid editing CLAUDE.md mid-session.',
    },
    {
      rule_id: 'context_index_stale', label: 'improve', priority: 2, measured: 1, threshold: 0, window_days: 7,
      title: 'Code index is out of date',
      why: 'Search returns old results.',
      steps: ['Call index_path for the listed projects.'],
      summary: '1 project has a stale easy-context index; target none, last 7 days',
      fix: 'Call index_path for the listed projects.',
      detail: { projects: ['alpha'] },
    },
    {
      rule_id: 'top_project_share', label: 'aware', priority: 3, measured: 0.82, threshold: 0.8, window_days: 7,
      title: 'One project uses most of your tokens',
      why: 'Your limit runs out sooner.',
      steps: ['Confirm this project should take most of your effort.'],
      summary: 'One project (alpha) used 82% of tokens; target 80% or less, last 7 days',
      fix: 'Confirm this project should take most of your effort.',
      detail: { project: 'alpha' },
    },
  ],
  abstained: [{ rule_id: 'kit_error_rate', title: 'Many tool calls fail', reason: 'no kernel-trace tool calls in the window' }],
}
export const plan = {
  groups: [
    { priority: 1, items: [{ rule_id: 'cache_hit_low', label: 'warning', measured: 0.31, threshold: 0.5, fix: 'Keep the start of your prompts the same.', title: 'Low cache reuse' }] },
  ],
}
export const kit = {
  projects: [
    {
      project: 'beta',
      missing_on_disk: true,
      parts: { kernel: { status: 'no data' }, git: { status: 'abstained' }, context: { status: 'no data' } },
    },
    {
      project: 'alpha',
      missing_on_disk: false,
      parts: {
        kernel: { status: 'ok', tool_calls: 40, tool_errors: 0, error_rate: 0.0, kit_tasks: 2, first_utc: '2026-10-01T12:00:00Z', last_utc: '2026-10-20T12:00:00Z', abstained: 0 },
        git: { status: 'ok', commits: 7, commits_by_day: { '2026-10-20': 7 } },
        context: { status: 'ok', last_index_at: '2026-10-01T12:00:00Z', latest_commit_utc: '2026-10-20T12:00:00Z', stale: true },
      },
    },
    {
      project: 'gamma',
      missing_on_disk: false,
      parts: {
        kernel: { status: 'ok', tool_calls: 5, tool_errors: 2, error_rate: 0.4, kit_tasks: 1, first_utc: '2026-10-01T12:00:00Z', last_utc: '2026-10-02T12:00:00Z', abstained: 1 },
        git: { status: 'ok', commits: 0, commits_by_day: {} },
        context: { status: 'ok', last_index_at: '2026-10-21T12:00:00Z', latest_commit_utc: null, stale: null },
      },
    },
    {
      project: 'delta',
      missing_on_disk: false,
      parts: {
        kernel: { status: 'ok', tool_calls: 1200, tool_errors: 0, error_rate: 0.0, kit_tasks: 9, first_utc: '2026-09-01T12:00:00Z', last_utc: '2026-10-21T12:00:00Z', abstained: 0 },
        git: { status: 'ok', commits: 342, commits_by_day: { '2026-10-21': 3, '2026-10-15': 2, '2025-10-21': 100 } },
        context: { status: 'ok', last_index_at: '2026-10-19T12:00:00Z', latest_commit_utc: '2026-10-19T12:00:00Z', stale: false },
        verifier: { status: 'ok', scope: 'repo', generated_at: '2026-10-20T12:00:00Z', overall: { value: 82, abstained: false, reason_code: null } },
      },
    },
  ],
  ui: { status: 'ok', sessions: 6, passed: 4, failed: 1, other: 1, actions: 33, first_utc: '2026-10-01T00:00:00Z', last_utc: '2026-10-21T00:00:00Z' },
}

export const connectors = {
  workspace_connectors: [],
  connectors: [
    {
      id: 'slack',
      category: 'Messaging',
      provider: 'Slack',
      claude_ai_connector: true,
      claude_ai_link: 'https://claude.ai/settings/connectors',
      configured: false,
      fields: [{ env: 'SLACK_MCP_XOXB_TOKEN', label: 'Bot token (xoxb-...)', secret: true }],
    },
    {
      id: 'discord',
      category: 'Messaging',
      provider: 'Discord',
      claude_ai_connector: false,
      claude_ai_link: null,
      configured: true,
      fields: [
        { env: 'DISCORD_BOT_NAME', label: 'Bot name', secret: false },
        { env: 'DISCORD_TOKEN', label: 'Bot token', secret: true },
      ],
    },
  ],
}

// HUN-68: GET /api/accounts and /api/workspaces shapes (3 Claude, 1 Codex, 1 detected, 1 missing).
const acct = (id, provider, label, folder, status, workspaces = []) => ({
  id, provider, label, folder, status, config_dir_env: null, workspaces,
})
export const accountList = [
  acct(1, 'claude', 'Claude default', '/home/o/.claude', 'confirmed', ['work']),
  acct(2, 'claude', 'Claude personal', '/home/o/.claude-personal', 'confirmed', []),
  acct(3, 'claude', 'Claude gone', '/home/o/.claude-gone', 'missing'),
  acct(4, 'codex', 'Codex', '/home/o/.codex', 'confirmed'),
  acct(5, 'claude', '', '/home/o/.claude-new', 'detected'),
]
export const accountsByAccount = { by: 'account', accounts: accountList }
export const accountsByProvider = {
  by: 'provider',
  providers: {
    claude: accountList.filter((a) => a.provider === 'claude'),
    codex: accountList.filter((a) => a.provider === 'codex'),
  },
}
// HUN-116: /api/accounts `status_line` (shape from the status-line data task). "Now" in tests: 2026-10-29T10:00:00Z.
export const statusLine = (over = {}) => ({
  email: null, plan: null, organization: null, last_model: null, last_active_utc: null,
  tokens_today: 0, tokens_7d: 0, limits: null, ...over,
})
const win = (used_percentage, resets_at, stale = false) => ({ used_percentage, resets_at, stale })
const withStatus = (a, status_line) => ({ ...a, status_line })
const statusAccounts = [
  withStatus(acct(1, 'claude', 'Claude default', '/home/o/.claude', 'confirmed', ['work']), statusLine({
    email: 'a@b.c', plan: 'stripe_subscription', organization: 'Acme Org', last_model: 'claude-opus-5-5',
    last_active_utc: '2026-10-29T08:00:00.000000Z', tokens_today: 1234, tokens_7d: 56789,
    limits: { five_hour: win(42.5, '2026-10-29T11:20:00Z'), seven_day: win(75, '2026-11-01T14:00:00Z'), written_at: '2026-10-29T09:55:00Z', stale: false },
  })),
  withStatus(acct(2, 'claude', 'Claude personal', '/home/o/.claude-personal', 'confirmed'), statusLine({
    email: 'me@home.test', last_model: 'claude-sonnet-5-5', last_active_utc: '2026-10-29T09:59:30Z', tokens_today: 10, tokens_7d: 20,
    limits: { five_hour: win(92, '2026-10-29T10:30:00Z'), seven_day: win(10, '2026-11-02T10:00:00Z'), written_at: '2026-10-29T09:55:00Z', stale: false },
  })),
  withStatus(acct(3, 'claude', 'Claude bare', '/home/o/.claude-bare', 'confirmed'), statusLine()),
  withStatus(acct(4, 'codex', 'Codex', '/home/o/.codex', 'confirmed'), statusLine({ tokens_today: 5, tokens_7d: 6 })),
  withStatus(acct(6, 'claude', 'Claude old', '/home/o/.claude-old', 'confirmed'), statusLine({
    limits: { five_hour: win(88, '2026-10-29T09:00:00Z', true), seven_day: win(null, null), written_at: '2026-10-29T08:00:00Z', stale: true },
  })),
  withStatus(acct(7, 'claude', 'Claude skew', '/home/o/.claude-skew', 'confirmed'), statusLine({
    limits: { five_hour: win(30, '2026-10-29T09:59:00Z'), seven_day: null, written_at: '2026-10-29T09:50:00Z', stale: false },
  })),
]
export const accountsStatus = { by: 'account', accounts: statusAccounts }
export const accountsStatusByProvider = {
  by: 'provider',
  providers: {
    claude: statusAccounts.filter((a) => a.provider === 'claude'),
    codex: statusAccounts.filter((a) => a.provider === 'codex'),
  },
}
export const workspaceList = {
  workspaces: [
    { id: 1, name: 'orphan', is_orphan: true },
    { id: 2, name: 'work', is_orphan: false },
    { id: 3, name: 'personal', is_orphan: false },
  ],
}
// HUN-69: Workspaces page shapes (GET /api/workspaces with repos, GET /api/connectors with workspace_connectors).
const ws = (id, name, extra = {}) => ({ id, name, color: null, icon: null, description: null, email: null, is_orphan: false, repos: [], ...extra })
export const workspacesFull = {
  workspaces: [
    ws(1, 'orphan', { is_orphan: true }),
    ws(2, 'company', { color: '#0b5fd6', icon: 'C', email: 'me@company.test', description: 'Day job', repos: ['/work/company', '/work/company-web'] }),
    ws(3, 'side'),
  ],
}
export const connectorsFull = {
  ...connectors,
  workspace_connectors: [
    { id: 'c1a2b3c4', alias: 'company-slack', provider: 'slack', workspace: 'company' },
    { id: 'discord', alias: 'discord', provider: 'discord', workspace: 'orphan' },
    { id: 'slack', alias: 'slack', provider: 'slack', workspace: 'orphan' },
  ],
}
export const accountsFull = {
  by: 'account',
  accounts: [
    acct(1, 'claude', 'Claude company', '/home/o/.claude', 'confirmed', ['company']),
    acct(2, 'claude', 'Claude personal', '/home/o/.claude-personal', 'confirmed', []),
    acct(4, 'codex', 'Codex', '/home/o/.codex', 'confirmed'),
  ],
}
export const legacyOne = { legacy: [{ id: 'slack', keychain: true, claude: true, foreign: false }] }
/** Overrides that give the Workspaces page its full dataset. */
export const workspaceOverrides = (extra = {}) => ({
  workspaces: workspacesFull,
  connectors: connectorsFull,
  accounts: accountsFull,
  ...extra,
})

// HUN-70: Usage filter options (UsageView tests pass these as overrides).
export const usageWorkspaces = {
  workspaces: [
    { id: 'orphan', name: 'orphan', is_orphan: true, repos: [] },
    { id: 'w1', name: 'Acme', is_orphan: false, repos: ['/work/acme'] },
    { id: 'w2', name: 'Empty', is_orphan: false, repos: [] },
  ],
}
export const usageAccounts = {
  accounts: [
    { id: 1, provider: 'claude', label: 'Claude company', status: 'confirmed' },
    { id: 2, provider: 'claude', label: 'Claude detected', status: 'detected' },
    { id: 3, provider: 'codex', label: 'Codex', status: 'confirmed' },
  ],
}

const firstRunStep = (id, done, hint) => ({ id, done, hint })
/** GET /api/first-run on a fresh home: all four steps not done. */
export const firstRun = {
  steps: [
    firstRunStep('first_ingest', false, 'easy-personal ingest'),
    firstRunStep('ingest_timer', false, 'easy-personal schedule install'),
    firstRunStep('accounts_confirmed', false, 'Open Agents and confirm an account'),
    firstRunStep('first_workspace', false, 'Open Workspaces and create one'),
  ],
}

export function fakeFetch(overrides = {}) {
  const calls = []
  const fn = async (url, init = {}) => {
    calls.push({ url: String(url), init })
    const u = new URL(String(url), 'http://127.0.0.1')
    const path = u.pathname
    let body
    if (path === '/api/usage/summary') body = overrides.summary ?? summary
    else if (path === '/api/usage/breakdown' && u.searchParams.has('until'))
      body = overrides.previousDay ?? previousDay
    else if (path === '/api/usage/breakdown') body = (overrides.breakdowns ?? breakdowns)[u.searchParams.get('by')]
    else if (path === '/api/usage/outliers') body = overrides.outliers ?? outliers
    else if (path === '/api/usage/sessions') body = overrides.sessions ?? sessions
    else if (path === '/api/warnings') body = overrides.warnings ?? warnings
    else if (path === '/api/plan') body = overrides.plan ?? plan
    else if (path === '/api/kit') body = overrides.kit ?? kit
    else if (path === '/api/first-run') body = overrides.firstRun ?? firstRun
    else if (path === '/api/accounts' && !init.method)
      body = (u.searchParams.get('by') === 'provider' ? overrides.accountsByProvider : overrides.accounts) ?? (u.searchParams.get('by') === 'provider' ? accountsByProvider : accountsByAccount)
    else if (path === '/api/workspaces' && !init.method) body = overrides.workspaces ?? workspaceList
    else if (path === '/api/connectors/legacy' && !init.method) body = overrides.legacy ?? { legacy: [] }
    else if (path === '/api/connectors' && !init.method) body = overrides.connectors ?? connectors
    if (overrides.status) return { ok: false, status: overrides.status, json: async () => ({ error: 'forbidden' }) }
    if (body === undefined) return { ok: false, status: 404, json: async () => ({}) }
    return { ok: true, status: 200, json: async () => body }
  }
  fn.calls = calls
  return fn
}
