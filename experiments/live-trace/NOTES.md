# Live trace spike: notes

Spike for HUN-165..170. Plan: the owner's doc "Claude Live Trace — Experimental Plan"
(https://claude.ai/artifact/EVYWZrCSD34Cn35cAaVusB). Everything in the plan below Phase 0 is a
hypothesis to measure, not a requirement. Metadata only (PRD FR-002 / NFR-002).

## Phase 0: documentation check (source table)

Source: owner's plan doc. Checked: 2026-10-31.

| Item | URL | Status | What it says |
| -- | -- | -- | -- |
| OTel monitoring | https://code.claude.com/docs/en/monitoring-usage.md | valid | `CLAUDE_CODE_ENABLE_TELEMETRY=1`; exporters otlp/console/prometheus; logs every 5 s, metrics every 60 s by default; events carry `session.id`, `prompt.id`, `tool_use_id`; `api_request` carries `cost_usd`; content redacted unless `OTEL_LOG_*`; traces beta. |
| Hooks | https://code.claude.com/docs/en/hooks.md | valid | 33 events; stdin `session_id`, `transcript_path`, `cwd`, `hook_event_name`, tool events add `tool_use_id`; sync by default, `async: true`; native `http` hook type, connection errors non-blocking. |
| Transcripts | https://code.claude.com/docs/en/sessions.md , https://code.claude.com/docs/en/claude-directory.md | location valid, format internal (changes between versions) | Subagents under `<session>/subagents/`; `cleanupPeriodDays` default 30. |
| stream-json / SDK | https://code.claude.com/docs/en/headless.md , https://code.claude.com/docs/en/agent-sdk/streaming-output.md | partly confirmed | Needs `--verbose`; types `system/init`, `assistant`, `user`, `stream_event`, `result`; json output has `total_cost_usd`; stream-json `result` field names unconfirmed. |
| Admin usage API | https://platform.claude.com/docs/en/manage-claude/usage-cost-analytics-api.md | valid (beta) | `/v1/organizations/analytics/usage` and `/cost`; Admin key; refresh within 4 h, up to 24 h. |
| Claude Code analytics API | https://platform.claude.com/docs/en/manage-claude/claude-code-analytics-api.md | valid | `/v1/organizations/usage_report/claude_code`; daily per user; ~1 h lag; no subscription usage. |
| Pro/Max usage | https://code.claude.com/docs/en/costs.md | no API found | `/usage` shows local cost at list price and plan bars. |
