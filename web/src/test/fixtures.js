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
    { rule_id: 'cache_hit_low', label: 'warning', priority: 1, measured: 0.31, threshold: 0.5, window_days: 7, fix: 'Keep prompts stable so the cache is reused.' },
    { rule_id: 'context_index_stale', label: 'improve', priority: 2, measured: 1, threshold: 0, window_days: 7, fix: 'Re-run the easy-context index.', detail: { projects: ['alpha'] } },
    { rule_id: 'top_project_share', label: 'aware', priority: 3, measured: 0.82, threshold: 0.8, window_days: 30, fix: 'One project uses most tokens.', detail: { project: 'alpha' } },
  ],
  abstained: [{ rule_id: 'kit_error_rate', reason: 'no kernel-trace tool calls in the window' }],
}
export const plan = {
  groups: [
    { priority: 1, items: [{ rule_id: 'cache_hit_low', label: 'warning', measured: 0.31, threshold: 0.5, fix: 'Keep prompts stable so the cache is reused.' }] },
    {
      priority: 3,
      items: [{ rule_id: 'top_project_share', label: 'aware', measured: 0.82, threshold: 0.8, fix: 'One project uses most tokens.', detail: { project: 'alpha' } }],
    },
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
        kernel: { status: 'ok', tool_calls: 40, tool_errors: 0, error_rate: 0.0, kit_tasks: 2, first_utc: '2026-10-01T00:00:00Z', last_utc: '2026-10-20T00:00:00Z', abstained: 0 },
        git: { status: 'ok', commits: 7, commits_by_day: { '2026-10-20': 7 } },
        context: { status: 'ok', last_index_at: '2026-10-01T00:00:00Z', latest_commit_utc: '2026-10-20T00:00:00Z', stale: true },
      },
    },
    {
      project: 'gamma',
      missing_on_disk: false,
      parts: {
        kernel: { status: 'ok', tool_calls: 5, tool_errors: 2, error_rate: 0.4, kit_tasks: 1, first_utc: '2026-10-01T00:00:00Z', last_utc: '2026-10-02T00:00:00Z', abstained: 1 },
        git: { status: 'ok', commits: 0, commits_by_day: {} },
        context: { status: 'ok', last_index_at: '2026-10-21T00:00:00Z', latest_commit_utc: null, stale: null },
      },
    },
  ],
  ui: { status: 'ok', sessions: 6, passed: 4, failed: 1, other: 1, actions: 33, first_utc: '2026-10-01T00:00:00Z', last_utc: '2026-10-21T00:00:00Z' },
}

export function fakeFetch(overrides = {}) {
  const calls = []
  const fn = async (url, init = {}) => {
    calls.push({ url: String(url), init })
    const u = new URL(String(url), 'http://127.0.0.1')
    const path = u.pathname
    let body
    if (path === '/api/usage/summary') body = overrides.summary ?? summary
    else if (path === '/api/usage/breakdown') body = (overrides.breakdowns ?? breakdowns)[u.searchParams.get('by')]
    else if (path === '/api/usage/outliers') body = overrides.outliers ?? outliers
    else if (path === '/api/usage/sessions') body = overrides.sessions ?? sessions
    else if (path === '/api/warnings') body = overrides.warnings ?? warnings
    else if (path === '/api/plan') body = overrides.plan ?? plan
    else if (path === '/api/kit') body = overrides.kit ?? kit
    if (overrides.status) return { ok: false, status: overrides.status, json: async () => ({ error: 'forbidden' }) }
    if (body === undefined) return { ok: false, status: 404, json: async () => ({}) }
    return { ok: true, status: 200, json: async () => body }
  }
  fn.calls = calls
  return fn
}
