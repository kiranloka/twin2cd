ALTER TABLE api_tokens ADD COLUMN runner_id TEXT REFERENCES runners(id);
CREATE INDEX IF NOT EXISTS idx_api_tokens_runner ON api_tokens (runner_id);
