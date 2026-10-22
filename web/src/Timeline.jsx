import { duration, fmt, localTime, totalTokens } from './format.js'

const MAX_SESSIONS = 50
const W = 600
const H = 56
const LANE_Y = 40
const LANE_H = 8
const TOP = 4

function SessionChart({ session, gapMinutes }) {
  const t0 = new Date(session.start_utc).getTime()
  const span = new Date(session.end_utc).getTime() - t0
  const x = (utc) => (span > 0 ? ((new Date(utc).getTime() - t0) / span) * (W - 8) + 4 : W / 2)
  const max = Math.max(...session.points.map(totalTokens), 1)
  const label =
    `${session.project} session ${session.session_id}: ${session.messages} messages, ` +
    `${session.gaps.length} ${session.gaps.length === 1 ? 'gap' : 'gaps'}, ` +
    `${localTime(session.start_utc)} to ${localTime(session.end_utc)}`
  return (
    <svg className="chart" viewBox={`0 0 ${W} ${H}`} role="img" aria-label={label}>
      <rect className="lane" x="4" y={LANE_Y} width={W - 8} height={LANE_H} />
      {session.gaps.map((g, i) => (
        <rect key={i} data-gap className="gap" x={x(g.start_utc)} y={LANE_Y} width={Math.max(x(g.end_utc) - x(g.start_utc), 1)} height={LANE_H}>
          <title>{`Pause of ${g.minutes} min (threshold ${gapMinutes} min)`}</title>
        </rect>
      ))}
      {session.points.map((p, i) => {
        const h = Math.max((totalTokens(p) / max) * (LANE_Y - TOP - 2), 1)
        return (
          <rect key={i} data-point className="point" x={x(p.ts_utc) - 1} y={LANE_Y - h} width="2" height={h}>
            <title>{`${localTime(p.ts_utc)}: ${fmt(totalTokens(p))} tokens (${p.model ?? 'unknown model'})`}</title>
          </rect>
        )
      })}
    </svg>
  )
}

export default function Timeline({ data }) {
  const shown = data.sessions.slice(0, MAX_SESSIONS)
  return (
    <section aria-labelledby="h-timeline">
      <h2 id="h-timeline">Session timeline</h2>
      <p className="muted">
        Each chart spans one session from start to end. Bars show tokens per message over time; shaded bands are
        gaps, a pause of {data.gap_minutes} min or more.
      </p>
      {shown.length === 0 ? (
        <p className="muted">No sessions in this period.</p>
      ) : (
        <ul className="timeline">
          {shown.map((s) => (
            <li key={s.session_id} data-session={s.session_id} data-testid={`session-${s.session_id}`}>
              <div className="session-head">
                <strong>{s.project}</strong>
                <span className="muted">{s.session_id}</span>
                <span>{`${localTime(s.start_utc)} to ${localTime(s.end_utc)}`}</span>
                <span>{duration(s.start_utc, s.end_utc)}</span>
              </div>
              <div className="muted">
                {s.models.join(', ') || 'unknown model'} · {fmt(s.messages)} messages · {fmt(totalTokens(s))} tokens
              </div>
              <SessionChart session={s} gapMinutes={data.gap_minutes} />
              {s.gaps.length > 0 && (
                <ul className="gaps">
                  {s.gaps.map((g, i) => (
                    <li key={i}>{`Gap of ${g.minutes} min, ${localTime(g.start_utc)} to ${localTime(g.end_utc)}`}</li>
                  ))}
                </ul>
              )}
            </li>
          ))}
        </ul>
      )}
      {data.sessions.length > MAX_SESSIONS && (
        <p className="muted">{`Showing ${MAX_SESSIONS} of ${data.sessions.length} sessions, newest first.`}</p>
      )}
    </section>
  )
}
