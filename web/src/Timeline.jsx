import FeedCard from './FeedCard.jsx'
import { duration, fmt, localTime, totalTokens } from './format.js'
import { Progress, useTopN } from './Widgets.jsx'

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

const plural = (n, word) => `${fmt(n)} ${word}${n === 1 ? '' : 's'}`

function Session({ s, data, share, charted }) {
  const id = `session-title-${s.session_id}`
  const models = s.models.length === 0 ? 'unknown model' : s.models.length === 1 ? s.models[0] : plural(s.models.length, 'model')
  const counts = [
    plural(s.messages, 'message'),
    typeof s.tool_calls === 'number' && plural(s.tool_calls, 'tool call'),
    `${fmt(totalTokens(s))} tokens`,
    charted && plural(s.gaps.length, 'gap'),
  ].filter(Boolean)
  return (
    <li data-session={s.session_id} data-testid={`session-${s.session_id}`}>
      <FeedCard
        id={id}
        icon={s.project}
        title={s.project}
        summary={`${models} · ${plural(s.messages, 'message')}`}
        time={localTime(s.start_utc)}
        footer={counts.map((c) => (
          <span key={c}>{c}</span>
        ))}
      >
        <p className="muted session-line">
          {s.models.join(', ') || 'unknown model'} · {s.session_id} · {`${localTime(s.start_utc)} to ${localTime(s.end_utc)}`} · {duration(s.start_utc, s.end_utc)}
        </p>
        {charted ? (
          <SessionChart session={s} gapMinutes={data.gap_minutes} />
        ) : (
          <p className="muted">{`Chart kept for the newest ${data.detail_sessions} sessions only.`}</p>
        )}
        <Progress ratio={share} label={`Share of the period's tokens: ${(share * 100).toFixed(1)}%`} />
        {charted && s.gaps.length > 0 && (
          <ul className="gaps">
            {s.gaps.map((g, i) => (
              <li key={i}>{`Gap of ${g.minutes} min, ${localTime(g.start_utc)} to ${localTime(g.end_utc)}`}</li>
            ))}
          </ul>
        )}
      </FeedCard>
    </li>
  )
}

export default function Timeline({ data }) {
  const { shown, toggle } = useTopN(data.sessions)
  const periodTokens = data.sessions.reduce((sum, s) => sum + totalTokens(s), 0)
  return (
    <section aria-labelledby="h-timeline">
      <h2 id="h-timeline">{`Session timeline (${data.sessions.length})`}</h2>
      <p className="muted">
        Each chart spans one session from start to end. Bars show tokens per message over time; shaded bands are
        gaps, a pause of {data.gap_minutes} min or more. The bar under a chart is the session's share of the
        period's tokens.
      </p>
      {shown.length === 0 ? (
        <p className="muted">No sessions in this period.</p>
      ) : (
        <ul className="timeline feed">
          {shown.map((s, i) => (
            <Session
              key={s.session_id}
              s={s}
              data={data}
              share={periodTokens > 0 ? totalTokens(s) / periodTokens : 0}
              charted={typeof data.detail_sessions !== 'number' || i < data.detail_sessions}
            />
          ))}
        </ul>
      )}
      {toggle}
    </section>
  )
}
