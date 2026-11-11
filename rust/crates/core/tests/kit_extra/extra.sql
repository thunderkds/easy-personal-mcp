-- T042: rows added to a copy of rust/parity/store-rich (after its `/work/rich/kit` paths became the temp work folder).
-- `@T@` is the temp work folder. Read by `kit_parity.rs` and by `make_expected.py`, so both build the same store.
INSERT INTO workspace_repos (workspace_id, repo_path) VALUES
  (2, '@T@/repo-a'), (2, '@T@/repo-b'), (2, '@T@/plain'), (2, '@T@/missing'), (3, '@T@/repo-c');

INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id, project_path, tool_names, agent_account_id) VALUES
  ('m', 'claude_code', 'x1', 901, 0, 0, '2026-01-15T10:00:00.000000Z', 's-a1', '@T@/repo-a', 'Read,mcp__easy-ui__check,mcp__easy-ui__snap', 1),
  ('m', 'claude_code', 'x1', 901, 1, 0, '2026-01-16T10:00:00.000000Z', NULL, '@T@/repo-a', 'mcp__easy-ui__check', 1),
  ('m', 'claude_code', 'x1', 901, 2, 0, '2026-01-17T10:00:00.000000Z', 's-b1', '@T@/repo-b', 'Bash', 2),
  ('m', 'claude_code', 'x1', 901, 3, 0, '2026-01-10T10:00:00.000000Z', 's-c1', '@T@/repo-c', 'mcp__easy-ui__x', 1),
  ('m', 'claude_code', 'x1', 901, 4, 0, '2026-01-11T10:00:00.000000Z', 's-p1', '@T@/plain', 'Read', 1),
  ('m', 'claude_code', 'x1', 901, 5, 0, '2026-01-12T10:00:00.000000Z', 's-m1', '@T@/missing', 'Read', 1);

INSERT INTO kit_trace_events (machine_id, source_path, inode, byte_offset, project_path, kit_task_id, abstained, ts_utc, tool_name, is_error) VALUES
  ('m', 'k1', 902, 0, '@T@/repo-a', 'T001', 0, '2026-01-14T10:00:00.000000Z', 'Read', 1),
  ('m', 'k1', 902, 1, '@T@/repo-a', 'T001', 0, '2026-01-14T11:00:00.000000Z', 'Bash', 0),
  ('m', 'k1', 902, 2, '@T@/repo-a', 'T002', 0, '2026-01-15T11:00:00.000000Z', 'Bash', NULL),
  ('m', 'k1', 902, 3, '@T@/repo-a', NULL, 1, NULL, NULL, NULL),
  ('m', 'k2', 903, 0, '@T@/repo-b', NULL, 1, NULL, NULL, NULL),
  ('m', 'k2', 903, 1, '@T@/repo-b', NULL, 1, NULL, NULL, NULL);

INSERT INTO verifier_scores (project_path, status, scope, generated_at_utc, overall_value, overall_abstained, reason_code) VALUES
  ('@T@/kit/alpha', 'ok', 'all', '2026-01-09T00:00:00Z', 61.3, 0, NULL),
  ('@T@/repo-a', 'ok', 'changes', '2026-01-10T00:00:00Z', 80.0, 0, NULL),
  ('@T@/repo-b', 'ok', 'all', '2026-01-12T00:00:00Z', 72.5, 0, NULL),
  ('@T@/plain', 'unknown', NULL, NULL, NULL, 0, NULL),
  ('@T@/repo-c', 'ok', 'all', '2026-01-11T00:00:00Z', NULL, 1, 'too_little'),
  ('@T@/missing', 'ok', 'all', '2026-01-11T00:00:00Z', 90.0, 0, NULL);

INSERT INTO ui_sessions (id, status, started_at_utc, ended_at_utc, action_count) VALUES
  ('u1', 'passed', '2026-01-05T10:00:00.000000Z', '2026-01-05T10:05:00.000000Z', 12),
  ('u2', 'failed', '2026-01-06T10:00:00.000000Z', NULL, 3),
  ('u3', 'running', '2026-01-07T10:00:00.000000Z', NULL, 0);

INSERT INTO context_index (project_path, last_index_at_utc) VALUES
  ('@T@/repo-a', '2026-01-01T00:00:00+00:00'),
  ('@T@/repo-b', '2026-01-04T00:00:00+00:00'),
  ('@T@/plain', NULL),
  ('@T@/repo-c', '2099-01-01T00:00:00+00:00');
