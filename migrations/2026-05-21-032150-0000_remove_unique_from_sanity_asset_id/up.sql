-- Your SQL goes here
-- This tells Postgres to drop only the unique restriction, leaving data untouched
ALTER TABLE posts DROP CONSTRAINT posts_sanity_asset_id_key;
