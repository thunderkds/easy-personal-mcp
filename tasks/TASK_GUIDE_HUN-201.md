Frontend-Implementer | C2 | Risk: Low | P1 | Refs: FR-067, PROJECT_SPEC_SITE.md §1 §2 §7

Owner, 2026-11-03: "the easy kit just update the link to [https://easy-kits.vercel.app/](<https://easy-kits.vercel.app/>), let update from our app. by the way, update the index page more visualize, will be easy to catch with the flow, diagram also."

Part A: replace every [https://personal-agentic-claude.vercel.app/](<https://personal-agentic-claude.vercel.app/>) with [https://easy-kits.vercel.app/](<https://easy-kits.vercel.app/>) (web/src/App.jsx + test, site/index.html header + footer, scripts/check_static.py allowlist, tests/test_site_content.py, tests/test_web_toolchain.py). README GitHub repo links stay.

Part B: site/index.html — inline SVG system diagram in #flow (sources → ingest → local store → MCP / CLI / dashboard; connect lane catalog → keychain → claude mcp add-json → launcher; "your machine" boundary), existing step lists restyled as numbered connected steps, a 4-step quick-start strip under the hero with verbatim README commands. No JS, no external hosts, dark + light, no horizontal scroll at 375 px.

Full guide: tasks/TASK_GUIDE\_<ID>.md in the repo.