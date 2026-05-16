-- Your SQL goes here
CREATE TABLE IF NOT EXISTS posts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  caption TEXT NULL,
  username VARCHAR(255) NOT NULL,
  sanity_asset_id TEXT NOT NULL UNIQUE,
  view_count INTEGER NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
-- Index for faster lookups by user
CREATE INDEX idx_posts_user_id ON posts (user_id);
-- Reuse the update trigger function
CREATE TRIGGER update_posts_updated_at BEFORE
UPDATE ON posts FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
