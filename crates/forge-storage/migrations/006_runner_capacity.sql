ALTER TABLE runners ADD COLUMN capacity_json TEXT NOT NULL DEFAULT '{"cpu":1,"memory_mb":1024,"max_concurrent_jobs":1}';
