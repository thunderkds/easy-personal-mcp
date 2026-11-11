DELETE FROM events;
DELETE FROM agent_accounts;
INSERT INTO agent_accounts (id, provider, folder, label, status) VALUES (1, 'claude', '@T@/acct', 'W', 'confirmed');
INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, project_path) VALUES
('m', 'claude_code', '/x/1.jsonl', 1, 0, 0, '@T@/proj'),
('m', 'claude_code', '/x/2.jsonl', 2, 0, 0, '@T@/proj600'),
('m', 'claude_code', '/x/3.jsonl', 3, 0, 0, '@T@/projcap'),
('m', 'claude_code', '/x/4.jsonl', 4, 0, 0, '@T@/gone-t043'),
('m', 'claude_code', '/x/5.jsonl', 5, 0, 0, '@T@/.claude/worktrees/wt');
