/**
 * A card of the feed: an icon tile with the first character of `icon` (mint, or amber with `tone="amber"`),
 * title and one-line summary, `time` right-aligned, the body (`children`) and an optional footer row.
 * `id` names the heading, so the article is labelled by its title. Other props (data-*) go to the article.
 */
export default function FeedCard({ id, icon, title, summary, time, footer, tone, className = '', children, ...rest }) {
  const initial = [...String(icon)][0]?.toUpperCase() ?? ''
  return (
    <article className={`feed-card ${className}`.trim()} aria-labelledby={id} {...rest}>
      <div className="feed-head">
        <span className={`feed-icon${tone === 'amber' ? ' tone-amber' : ''}`} aria-hidden="true">
          {initial}
        </span>
        <div className="feed-title">
          <h3 id={id}>{title}</h3>
          {summary && <div className="feed-sub">{summary}</div>}
        </div>
        {time && <span className="feed-time">{time}</span>}
      </div>
      {children && <div className="feed-body">{children}</div>}
      {footer && <footer className="feed-foot">{footer}</footer>}
    </article>
  )
}
