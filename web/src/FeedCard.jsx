const DOTS = 5

/** One of the five dot colors, the same for the same text. */
const dotOf = (text) => ([...text].reduce((sum, ch) => sum + ch.codePointAt(0), 0) % DOTS) + 1

/**
 * A card of the feed: round icon with the first character of `icon`, title and one-line summary,
 * `time` right-aligned after a thin divider, the body (`children`) and an optional footer row.
 * `id` names the heading, so the article is labelled by its title. Other props (data-*) go to the article.
 */
export default function FeedCard({ id, icon, title, summary, time, footer, className = '', children, ...rest }) {
  const initial = [...String(icon)][0]?.toUpperCase() ?? ''
  return (
    <article className={`feed-card ${className}`.trim()} aria-labelledby={id} {...rest}>
      <div className="feed-head">
        <span className={`feed-icon dot-${dotOf(String(icon))}`} aria-hidden="true">
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
