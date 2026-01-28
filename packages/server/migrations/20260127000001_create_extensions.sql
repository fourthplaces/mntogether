-- Enable required PostgreSQL extensions

-- UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Vector similarity search (for AI embeddings)
CREATE EXTENSION IF NOT EXISTS vector;
