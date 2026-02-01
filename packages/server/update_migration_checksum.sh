#!/bin/bash
set -e

# Update migration 88 checksum in the database
# This is needed because we modified the migration file after it was applied

# Calculate the new checksum
NEW_CHECKSUM=$(sha256sum migrations/000088_rename_listing_type_to_post_type.sql | awk '{print $1}')

echo "New checksum for migration 88: $NEW_CHECKSUM"

# Connect to database and update the checksum
cat << EOF | psql "$DATABASE_URL"
UPDATE _sqlx_migrations
SET checksum = decode('$NEW_CHECKSUM', 'hex')
WHERE version = 88;

SELECT version, description, encode(checksum, 'hex') as checksum
FROM _sqlx_migrations
WHERE version = 88;
EOF

echo "Migration checksum updated successfully!"
