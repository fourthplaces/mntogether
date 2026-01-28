# Seed Data

## Running the Seed Script

Import organizations from the JSON file:

```bash
# From project root
cd packages/server
cargo run --bin seed_organizations
```

The script will:
1. Read `data/immigrant_resources_seed.json`
2. Use OpenAI GPT-4o-mini to extract tags from each organization's description
3. Create organizations in the database
4. Create and associate tags (services, languages, communities)
5. Skip organizations that already exist

## Requirements

- `OPENAI_API_KEY` environment variable must be set
- Database must be running and migrations applied
- `data/immigrant_resources_seed.json` must exist

## What Gets Created

For each organization:
- **Organization record** with name, description, contact info, location
- **Service tags**: food_assistance, housing_assistance, legal_services, etc.
- **Language tags**: english, spanish, somali, hmong, etc.
- **Community tags**: general, latino, somali, hmong, vietnamese, etc.

Tags are extracted automatically using AI from the `populations_served` field.

## Output

```
✓ Connected to database
✓ Loaded 50 organizations from JSON

🚀 Starting seed process...

[1/50] Processing: 360 Communities – Burnsville Resource Center & Food Shelf
  → Services: ["food_assistance", "emergency_financial_aid"]
  → Languages: ["english"]
  → Communities: ["general"]
  ✓ Created organization with 3 tags

...

✨ Seed complete!
   Created: 50
   Skipped: 0
   Total: 50
```
