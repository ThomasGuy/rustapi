-- This file should undo anything in `up.sql`
DROP TRIGGER IF EXISTS update_comments_updated_at ON comments;
DROP TABLE IF EXISTS comments;
