// The funny lines under a loading label (HUN-229, FR-065). Short, friendly, on-theme; one is picked per indicator.
export const LOADING_LINES = [
  'Counting tokens one by one…',
  'Asking the cache nicely…',
  'Herding stray sessions…',
  'Dusting off old transcripts…',
  'Bribing the database with coffee…',
  'Sorting receipts from your robots…',
  'Teaching workspaces to share…',
  'Polishing the token meter…',
  'Untangling the session timeline…',
  'Warming up the cache…',
  'Waking the sleepy scheduler…',
  'Stacking tokens into tidy piles…',
]

/** One line from the list; `random` is the source of a number in [0, 1) (a test can pass its own). */
export const pickLoadingLine = (random = Math.random) => LOADING_LINES[Math.floor(random() * LOADING_LINES.length)]
