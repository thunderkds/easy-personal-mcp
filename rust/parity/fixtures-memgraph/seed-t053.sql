DELETE FROM events;
DELETE FROM agent_accounts;
INSERT INTO agent_accounts (id, provider, folder, label, status) VALUES (1, 'claude', '@T@/t053/acct', 'W', 'confirmed');
INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, project_path) VALUES
('m', 'claude_code', '/x/1.jsonl', 1, 0, 0, '@T@/t053/projh'),
('m', 'claude_code', '/x/2.jsonl', 2, 0, 0, '@T@/t053/projdup'),
('m', 'claude_code', '/x/3.jsonl', 3, 0, 0, '@T@/t053/projem'),
('m', 'claude_code', '/x/4.jsonl', 4, 0, 0, '@T@/t053/projwt'),
('m', 'claude_code', '/x/5.jsonl', 5, 0, 0, '@T@/t053/projdd');
