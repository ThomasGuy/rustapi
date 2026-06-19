-- This file should undo anything in `up.sql`
-- Revert: Add back the string column and extract the nested asset reference string
ALTER TABLE posts ADD COLUMN sanity_asset_id VARCHAR(255);

UPDATE posts
SET sanity_asset_id = sanity_image->'asset'->>'reference'
WHERE sanity_image IS NOT NULL;

ALTER TABLE posts DROP COLUMN sanity_image;
