-- Skills now carry a long-form README in addition to the structured
-- manifest. Stored as raw markdown; rendered client-side.

ALTER TABLE skills ADD COLUMN readme TEXT;
ALTER TABLE skills ADD COLUMN manifest JSONB NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE skills ADD COLUMN install_command TEXT;
ALTER TABLE skills ADD COLUMN repository_url TEXT;
ALTER TABLE skills ADD COLUMN homepage_url TEXT;
ALTER TABLE skills ADD COLUMN tags TEXT[] NOT NULL DEFAULT '{}';
ALTER TABLE skills ADD COLUMN install_count BIGINT NOT NULL DEFAULT 0;
