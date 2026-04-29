-- Your SQL goes here
CREATE TABLE IF NOT EXISTS comments (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  username VARCHAR(255) NOT NULL,
  comment TEXT NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
-- Index for faster lookups by user
CREATE INDEX idx_comments_post_id ON comments (post_id);
-- Reuse the update trigger function
CREATE TRIGGER update_comments_updated_at BEFORE
UPDATE ON comments FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
