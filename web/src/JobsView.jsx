import { getJson } from './api.js'
import { ago, localTime } from './format.js'
import { useLoad } from './useLoad.js'

const NO_DATA = {
  systemd: 'systemd timers: no data (no systemctl or no user session)',
  cron: 'crontab: no data (no crontab for this user)',
}
const COLUMNS = ['Job', 'Kind', 'Schedule', 'Next run', 'Last run', 'Last exit']

const nextRun = (job) => (job.next_run ? localTime(job.next_run) : '—')
// A crontab keeps no run history: its last run is not recorded, which is not the same as never.
const lastRun = (job) => (job.kind === 'cron' ? 'not recorded' : ago(job.last_run))

function Exit({ code }) {
  if (code === null) return '—'
  return code === 0 ? '✓ 0' : <span className="error">{`✗ ${code}`}</span>
}

function Table({ jobs }) {
  return (
    <div className="table-wrap table-frame">
      <table>
        <thead>
          <tr>
            {COLUMNS.map((title) => (
              <th scope="col" key={title}>
                {title}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {jobs.map((job) => (
            <tr key={`${job.kind}:${job.name}`}>
              <th scope="row">{job.name}</th>
              <td>{job.kind}</td>
              <td>{job.schedule ?? '—'}</td>
              <td>{nextRun(job)}</td>
              <td>{lastRun(job)}</td>
              <td>
                <Exit code={job.last_exit} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function Footer({ hidden }) {
  const count = hidden === 0 ? 'No other jobs hidden' : `${hidden} other job${hidden === 1 ? '' : 's'} hidden`
  return <p className="muted">{`${count} · /loop jobs and cloud routines are not included`}</p>
}

/** `// work` › Scheduled: agent timers and cron lines, read live. Only each command's first word is ever shown. */
export default function JobsView() {
  const state = useLoad(() => getJson('/api/jobs'))
  return (
    <main>
      <h1>Scheduled jobs</h1>
      {state.status === 'loading' && <p role="status">Loading scheduled jobs</p>}
      {state.status === 'error' && (
        <p role="alert" className="error">
          Could not load scheduled jobs: {state.message}
        </p>
      )}
      {state.status === 'ready' && (
        <section aria-labelledby="h-jobs">
          <h2 id="h-jobs">Agent jobs</h2>
          {state.data.jobs.length === 0 ? <p className="muted">No agent jobs found.</p> : <Table jobs={state.data.jobs} />}
          {Object.entries(NO_DATA)
            .filter(([kind]) => state.data.sources[kind] === 'no data')
            .map(([kind, text]) => (
              <p key={kind} className="muted">
                {text}
              </p>
            ))}
          <Footer hidden={state.data.hidden} />
        </section>
      )}
    </main>
  )
}
