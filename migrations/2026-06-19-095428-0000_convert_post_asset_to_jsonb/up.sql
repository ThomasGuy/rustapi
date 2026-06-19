-- Your SQL goes here
-- 1. Temporarily add the new jsonb column
ALTER TABLE posts ADD COLUMN sanity_image JSONB;

-- 2. Migrate existing data: turn the raw string ID into the nested Sanity JSON layout
UPDATE posts
SET sanity_image = jsonb_build_object(
    'asset', jsonb_build_object(
        'reference', sanity_asset_id,
        'asset_type', 'reference'
    ),
    'hotspot', NULL,
    'crop', NULL
)
WHERE sanity_asset_id IS NOT NULL;

-- 3. Make it non-nullable if every post MUST have an image, or leave it as is
ALTER TABLE posts ALTER COLUMN sanity_image SET NOT NULL;

-- 4. Drop the old string column safely
ALTER TABLE posts DROP COLUMN sanity_asset_id;
