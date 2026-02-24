--
-- PostgreSQL database dump
--

\restrict ohRKeLUWI5wSJbDPr5vMBj7TP5w2WAiMJa0IV9rtzpJkxneT52B2Fug7fP88AHa

-- Dumped from database version 16.11 (Debian 16.11-1.pgdg12+1)
-- Dumped by pg_dump version 16.11 (Debian 16.11-1.pgdg12+1)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: uuid-ossp; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS "uuid-ossp" WITH SCHEMA public;


--
-- Name: EXTENSION "uuid-ossp"; Type: COMMENT; Schema: -; Owner: -
--

COMMENT ON EXTENSION "uuid-ossp" IS 'generate universally unique identifiers (UUIDs)';


--
-- Name: vector; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS vector WITH SCHEMA public;


--
-- Name: EXTENSION vector; Type: COMMENT; Schema: -; Owner: -
--

COMMENT ON EXTENSION vector IS 'vector data type and ivfflat and hnsw access methods';


--
-- Name: scrape_job_status; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.scrape_job_status AS ENUM (
    'pending',
    'scraping',
    'extracting',
    'syncing',
    'completed',
    'failed'
);


--
-- Name: haversine_distance(double precision, double precision, double precision, double precision); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.haversine_distance(lat1 double precision, lng1 double precision, lat2 double precision, lng2 double precision) RETURNS double precision
    LANGUAGE sql IMMUTABLE STRICT
    AS $$
    SELECT 3959.0 * acos(
        LEAST(1.0, GREATEST(-1.0,
            cos(radians(lat1)) * cos(radians(lat2)) *
            cos(radians(lng2) - radians(lng1)) +
            sin(radians(lat1)) * sin(radians(lat2))
        ))
    )
$$;


--
-- Name: haversine_distance(numeric, numeric, numeric, numeric); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.haversine_distance(lat1 numeric, lng1 numeric, lat2 numeric, lng2 numeric) RETURNS numeric
    LANGUAGE plpgsql IMMUTABLE
    AS $$
DECLARE
    r NUMERIC := 6371; -- Earth radius in kilometers
    dlat NUMERIC;
    dlng NUMERIC;
    a NUMERIC;
    c NUMERIC;
BEGIN
    -- Handle NULL inputs
    IF lat1 IS NULL OR lng1 IS NULL OR lat2 IS NULL OR lng2 IS NULL THEN
        RETURN NULL;
    END IF;

    dlat := radians(lat2 - lat1);
    dlng := radians(lng2 - lng1);

    a := sin(dlat/2) * sin(dlat/2) +
         cos(radians(lat1)) * cos(radians(lat2)) *
         sin(dlng/2) * sin(dlng/2);

    c := 2 * atan2(sqrt(a), sqrt(1-a));

    RETURN r * c; -- Distance in kilometers
END;
$$;


--
-- Name: FUNCTION haversine_distance(lat1 numeric, lng1 numeric, lat2 numeric, lng2 numeric); Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON FUNCTION public.haversine_distance(lat1 numeric, lng1 numeric, lat2 numeric, lng2 numeric) IS 'Calculate distance in kilometers between two lat/lng points using Haversine formula';


--
-- Name: search_posts_by_similarity(public.vector, double precision, integer); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.search_posts_by_similarity(query_embedding public.vector, match_threshold double precision DEFAULT 0.6, match_count integer DEFAULT 20) RETURNS TABLE(post_id uuid, title text, description text, organization_name text, category text, post_type text, similarity double precision)
    LANGUAGE plpgsql
    AS $$
BEGIN
    RETURN QUERY
    SELECT
        p.id as post_id,
        p.title,
        p.description,
        p.organization_name,
        p.category,
        p.post_type,
        1 - (p.embedding <=> query_embedding) as similarity
    FROM posts p
    WHERE p.embedding IS NOT NULL
        AND p.deleted_at IS NULL
        AND p.status = 'active'
        AND 1 - (p.embedding <=> query_embedding) > match_threshold
    ORDER BY p.embedding <=> query_embedding
    LIMIT match_count;
END;
$$;


--
-- Name: search_websites_by_similarity(public.vector, double precision, integer); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.search_websites_by_similarity(query_embedding public.vector, match_threshold double precision DEFAULT 0.6, match_count integer DEFAULT 20) RETURNS TABLE(website_id uuid, assessment_id uuid, website_url text, organization_name text, recommendation text, assessment_markdown text, similarity double precision)
    LANGUAGE plpgsql
    AS $$
BEGIN
    RETURN QUERY
    SELECT
        w.id as website_id,
        wa.id as assessment_id,
        w.url as website_url,
        wa.organization_name,
        wa.recommendation,
        wa.assessment_markdown,
        (1 - (wa.embedding <=> query_embedding))::float as similarity
    FROM website_assessments wa
    JOIN websites w ON w.id = wa.website_id
    WHERE wa.embedding IS NOT NULL
        AND 1 - (wa.embedding <=> query_embedding) > match_threshold
    ORDER BY wa.embedding <=> query_embedding
    LIMIT match_count;
END;
$$;


--
-- Name: seesaw_notify_saga(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.seesaw_notify_saga() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM pg_notify(
        'seesaw_saga_' || NEW.correlation_id::text,
        json_build_object(
            'event_id', NEW.event_id,
            'correlation_id', NEW.correlation_id,
            'event_type', NEW.event_type
        )::text
    );
    RETURN NEW;
END;
$$;


--
-- Name: update_identifiers_updated_at(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.update_identifiers_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;


--
-- Name: update_page_snapshot_listings_count(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.update_page_snapshot_listings_count() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF NEW.page_snapshot_id IS NOT NULL THEN
    UPDATE page_snapshots
    SET listings_extracted_count = (
      SELECT COUNT(*)
      FROM posts
      WHERE page_snapshot_id = NEW.page_snapshot_id
    )
    WHERE id = NEW.page_snapshot_id;
  END IF;
  RETURN NEW;
END;
$$;


--
-- Name: update_posts_updated_at(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.update_posts_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;


--
-- Name: update_scrape_jobs_updated_at(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.update_scrape_jobs_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: _sqlx_migrations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public._sqlx_migrations (
    version bigint NOT NULL,
    description text NOT NULL,
    installed_on timestamp with time zone DEFAULT now() NOT NULL,
    success boolean NOT NULL,
    checksum bytea NOT NULL,
    execution_time bigint NOT NULL
);


--
-- Name: active_languages; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.active_languages (
    language_code text NOT NULL,
    language_name text NOT NULL,
    native_name text NOT NULL,
    enabled boolean DEFAULT true,
    added_at timestamp with time zone DEFAULT now()
);


--
-- Name: TABLE active_languages; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.active_languages IS 'Dynamic language system - add languages without code changes';


--
-- Name: COLUMN active_languages.language_code; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.active_languages.language_code IS 'ISO 639-1 code (en, es, so, etc.)';


--
-- Name: agent_assistant_configs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.agent_assistant_configs (
    agent_id uuid NOT NULL,
    preamble text DEFAULT ''::text NOT NULL,
    config_name text DEFAULT 'admin'::text NOT NULL
);


--
-- Name: agents; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.agents (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    member_id uuid NOT NULL,
    display_name text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    role text DEFAULT 'assistant'::text NOT NULL,
    status text DEFAULT 'active'::text NOT NULL,
    CONSTRAINT agents_role_check CHECK ((role = ANY (ARRAY['assistant'::text, 'curator'::text]))),
    CONSTRAINT agents_status_check CHECK ((status = ANY (ARRAY['draft'::text, 'active'::text, 'paused'::text])))
);


--
-- Name: business_listings; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.business_listings (
    listing_id uuid NOT NULL,
    business_type text,
    support_needed text[],
    current_situation text,
    accepts_donations boolean DEFAULT false,
    donation_link text,
    gift_cards_available boolean DEFAULT false,
    gift_card_link text,
    remote_ok boolean DEFAULT false,
    delivery_available boolean DEFAULT false,
    online_ordering_link text
);


--
-- Name: TABLE business_listings; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.business_listings IS 'Business-specific properties (economic solidarity)';


--
-- Name: contacts; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.contacts (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    contactable_type text NOT NULL,
    contactable_id uuid NOT NULL,
    contact_type text NOT NULL,
    contact_value text NOT NULL,
    contact_label text,
    is_public boolean DEFAULT true,
    display_order integer DEFAULT 0,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT contacts_contact_type_check CHECK ((contact_type = ANY (ARRAY['phone'::text, 'email'::text, 'website'::text, 'address'::text, 'booking_url'::text, 'social'::text])))
);


--
-- Name: containers; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.containers (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    language text DEFAULT 'en'::text,
    created_at timestamp with time zone DEFAULT now(),
    last_activity_at timestamp with time zone DEFAULT now(),
    tags jsonb DEFAULT '{}'::jsonb
);


--
-- Name: TABLE containers; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.containers IS 'Generic message containers for AI chat, listing comments, org discussions, etc.';


--
-- Name: COLUMN containers.language; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.containers.language IS 'Language for this conversation';


--
-- Name: document_references; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.document_references (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    document_id uuid NOT NULL,
    reference_kind text NOT NULL,
    reference_id text NOT NULL,
    referenced_at timestamp with time zone DEFAULT now(),
    display_order integer DEFAULT 0,
    CONSTRAINT document_references_reference_kind_check CHECK ((reference_kind = ANY (ARRAY['listing'::text, 'organization'::text, 'contact'::text])))
);


--
-- Name: TABLE document_references; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.document_references IS 'Tracks entities referenced in documents for staleness detection';


--
-- Name: COLUMN document_references.reference_kind; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.document_references.reference_kind IS 'Type of entity: listing, organization, contact';


--
-- Name: COLUMN document_references.reference_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.document_references.reference_id IS 'UUID of referenced entity';


--
-- Name: page_snapshots; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.page_snapshots (
    id uuid NOT NULL,
    url text NOT NULL,
    content_hash bytea NOT NULL,
    html text NOT NULL,
    markdown text,
    fetched_via text NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    crawled_at timestamp with time zone NOT NULL,
    listings_extracted_count integer DEFAULT 0,
    extraction_completed_at timestamp with time zone,
    extraction_status text DEFAULT 'pending'::text,
    CONSTRAINT page_snapshots_extraction_status_check CHECK ((extraction_status = ANY (ARRAY['pending'::text, 'processing'::text, 'completed'::text, 'failed'::text])))
);


--
-- Name: COLUMN page_snapshots.listings_extracted_count; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.page_snapshots.listings_extracted_count IS 'Cached count of listings extracted from this page snapshot. Updated automatically via trigger.';


--
-- Name: post_sources; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.post_sources (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    post_id uuid NOT NULL,
    source_type text NOT NULL,
    source_id uuid NOT NULL,
    source_url text,
    first_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    disappeared_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: posts; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.posts (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    title text NOT NULL,
    description text NOT NULL,
    description_markdown text,
    summary text,
    urgency text,
    status text DEFAULT 'pending_approval'::text,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    submission_type text DEFAULT 'scraped'::text,
    submitted_by_member_id uuid,
    location text,
    submitted_from_ip inet,
    fingerprint text,
    extraction_confidence text,
    latitude numeric(9,6),
    longitude numeric(9,6),
    source_url text,
    post_type text DEFAULT 'service'::text NOT NULL,
    category text DEFAULT 'other'::text NOT NULL,
    capacity_status text DEFAULT 'accepting'::text,
    verified_at timestamp with time zone,
    source_language text DEFAULT 'en'::text NOT NULL,
    page_snapshot_id uuid,
    title_normalized text GENERATED ALWAYS AS (lower(regexp_replace(title, '[^a-z0-9]'::text, ''::text, 'gi'::text))) STORED,
    disappeared_at timestamp with time zone,
    deleted_at timestamp with time zone,
    deleted_reason text,
    embedding public.vector(1024),
    revision_of_post_id uuid,
    comments_container_id uuid,
    translation_of_id uuid,
    submitted_by_id uuid,
    published_at timestamp with time zone,
    duplicate_of_id uuid,
    CONSTRAINT chk_needs_lat CHECK (((latitude >= ('-90'::integer)::numeric) AND (latitude <= (90)::numeric))),
    CONSTRAINT chk_needs_lng CHECK (((longitude >= ('-180'::integer)::numeric) AND (longitude <= (180)::numeric))),
    CONSTRAINT listings_capacity_status_check CHECK ((capacity_status = ANY (ARRAY['accepting'::text, 'paused'::text, 'at_capacity'::text]))),
    CONSTRAINT listings_status_check CHECK ((status = ANY (ARRAY['pending_approval'::text, 'active'::text, 'filled'::text, 'rejected'::text, 'expired'::text, 'archived'::text]))),
    CONSTRAINT listings_submission_type_check CHECK ((submission_type = ANY (ARRAY['scraped'::text, 'admin'::text, 'org_submitted'::text]))),
    CONSTRAINT listings_urgency_check CHECK ((urgency = ANY (ARRAY['low'::text, 'medium'::text, 'high'::text, 'urgent'::text]))),
    CONSTRAINT organization_needs_extraction_confidence_check CHECK ((extraction_confidence = ANY (ARRAY['high'::text, 'medium'::text, 'low'::text]))),
    CONSTRAINT posts_post_type_check CHECK ((post_type = ANY (ARRAY['service'::text, 'opportunity'::text, 'business'::text, 'professional'::text])))
);


--
-- Name: TABLE posts; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.posts IS 'Base listings table (services, opportunities, businesses)';


--
-- Name: COLUMN posts.submission_type; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.posts.submission_type IS 'How this need was created: scraped | user_submitted';


--
-- Name: COLUMN posts.submitted_by_member_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.posts.submitted_by_member_id IS 'Member who submitted this need (for user-submitted needs only)';


--
-- Name: COLUMN posts.location; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.posts.location IS 'Location/area for this need (e.g., "North Minneapolis", "Downtown St. Paul")';


--
-- Name: COLUMN posts.submitted_from_ip; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.posts.submitted_from_ip IS 'IP address of submitter (for geolocation, spam prevention)';


--
-- Name: COLUMN posts.fingerprint; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.posts.fingerprint IS 'Normalized content fingerprint: lowercase, no punctuation, first 300 chars of description. Used to detect content changes more reliably than title matching.';


--
-- Name: COLUMN posts.extraction_confidence; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.posts.extraction_confidence IS 'AI extraction confidence (high/medium/low). Used for admin triage, NOT decision-making. Sort low confidence first in approval queue.';


--
-- Name: COLUMN posts.latitude; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.posts.latitude IS 'Latitude inherited from organization/source for proximity matching';


--
-- Name: COLUMN posts.longitude; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.posts.longitude IS 'Longitude inherited from organization/source for proximity matching';


--
-- Name: COLUMN posts.source_url; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.posts.source_url IS 'The specific page URL where this need was scraped from (may be different from the main organization source URL)';


--
-- Name: COLUMN posts.post_type; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.posts.post_type IS 'Hot path: service/opportunity/business (affects routing)';


--
-- Name: COLUMN posts.category; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.posts.category IS 'Hot path: primary navigation filter (food, housing, healthcare, legal)';


--
-- Name: COLUMN posts.capacity_status; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.posts.capacity_status IS 'Hot path: visibility filter (accepting/paused/at_capacity)';


--
-- Name: COLUMN posts.page_snapshot_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.posts.page_snapshot_id IS 'Links to the cached page content (HTML/markdown) that this listing was extracted from. Enables viewing original source.';


--
-- Name: COLUMN posts.deleted_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.posts.deleted_at IS 'Soft delete timestamp - post is hidden but preserved for link continuity';


--
-- Name: COLUMN posts.deleted_reason; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.posts.deleted_reason IS 'Reason for deletion, e.g. "Duplicate of post <uuid>" or "Merged into <uuid>"';


--
-- Name: COLUMN posts.embedding; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.posts.embedding IS 'Semantic embedding (1024 dimensions) for search';


--
-- Name: sources; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.sources (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source_type text NOT NULL,
    url text,
    organization_id uuid,
    status text DEFAULT 'pending_review'::text NOT NULL,
    active boolean DEFAULT true NOT NULL,
    scrape_frequency_hours integer DEFAULT 24 NOT NULL,
    last_scraped_at timestamp with time zone,
    submitted_by uuid,
    submitter_type text,
    submission_context text,
    reviewed_by uuid,
    reviewed_at timestamp with time zone,
    rejection_reason text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: website_snapshots; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.website_snapshots (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    website_id uuid NOT NULL,
    page_url text NOT NULL,
    page_snapshot_id uuid,
    submitted_by uuid,
    submitted_at timestamp with time zone DEFAULT now() NOT NULL,
    last_scraped_at timestamp with time zone,
    scrape_status text DEFAULT 'pending'::text NOT NULL,
    scrape_error text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    last_synced_at timestamp with time zone,
    CONSTRAINT domain_snapshots_scrape_status_check CHECK ((scrape_status = ANY (ARRAY['pending'::text, 'scraped'::text, 'failed'::text])))
);


--
-- Name: COLUMN website_snapshots.last_synced_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.website_snapshots.last_synced_at IS 'When the extraction library last synced content for this URL';


--
-- Name: website_sources; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.website_sources (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source_id uuid NOT NULL,
    domain text NOT NULL,
    max_crawl_depth integer DEFAULT 2 NOT NULL,
    crawl_rate_limit_seconds integer DEFAULT 5 NOT NULL,
    is_trusted boolean DEFAULT false NOT NULL
);


--
-- Name: domain_statistics; Type: VIEW; Schema: public; Owner: -
--

CREATE VIEW public.domain_statistics AS
 SELECT s.id AS domain_id,
    ws.domain AS domain_url,
    s.status AS domain_status,
    count(DISTINCT ds.id) AS total_page_urls,
    count(DISTINCT ds.id) FILTER (WHERE (ds.scrape_status = 'scraped'::text)) AS scraped_pages,
    count(DISTINCT ds.id) FILTER (WHERE (ds.scrape_status = 'pending'::text)) AS pending_pages,
    count(DISTINCT ds.id) FILTER (WHERE (ds.scrape_status = 'failed'::text)) AS failed_pages,
    count(DISTINCT ps2.id) AS total_snapshots,
    count(DISTINCT l.id) AS total_listings,
    count(DISTINCT l.id) FILTER (WHERE (l.status = 'active'::text)) AS active_listings,
    count(DISTINCT l.id) FILTER (WHERE (l.status = 'pending_approval'::text)) AS pending_listings,
    max(ds.last_scraped_at) AS last_scraped_at,
    s.created_at AS domain_created_at
   FROM (((((public.sources s
     JOIN public.website_sources ws ON ((ws.source_id = s.id)))
     LEFT JOIN public.website_snapshots ds ON ((ds.website_id = s.id)))
     LEFT JOIN public.page_snapshots ps2 ON ((ps2.id = ds.page_snapshot_id)))
     LEFT JOIN public.post_sources src ON (((src.source_type = 'website'::text) AND (src.source_id = s.id))))
     LEFT JOIN public.posts l ON ((l.id = src.post_id)))
  WHERE (s.source_type = 'website'::text)
  GROUP BY s.id, ws.domain, s.status, s.created_at;


--
-- Name: extraction_embeddings; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.extraction_embeddings (
    url text NOT NULL,
    site_url text NOT NULL,
    embedding bytea NOT NULL
);


--
-- Name: extraction_pages; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.extraction_pages (
    url text NOT NULL,
    site_url text NOT NULL,
    content text NOT NULL,
    content_hash text NOT NULL,
    fetched_at timestamp with time zone NOT NULL,
    title text,
    http_headers jsonb DEFAULT '{}'::jsonb NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: extraction_summaries; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.extraction_summaries (
    url text NOT NULL,
    site_url text NOT NULL,
    text text NOT NULL,
    signals jsonb DEFAULT '{}'::jsonb NOT NULL,
    language text,
    created_at timestamp with time zone NOT NULL,
    prompt_hash text NOT NULL,
    content_hash text NOT NULL
);


--
-- Name: identifiers; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.identifiers (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    member_id uuid NOT NULL,
    phone_hash character varying(64) NOT NULL,
    is_admin boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: jobs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.jobs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    job_type text NOT NULL,
    args jsonb,
    next_run_at timestamp with time zone,
    last_run_at timestamp with time zone,
    max_retries integer DEFAULT 3 NOT NULL,
    retry_count integer DEFAULT 0 NOT NULL,
    version integer DEFAULT 1 NOT NULL,
    idempotency_key text,
    reference_id uuid,
    priority integer DEFAULT 0 NOT NULL,
    error_message text,
    error_kind text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    frequency text,
    timezone text DEFAULT 'UTC'::text NOT NULL,
    overlap_policy text DEFAULT 'skip'::text NOT NULL,
    misfire_policy text DEFAULT 'skip_to_latest'::text NOT NULL,
    timeout_ms bigint DEFAULT 300000 NOT NULL,
    lease_duration_ms bigint DEFAULT 60000 NOT NULL,
    lease_expires_at timestamp with time zone,
    worker_id text,
    enabled boolean DEFAULT true NOT NULL,
    container_id uuid,
    workflow_id uuid,
    dead_lettered_at timestamp with time zone,
    dead_letter_reason text,
    replay_count integer DEFAULT 0 NOT NULL,
    resolved_at timestamp with time zone,
    resolution_note text,
    root_job_id uuid,
    dedupe_key text,
    attempt integer DEFAULT 1 NOT NULL,
    command_version integer DEFAULT 1 NOT NULL
);


--
-- Name: TABLE jobs; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.jobs IS 'Background job queue for seesaw-rs commands';


--
-- Name: COLUMN jobs.status; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.status IS 'Job status: pending, running, completed, failed, dead_letter';


--
-- Name: COLUMN jobs.job_type; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.job_type IS 'Type of job (e.g., scrape_resource_link, extract_needs)';


--
-- Name: COLUMN jobs.args; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.args IS 'Serialized command payload';


--
-- Name: COLUMN jobs.idempotency_key; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.idempotency_key IS 'Prevents duplicate job execution';


--
-- Name: COLUMN jobs.frequency; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.frequency IS 'RRULE or cron expression for recurring jobs';


--
-- Name: COLUMN jobs.timezone; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.timezone IS 'Timezone for scheduling (default UTC)';


--
-- Name: COLUMN jobs.overlap_policy; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.overlap_policy IS 'How to handle overlapping runs: allow, skip, coalesce_latest';


--
-- Name: COLUMN jobs.misfire_policy; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.misfire_policy IS 'How to handle missed runs: catch_up, skip_to_latest';


--
-- Name: COLUMN jobs.timeout_ms; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.timeout_ms IS 'Job execution timeout in milliseconds';


--
-- Name: COLUMN jobs.lease_duration_ms; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.lease_duration_ms IS 'How long a worker can hold the job';


--
-- Name: COLUMN jobs.lease_expires_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.lease_expires_at IS 'When the current lease expires';


--
-- Name: COLUMN jobs.worker_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.worker_id IS 'ID of worker currently processing this job';


--
-- Name: COLUMN jobs.enabled; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.enabled IS 'Whether the job is enabled for execution';


--
-- Name: COLUMN jobs.container_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.container_id IS 'Multi-tenancy container ID';


--
-- Name: COLUMN jobs.workflow_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.workflow_id IS 'Parent workflow ID for job coordination';


--
-- Name: COLUMN jobs.dead_lettered_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.dead_lettered_at IS 'When job was moved to dead letter';


--
-- Name: COLUMN jobs.dead_letter_reason; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.dead_letter_reason IS 'Why job was dead lettered';


--
-- Name: COLUMN jobs.replay_count; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.replay_count IS 'Number of times job was replayed from dead letter';


--
-- Name: COLUMN jobs.resolved_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.resolved_at IS 'When dead letter was resolved';


--
-- Name: COLUMN jobs.resolution_note; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.resolution_note IS 'Notes about dead letter resolution';


--
-- Name: COLUMN jobs.root_job_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.root_job_id IS 'Original job ID in retry chain';


--
-- Name: COLUMN jobs.dedupe_key; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.dedupe_key IS 'Key for deduplication';


--
-- Name: COLUMN jobs.attempt; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.attempt IS 'Current attempt number';


--
-- Name: COLUMN jobs.command_version; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.jobs.command_version IS 'Version of the command schema';


--
-- Name: listing_contacts; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.listing_contacts (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    listing_id uuid NOT NULL,
    contact_type text NOT NULL,
    contact_value text NOT NULL,
    contact_label text,
    display_order integer DEFAULT 0,
    CONSTRAINT listing_contacts_contact_type_check CHECK ((contact_type = ANY (ARRAY['phone'::text, 'email'::text, 'website'::text, 'address'::text])))
);


--
-- Name: listing_delivery_modes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.listing_delivery_modes (
    listing_id uuid NOT NULL,
    delivery_mode text NOT NULL,
    CONSTRAINT listing_delivery_modes_delivery_mode_check CHECK ((delivery_mode = ANY (ARRAY['in_person'::text, 'phone'::text, 'online'::text, 'mail'::text, 'home_visit'::text])))
);


--
-- Name: TABLE listing_delivery_modes; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.listing_delivery_modes IS 'How services can be accessed (multiple modes possible)';


--
-- Name: listing_reports; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.listing_reports (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    listing_id uuid NOT NULL,
    reported_by uuid,
    reporter_email text,
    reason text NOT NULL,
    category text NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    resolved_by uuid,
    resolved_at timestamp with time zone,
    resolution_notes text,
    action_taken text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT listing_reports_action_taken_check CHECK ((action_taken = ANY (ARRAY['listing_deleted'::text, 'listing_rejected'::text, 'listing_updated'::text, 'no_action'::text]))),
    CONSTRAINT listing_reports_category_check CHECK ((category = ANY (ARRAY['inappropriate_content'::text, 'spam'::text, 'misleading_information'::text, 'duplicate'::text, 'outdated'::text, 'offensive'::text, 'other'::text]))),
    CONSTRAINT listing_reports_status_check CHECK ((status = ANY (ARRAY['pending'::text, 'resolved'::text, 'dismissed'::text])))
);


--
-- Name: locations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.locations (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name text,
    address_line_1 text,
    address_line_2 text,
    city text,
    state text,
    postal_code text,
    latitude double precision,
    longitude double precision,
    location_type text DEFAULT 'physical'::text NOT NULL,
    accessibility_notes text,
    transportation_notes text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: TABLE locations; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.locations IS 'Physical, virtual, or postal locations where services are delivered (HSDS-aligned)';


--
-- Name: COLUMN locations.location_type; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.locations.location_type IS 'physical, virtual, or postal (HSDS location_type)';


--
-- Name: members; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.members (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    expo_push_token text NOT NULL,
    searchable_text text NOT NULL,
    active boolean DEFAULT true,
    notification_count_this_week integer DEFAULT 0,
    paused_until timestamp with time zone,
    created_at timestamp with time zone DEFAULT now(),
    latitude numeric(9,6),
    longitude numeric(9,6),
    location_name text,
    embedding public.vector(1024),
    CONSTRAINT chk_members_lat CHECK (((latitude >= ('-90'::integer)::numeric) AND (latitude <= (90)::numeric))),
    CONSTRAINT chk_members_lng CHECK (((longitude >= ('-180'::integer)::numeric) AND (longitude <= (180)::numeric)))
);


--
-- Name: TABLE members; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.members IS 'Privacy-first member registry (zero PII, only expo_push_token)';


--
-- Name: COLUMN members.searchable_text; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.members.searchable_text IS 'TEXT-FIRST source of truth: all capabilities, skills, interests';


--
-- Name: COLUMN members.latitude; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.members.latitude IS 'Coarse latitude (city-level precision, 2 decimal places stored) - required for matching';


--
-- Name: COLUMN members.longitude; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.members.longitude IS 'Coarse longitude (city-level precision, 2 decimal places stored) - required for matching';


--
-- Name: COLUMN members.location_name; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.members.location_name IS 'Human-readable location for display (e.g., "Minneapolis, MN")';


--
-- Name: COLUMN members.embedding; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.members.embedding IS 'Semantic embedding (1024 dimensions from text-embedding-3-small)';


--
-- Name: messages; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.messages (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    container_id uuid NOT NULL,
    role text NOT NULL,
    content text NOT NULL,
    created_at timestamp with time zone DEFAULT now(),
    sequence_number integer NOT NULL,
    author_id uuid,
    moderation_status text DEFAULT 'approved'::text,
    parent_message_id uuid,
    updated_at timestamp with time zone DEFAULT now(),
    edited_at timestamp with time zone,
    CONSTRAINT messages_moderation_status_check CHECK ((moderation_status = ANY (ARRAY['approved'::text, 'pending'::text, 'flagged'::text, 'removed'::text]))),
    CONSTRAINT messages_role_check CHECK ((role = ANY (ARRAY['user'::text, 'assistant'::text, 'comment'::text])))
);


--
-- Name: TABLE messages; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.messages IS 'Messages in containers (AI chat messages, public comments, etc.)';


--
-- Name: COLUMN messages.role; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.messages.role IS 'Message role: user, assistant (for AI chat), comment (for public discussions)';


--
-- Name: COLUMN messages.sequence_number; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.messages.sequence_number IS 'Message order in conversation';


--
-- Name: COLUMN messages.author_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.messages.author_id IS 'Optional member ID - null for anonymous comments or AI messages';


--
-- Name: COLUMN messages.moderation_status; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.messages.moderation_status IS 'Moderation status: approved, pending, flagged, removed';


--
-- Name: COLUMN messages.parent_message_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.messages.parent_message_id IS 'For threaded discussions - parent message ID';


--
-- Name: migration_workflows; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.migration_workflows (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name text NOT NULL,
    phase text DEFAULT 'running'::text NOT NULL,
    total_items bigint DEFAULT 0 NOT NULL,
    completed_items bigint DEFAULT 0 NOT NULL,
    failed_items bigint DEFAULT 0 NOT NULL,
    skipped_items bigint DEFAULT 0 NOT NULL,
    last_processed_id uuid,
    dry_run boolean DEFAULT true NOT NULL,
    error_budget numeric(5,4) DEFAULT 0.01 NOT NULL,
    started_at timestamp with time zone DEFAULT now() NOT NULL,
    paused_at timestamp with time zone,
    completed_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: noteables; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.noteables (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    note_id uuid NOT NULL,
    noteable_type text NOT NULL,
    noteable_id uuid NOT NULL,
    added_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: notes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.notes (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    content text NOT NULL,
    severity text DEFAULT 'info'::text NOT NULL,
    source_url text,
    source_id uuid,
    source_type text,
    is_public boolean DEFAULT false NOT NULL,
    created_by text DEFAULT 'system'::text NOT NULL,
    expired_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    embedding public.vector(1024),
    cta_text text
);


--
-- Name: opportunity_listings; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.opportunity_listings (
    listing_id uuid NOT NULL,
    opportunity_type text DEFAULT 'other'::text NOT NULL,
    time_commitment text,
    requires_background_check boolean DEFAULT false,
    minimum_age integer,
    skills_needed text[],
    remote_ok boolean DEFAULT false,
    CONSTRAINT opportunity_listings_opportunity_type_check CHECK ((opportunity_type = ANY (ARRAY['volunteer'::text, 'donation'::text, 'customer'::text, 'partnership'::text, 'other'::text])))
);


--
-- Name: TABLE opportunity_listings; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.opportunity_listings IS 'Opportunity-specific properties (volunteer, donation, etc.)';


--
-- Name: organization_tags; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.organization_tags (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    organization_id uuid NOT NULL,
    kind text NOT NULL,
    value text NOT NULL,
    created_at timestamp with time zone DEFAULT now(),
    CONSTRAINT organization_tags_kind_check CHECK ((kind = ANY (ARRAY['service'::text, 'language'::text, 'community'::text])))
);


--
-- Name: TABLE organization_tags; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.organization_tags IS '
Flexible tagging for organizations.

Example service tags:
  - food_assistance, housing_assistance, legal_services
  - employment_support, emergency_financial_aid, shelter

Example language tags:
  - english, spanish, somali, hmong, karen, vietnamese

Example community tags:
  - latino, somali, hmong, karen, vietnamese, east_african, native_american, general
';


--
-- Name: COLUMN organization_tags.kind; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.organization_tags.kind IS 'Tag category: service (food_assistance), language (spanish), or community (somali)';


--
-- Name: COLUMN organization_tags.value; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.organization_tags.value IS 'Tag value: e.g., food_assistance, spanish, somali, general';


--
-- Name: organizations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.organizations (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name text NOT NULL,
    description text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    status text DEFAULT 'pending_review'::text NOT NULL,
    submitted_by uuid,
    submitter_type text,
    submission_context text,
    reviewed_by uuid,
    reviewed_at timestamp with time zone,
    rejection_reason text
);


--
-- Name: page_extractions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.page_extractions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    page_snapshot_id uuid NOT NULL,
    extraction_type text NOT NULL,
    content jsonb NOT NULL,
    model text,
    prompt_version text,
    tokens_used integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    is_current boolean DEFAULT true NOT NULL
);


--
-- Name: TABLE page_extractions; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.page_extractions IS 'AI-extracted content from page snapshots, versioned by type';


--
-- Name: COLUMN page_extractions.extraction_type; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.page_extractions.extraction_type IS 'Type of extraction: summary, posts, contacts, hours, events, etc.';


--
-- Name: COLUMN page_extractions.content; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.page_extractions.content IS 'JSON content structure varies by extraction_type';


--
-- Name: COLUMN page_extractions.is_current; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.page_extractions.is_current IS 'Only one extraction per page/type can be current';


--
-- Name: page_snapshot_details; Type: VIEW; Schema: public; Owner: -
--

CREATE VIEW public.page_snapshot_details AS
 SELECT ps.id AS snapshot_id,
    ps.url,
    ps.content_hash,
    ps.crawled_at,
    ps.fetched_via,
    ps.listings_extracted_count,
    ps.extraction_status,
    ps.extraction_completed_at,
    ds.id AS domain_snapshot_id,
    ds.website_id AS domain_id,
    ds.page_url AS submitted_page_url,
    ds.scrape_status,
    ds.last_scraped_at,
    ds.submitted_at,
    ws.domain AS domain_url,
    s.status AS domain_status,
    count(l.id) AS actual_listings_count
   FROM ((((public.page_snapshots ps
     LEFT JOIN public.website_snapshots ds ON ((ds.page_snapshot_id = ps.id)))
     LEFT JOIN public.sources s ON ((s.id = ds.website_id)))
     LEFT JOIN public.website_sources ws ON ((ws.source_id = s.id)))
     LEFT JOIN public.posts l ON ((l.page_snapshot_id = ps.id)))
  GROUP BY ps.id, ps.url, ps.content_hash, ps.crawled_at, ps.fetched_via, ps.listings_extracted_count, ps.extraction_status, ps.extraction_completed_at, ds.id, ds.website_id, ds.page_url, ds.scrape_status, ds.last_scraped_at, ds.submitted_at, ws.domain, s.status;


--
-- Name: page_summaries; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.page_summaries (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    page_snapshot_id uuid NOT NULL,
    content_hash text NOT NULL,
    content text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: post_locations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.post_locations (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    post_id uuid NOT NULL,
    location_id uuid NOT NULL,
    is_primary boolean DEFAULT false NOT NULL,
    notes text,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: TABLE post_locations; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.post_locations IS 'Links posts to locations (HSDS service_at_location equivalent)';


--
-- Name: post_page_sources; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.post_page_sources (
    post_id uuid NOT NULL,
    page_snapshot_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: providers; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.providers (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name text NOT NULL,
    bio text,
    why_statement text,
    headline text,
    profile_image_url text,
    member_id uuid,
    source_id uuid,
    location text,
    latitude double precision,
    longitude double precision,
    service_radius_km integer,
    offers_in_person boolean DEFAULT false,
    offers_remote boolean DEFAULT false,
    accepting_clients boolean DEFAULT true,
    status text DEFAULT 'pending_review'::text NOT NULL,
    submitted_by uuid,
    reviewed_by uuid,
    reviewed_at timestamp with time zone,
    rejection_reason text,
    embedding public.vector(1536),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT providers_status_check CHECK ((status = ANY (ARRAY['pending_review'::text, 'approved'::text, 'rejected'::text, 'suspended'::text])))
);


--
-- Name: referral_document_translations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.referral_document_translations (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    document_id uuid NOT NULL,
    language_code text NOT NULL,
    content text NOT NULL,
    title text,
    translated_at timestamp with time zone DEFAULT now(),
    translation_model text DEFAULT 'gpt-4o'::text
);


--
-- Name: TABLE referral_document_translations; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.referral_document_translations IS 'Translated versions of referral documents';


--
-- Name: referral_documents; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.referral_documents (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    container_id uuid,
    source_language text NOT NULL,
    content text NOT NULL,
    slug text NOT NULL,
    title text,
    status text DEFAULT 'draft'::text,
    edit_token text,
    view_count integer DEFAULT 0,
    last_viewed_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    CONSTRAINT referral_documents_status_check CHECK ((status = ANY (ARRAY['draft'::text, 'published'::text, 'archived'::text])))
);


--
-- Name: TABLE referral_documents; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.referral_documents IS 'Generated referral documents (markdown + components) - completely public, no auth';


--
-- Name: COLUMN referral_documents.content; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.referral_documents.content IS 'Markdown with JSX-like components: <Listing id="..." />, <Map>, <Contact>';


--
-- Name: COLUMN referral_documents.slug; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.referral_documents.slug IS 'Human-readable URL slug (e.g., warm-mountain-7423)';


--
-- Name: COLUMN referral_documents.edit_token; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.referral_documents.edit_token IS 'Secret token for editing (no auth required)';


--
-- Name: schedules; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.schedules (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    schedulable_type text NOT NULL,
    schedulable_id uuid NOT NULL,
    day_of_week integer,
    opens_at time without time zone,
    closes_at time without time zone,
    timezone text DEFAULT 'America/Chicago'::text NOT NULL,
    valid_from date,
    valid_to date,
    notes text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    dtstart timestamp with time zone,
    dtend timestamp with time zone,
    rrule text,
    exdates text,
    is_all_day boolean DEFAULT false NOT NULL,
    duration_minutes integer,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT schedules_day_of_week_check CHECK (((day_of_week IS NULL) OR ((day_of_week >= 0) AND (day_of_week <= 6))))
);


--
-- Name: TABLE schedules; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.schedules IS 'Operating hours for posts, locations, or post_locations (polymorphic)';


--
-- Name: COLUMN schedules.schedulable_type; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.schedules.schedulable_type IS 'post, location, or post_location';


--
-- Name: COLUMN schedules.day_of_week; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.schedules.day_of_week IS '0=Sunday through 6=Saturday';


--
-- Name: COLUMN schedules.dtstart; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.schedules.dtstart IS 'Start datetime for one-off or recurring events';


--
-- Name: COLUMN schedules.dtend; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.schedules.dtend IS 'End datetime for one-off events';


--
-- Name: COLUMN schedules.rrule; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.schedules.rrule IS 'RFC 5545 recurrence rule string (e.g. FREQ=WEEKLY;BYDAY=MO)';


--
-- Name: COLUMN schedules.exdates; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.schedules.exdates IS 'Comma-separated ISO dates for exception dates';


--
-- Name: COLUMN schedules.is_all_day; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.schedules.is_all_day IS 'Whether this is an all-day event';


--
-- Name: COLUMN schedules.duration_minutes; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.schedules.duration_minutes IS 'Duration in minutes (alternative to dtend for recurring)';


--
-- Name: scrape_jobs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.scrape_jobs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source_id uuid NOT NULL,
    status public.scrape_job_status DEFAULT 'pending'::public.scrape_job_status NOT NULL,
    error_message text,
    scraped_at timestamp with time zone,
    extracted_at timestamp with time zone,
    synced_at timestamp with time zone,
    new_needs_count integer,
    changed_needs_count integer,
    disappeared_needs_count integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    completed_at timestamp with time zone
);


--
-- Name: TABLE scrape_jobs; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.scrape_jobs IS 'Tracks async scraping jobs. GraphQL returns job_id immediately, admin polls for progress.';


--
-- Name: search_queries; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.search_queries (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    query_text text NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    sort_order integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: seesaw_dlq; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.seesaw_dlq (
    id bigint NOT NULL,
    event_id uuid NOT NULL,
    effect_id text NOT NULL,
    correlation_id uuid NOT NULL,
    error text NOT NULL,
    event_type text NOT NULL,
    event_payload jsonb DEFAULT '{}'::jsonb NOT NULL,
    reason text DEFAULT 'max_retries_exceeded'::text NOT NULL,
    attempts integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: seesaw_dlq_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.seesaw_dlq_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: seesaw_dlq_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.seesaw_dlq_id_seq OWNED BY public.seesaw_dlq.id;


--
-- Name: seesaw_effect_executions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.seesaw_effect_executions (
    event_id uuid NOT NULL,
    effect_id text NOT NULL,
    correlation_id uuid NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    event_type text NOT NULL,
    event_payload jsonb DEFAULT '{}'::jsonb NOT NULL,
    parent_event_id uuid,
    execute_at timestamp with time zone DEFAULT now() NOT NULL,
    timeout_seconds integer DEFAULT 30 NOT NULL,
    max_attempts integer DEFAULT 3 NOT NULL,
    priority integer DEFAULT 0 NOT NULL,
    attempts integer DEFAULT 0 NOT NULL,
    result jsonb,
    error text,
    claimed_at timestamp with time zone,
    last_attempted_at timestamp with time zone,
    completed_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    batch_id uuid,
    batch_index integer,
    batch_size integer
);


--
-- Name: seesaw_events; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.seesaw_events (
    id bigint NOT NULL,
    event_id uuid NOT NULL,
    parent_id uuid,
    correlation_id uuid NOT NULL,
    event_type text NOT NULL,
    payload jsonb DEFAULT '{}'::jsonb NOT NULL,
    hops integer DEFAULT 0 NOT NULL,
    retry_count integer DEFAULT 0 NOT NULL,
    locked_until timestamp with time zone,
    processed_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    batch_id uuid,
    batch_index integer,
    batch_size integer
)
PARTITION BY RANGE (created_at);


--
-- Name: seesaw_events_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.seesaw_events_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: seesaw_events_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.seesaw_events_id_seq OWNED BY public.seesaw_events.id;


--
-- Name: seesaw_events_default; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.seesaw_events_default (
    id bigint DEFAULT nextval('public.seesaw_events_id_seq'::regclass) NOT NULL,
    event_id uuid NOT NULL,
    parent_id uuid,
    correlation_id uuid NOT NULL,
    event_type text NOT NULL,
    payload jsonb DEFAULT '{}'::jsonb NOT NULL,
    hops integer DEFAULT 0 NOT NULL,
    retry_count integer DEFAULT 0 NOT NULL,
    locked_until timestamp with time zone,
    processed_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    batch_id uuid,
    batch_index integer,
    batch_size integer
);


--
-- Name: seesaw_join_entries; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.seesaw_join_entries (
    join_effect_id character varying NOT NULL,
    correlation_id uuid NOT NULL,
    source_event_id uuid NOT NULL,
    source_event_type character varying NOT NULL,
    source_payload jsonb NOT NULL,
    source_created_at timestamp with time zone NOT NULL,
    batch_id uuid NOT NULL,
    batch_index integer NOT NULL,
    batch_size integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: seesaw_join_windows; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.seesaw_join_windows (
    join_effect_id character varying NOT NULL,
    correlation_id uuid NOT NULL,
    mode character varying DEFAULT 'same_batch'::character varying NOT NULL,
    batch_id uuid NOT NULL,
    target_count integer NOT NULL,
    status character varying DEFAULT 'open'::character varying NOT NULL,
    sealed_at timestamp with time zone,
    processing_started_at timestamp with time zone,
    completed_at timestamp with time zone,
    last_error character varying,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: seesaw_processed; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.seesaw_processed (
    event_id uuid NOT NULL,
    correlation_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: seesaw_state; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.seesaw_state (
    correlation_id uuid NOT NULL,
    state jsonb DEFAULT '{}'::jsonb NOT NULL,
    version integer DEFAULT 0 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: service_listings; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.service_listings (
    listing_id uuid NOT NULL,
    requires_identification boolean DEFAULT false,
    remote_available boolean DEFAULT false,
    requires_appointment boolean DEFAULT false,
    walk_ins_accepted boolean DEFAULT true,
    in_person_available boolean DEFAULT true,
    home_visits_available boolean DEFAULT false,
    wheelchair_accessible boolean DEFAULT false,
    interpretation_available boolean DEFAULT false,
    free_service boolean DEFAULT false,
    sliding_scale_fees boolean DEFAULT false,
    accepts_insurance boolean DEFAULT false,
    evening_hours boolean DEFAULT false,
    weekend_hours boolean DEFAULT false
);


--
-- Name: TABLE service_listings; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.service_listings IS 'Generic service properties applicable across all service types (legal, healthcare, social services, etc.)';


--
-- Name: COLUMN service_listings.requires_identification; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.service_listings.requires_identification IS 'Service requires government-issued ID';


--
-- Name: COLUMN service_listings.remote_available; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.service_listings.remote_available IS 'Offers remote/virtual services (phone, video, online)';


--
-- Name: COLUMN service_listings.requires_appointment; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.service_listings.requires_appointment IS 'Must schedule appointment in advance';


--
-- Name: COLUMN service_listings.walk_ins_accepted; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.service_listings.walk_ins_accepted IS 'Accepts walk-in clients without appointment';


--
-- Name: COLUMN service_listings.in_person_available; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.service_listings.in_person_available IS 'Offers in-person services at physical location';


--
-- Name: COLUMN service_listings.home_visits_available; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.service_listings.home_visits_available IS 'Provider travels to client location';


--
-- Name: COLUMN service_listings.wheelchair_accessible; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.service_listings.wheelchair_accessible IS 'Physical location is wheelchair accessible';


--
-- Name: COLUMN service_listings.interpretation_available; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.service_listings.interpretation_available IS 'Provides language interpretation services';


--
-- Name: COLUMN service_listings.free_service; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.service_listings.free_service IS 'Service is completely free';


--
-- Name: COLUMN service_listings.sliding_scale_fees; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.service_listings.sliding_scale_fees IS 'Fees vary based on income/ability to pay';


--
-- Name: COLUMN service_listings.accepts_insurance; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.service_listings.accepts_insurance IS 'Accepts health/other insurance';


--
-- Name: COLUMN service_listings.evening_hours; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.service_listings.evening_hours IS 'Available during evening hours (after 5pm)';


--
-- Name: COLUMN service_listings.weekend_hours; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.service_listings.weekend_hours IS 'Available on weekends';


--
-- Name: social_sources; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.social_sources (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source_id uuid NOT NULL,
    source_type text NOT NULL,
    handle text NOT NULL
);


--
-- Name: sync_batches; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.sync_batches (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    resource_type text NOT NULL,
    source_id uuid,
    status text DEFAULT 'pending'::text NOT NULL,
    summary text,
    proposal_count integer DEFAULT 0 NOT NULL,
    approved_count integer DEFAULT 0 NOT NULL,
    rejected_count integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    reviewed_at timestamp with time zone,
    expires_at timestamp with time zone,
    CONSTRAINT sync_batches_status_check CHECK ((status = ANY (ARRAY['pending'::text, 'partially_reviewed'::text, 'completed'::text, 'expired'::text])))
);


--
-- Name: sync_proposal_merge_sources; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.sync_proposal_merge_sources (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    proposal_id uuid NOT NULL,
    source_entity_id uuid NOT NULL
);


--
-- Name: sync_proposals; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.sync_proposals (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    batch_id uuid NOT NULL,
    operation text NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    entity_type text NOT NULL,
    draft_entity_id uuid,
    target_entity_id uuid,
    reason text,
    reviewed_by uuid,
    reviewed_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT sync_proposals_operation_check CHECK ((operation = ANY (ARRAY['insert'::text, 'update'::text, 'delete'::text, 'merge'::text]))),
    CONSTRAINT sync_proposals_status_check CHECK ((status = ANY (ARRAY['pending'::text, 'approved'::text, 'rejected'::text])))
);


--
-- Name: tag_kinds; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.tag_kinds (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    slug text NOT NULL,
    display_name text NOT NULL,
    description text,
    allowed_resource_types text[] DEFAULT '{}'::text[] NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    required boolean DEFAULT false NOT NULL,
    is_public boolean DEFAULT false NOT NULL
);


--
-- Name: COLUMN tag_kinds.is_public; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.tag_kinds.is_public IS 'Whether tags of this kind are visible on the public home page';


--
-- Name: taggables; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.taggables (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tag_id uuid NOT NULL,
    taggable_type text NOT NULL,
    taggable_id uuid NOT NULL,
    added_at timestamp with time zone DEFAULT now(),
    CONSTRAINT taggables_taggable_type_check CHECK ((taggable_type = ANY (ARRAY['post'::text, 'organization'::text, 'referral_document'::text, 'domain'::text, 'provider'::text, 'container'::text, 'website'::text, 'resource'::text])))
);


--
-- Name: TABLE taggables; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.taggables IS 'Polymorphic join table: links tags to any entity (listing, organization, document, domain)';


--
-- Name: COLUMN taggables.taggable_type; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.taggables.taggable_type IS 'Entity type: listing, organization, referral_document, domain';


--
-- Name: COLUMN taggables.taggable_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.taggables.taggable_id IS 'UUID of the tagged entity';


--
-- Name: tags; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.tags (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    kind text NOT NULL,
    value text NOT NULL,
    display_name text,
    created_at timestamp with time zone DEFAULT now(),
    parent_tag_id uuid,
    color text,
    description text,
    emoji text
);


--
-- Name: TABLE tags; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.tags IS 'audience_role tags: recipient (receiving services), donor (giving money/goods), volunteer (giving time), participant (attending events/groups)';


--
-- Name: COLUMN tags.kind; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.tags.kind IS 'Tag type (community_served, service_area, population, org_leadership, etc.)';


--
-- Name: COLUMN tags.value; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.tags.value IS 'Tag value (somali, minneapolis, seniors, etc.)';


--
-- Name: COLUMN tags.display_name; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.tags.display_name IS 'Human-readable name for UI';


--
-- Name: COLUMN tags.parent_tag_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.tags.parent_tag_id IS 'Self-referential FK for tag hierarchy (e.g., Food > Food Pantries)';


--
-- Name: COLUMN tags.color; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.tags.color IS 'Optional hex color for display (e.g., #3b82f6)';


--
-- Name: COLUMN tags.description; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.tags.description IS 'Optional description of the tag purpose';


--
-- Name: tavily_search_queries; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.tavily_search_queries (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    website_research_id uuid NOT NULL,
    query text NOT NULL,
    search_depth character varying(20),
    max_results integer,
    days_filter integer,
    executed_at timestamp with time zone DEFAULT now()
);


--
-- Name: tavily_search_results; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.tavily_search_results (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    query_id uuid NOT NULL,
    title text NOT NULL,
    url text NOT NULL,
    content text NOT NULL,
    score double precision NOT NULL,
    published_date text,
    created_at timestamp with time zone DEFAULT now()
);


--
-- Name: taxonomy_crosswalks; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.taxonomy_crosswalks (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tag_id uuid NOT NULL,
    external_system text NOT NULL,
    external_code text NOT NULL,
    external_name text,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: TABLE taxonomy_crosswalks; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.taxonomy_crosswalks IS 'Maps internal tags to external taxonomy codes (211HSIS, Open Eligibility, NTEE)';


--
-- Name: website_assessments; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.website_assessments (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    website_id uuid NOT NULL,
    website_research_id uuid,
    assessment_markdown text NOT NULL,
    recommendation character varying(50) NOT NULL,
    confidence_score double precision,
    organization_name text,
    founded_year integer,
    generated_by uuid,
    generated_at timestamp with time zone DEFAULT now(),
    model_used character varying(100) NOT NULL,
    reviewed_by_human boolean DEFAULT false,
    human_notes text,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    embedding public.vector(1024)
);


--
-- Name: COLUMN website_assessments.embedding; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.website_assessments.embedding IS 'Semantic embedding (1024 dimensions from text-embedding-3-small)';


--
-- Name: website_research; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.website_research (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    website_id uuid NOT NULL,
    homepage_url text NOT NULL,
    homepage_fetched_at timestamp with time zone NOT NULL,
    tavily_searches_completed_at timestamp with time zone,
    created_by uuid,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now()
);


--
-- Name: website_research_homepage; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.website_research_homepage (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    website_research_id uuid NOT NULL,
    html text,
    markdown text,
    created_at timestamp with time zone DEFAULT now()
);


--
-- Name: website_snapshot_listings; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.website_snapshot_listings (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    website_snapshot_id uuid NOT NULL,
    listing_id uuid NOT NULL,
    extracted_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: website_snapshots_with_listings; Type: VIEW; Schema: public; Owner: -
--

CREATE VIEW public.website_snapshots_with_listings AS
SELECT
    NULL::uuid AS id,
    NULL::uuid AS website_id,
    NULL::text AS page_url,
    NULL::uuid AS page_snapshot_id,
    NULL::uuid AS submitted_by,
    NULL::timestamp with time zone AS submitted_at,
    NULL::timestamp with time zone AS last_scraped_at,
    NULL::text AS scrape_status,
    NULL::text AS scrape_error,
    NULL::timestamp with time zone AS created_at,
    NULL::timestamp with time zone AS updated_at,
    NULL::boolean AS has_listings,
    NULL::bigint AS listings_count;


--
-- Name: zip_codes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.zip_codes (
    zip_code text NOT NULL,
    city text NOT NULL,
    state text DEFAULT 'MN'::text NOT NULL,
    latitude double precision NOT NULL,
    longitude double precision NOT NULL
);


--
-- Name: seesaw_events_default; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.seesaw_events ATTACH PARTITION public.seesaw_events_default DEFAULT;


--
-- Name: seesaw_dlq id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.seesaw_dlq ALTER COLUMN id SET DEFAULT nextval('public.seesaw_dlq_id_seq'::regclass);


--
-- Name: seesaw_events id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.seesaw_events ALTER COLUMN id SET DEFAULT nextval('public.seesaw_events_id_seq'::regclass);


--
-- Data for Name: _sqlx_migrations; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public._sqlx_migrations (version, description, installed_on, success, checksum, execution_time) FROM stdin;
1	create extensions	2026-02-11 08:43:24.222669+00	t	\\xdfb08b1df79744231b38fbe8171bd5d6ff675e4c6067e7467e1ec49244eb9c5d4bfc0aa97a765e6b7b60b152b435493c	19192250
2	create organization sources	2026-02-11 08:43:24.24271+00	t	\\xdc5db8a09151dce4abc7cbc239017fc18e91b4a6bcfe581d83062578a9e2170589a371f55a81d163f886a6274c5d396c	5695084
3	create organization needs	2026-02-11 08:43:24.248814+00	t	\\xd356cf5f86456454b4d242a84a5a8e716482c9482aae3b53e419d392868b11e2325c9a6b1ae2d39fe0ae914520a7e9fc	6161167
4	create volunteers	2026-02-11 08:43:24.255288+00	t	\\xa1f35e2a2e6c3dd116a9a591d947a2103400002e7acd353479feec24d92e9efc7086f1ac13258c0dca624fa137cda913	5394167
5	add user submitted needs	2026-02-11 08:43:24.260989+00	t	\\xa1782d1c2f052a97348b7200af44828ca0a77d894e23bd87dabd4f4685752d4efdb7eaa5f2b2a2f61bc04bbc365d8578	1767791
6	add fingerprint and confidence	2026-02-11 08:43:24.263019+00	t	\\x5862e3b05eb13399e56d5a75e16f8c447f60258643296cc3e7e44745ee9235f64065cb049e86700fd8444bec76db666e	1587000
7	create organizations	2026-02-11 08:43:24.264833+00	t	\\xe815077f0a9d8ba69670f93797da5d916765b8b48b218afee7d9dcfb60b112e50c80dcf6da9fcc0ed1c5b5e8c95effce	3838750
8	create scrape jobs	2026-02-11 08:43:24.268906+00	t	\\xdb1a11f8a12158c1bfc51d3b471ba5f8f9df751fe40f05c7bd9f10c6b3ff28c26c35777155b3f069efda947ba82d971a	5885458
9	create organization tags	2026-02-11 08:43:24.275122+00	t	\\x99fa5239a5d38f8dcfd2117b40faccdaa7c17d87773ddb8bc49460f63800f1314b2534e09009f835998c5aab5d2c679c	6398166
10	create tags	2026-02-11 08:43:24.282019+00	t	\\x0a4fc91effc56816bf4c2613961c5db436fcb6001d8bd68ec8708a528a5fffb81b82dc446fd2340f52509a5e19dcc948	5581209
11	link organizations	2026-02-11 08:43:24.287791+00	t	\\xdabe48a018522156940ab2e0d4ded1f68444636361c43d2d14a97da3ce92d971b19e85b508e6d67a16f46c0d4b170d3f	3019917
12	create tags on organizations	2026-02-11 08:43:24.291172+00	t	\\x52d67ab95997de60a1a85a795b22dedabd3d283e89d50e6ab7d255c3679b2a04b027e4c6a81aca47ffedb97e9e3e9827	4544833
13	create posts	2026-02-11 08:43:24.296018+00	t	\\x5066b83694ccbfda266cc2460728d17b0f76003a60be2a30770fc04698b1bdb18a9929a1be46d960de3c529e876e118d	6255666
14	add location coordinates	2026-02-11 08:43:24.302503+00	t	\\x50fe7e6acc36f46b6562fbbed0934db5811731f902ba84560f6d2429f213e9aff6a1a03037b48c4ec72cd730c3314a6f	5908625
15	create identifiers	2026-02-11 08:43:24.308663+00	t	\\xcc583f44ede22e2f88d8c06ec4de6a388f8a783f2226c57bea182a1ceddb0e74040b1badfe1c6ba9ba342f3cd93c867d	3519000
16	create notifications	2026-02-11 08:43:24.31247+00	t	\\x7f55ff5a47114a0c8bab1c80212af003ee049a2725bd7d17bb9dd5c4442b1422d0db4535dd97a7d158171aec3cc986a2	5729083
17	add embeddings	2026-02-11 08:43:24.318501+00	t	\\xec4347554609d2e03ec375de3862b17583b6ac33e4a2cd159a6117cd49720b1f65221720901c762aec6d5819a4113881	45606708
18	add outreach copy to posts	2026-02-11 08:43:24.364702+00	t	\\x7fccdc162a23451292d52f790d2599c29b3b0c0494bb39ad3990367813397c70451fffe6a85f9747ce60ff2f82ac512f	2062875
19	improve vector search indexes	2026-02-11 08:43:24.367007+00	t	\\x2e2f53a9ef51a3f792e003bdf11fe2bad7c7ae0cad7c1c2f41f93b758aa1455288f4ae30059ff08e4d757dc5aaf43f34	204429917
20	add location to needs	2026-02-11 08:43:24.57198+00	t	\\xac1f296d2f2bf52a09afd8590494b13f7ee570d129537a6c1db2d7de3bd27c2f430310a71ec831bc0ba72909c49a6746	3085833
21	rename volunteer to member in needs	2026-02-11 08:43:24.575293+00	t	\\x5290fe9e8ba5b1a7baf6588a1ac75825a500a7b603241eaceabada2d9624dec047e58760a395c019f911b7ea57f15bf9	1644000
22	change embedding dimensions	2026-02-11 08:43:24.577203+00	t	\\xf58e94ddb3902423ba291fad7144e9b8f1023a1bfd3415a880780d30dd5d2449a5bd255d72360eaa4e857567f3974417	27028500
23	add last displayed at to posts	2026-02-11 08:43:24.604638+00	t	\\x2c4754d92535eeee4a1cbb64e33404ea6299564498eb2813371d15963741970eb4d760541f7acaa590d9c7745ddd2ed1	1604875
24	create jobs table	2026-02-11 08:43:24.60656+00	t	\\x0dad2f85b3b2aae170a88e370feaa1c192e078f4bda5bd3fc22823bbe9a6bc9f5dc8a0d3bfa40d7436d7c2d0d6a9b4ec	6989583
25	create intelligent crawler tables	2026-02-11 08:43:24.613852+00	t	\\x935f6e28d2983455d9b78c41a6ceab844af6f6242542806c620f91150f2356c26181233481f48d4f3d575ad50305bd53	28688583
26	add source url to needs	2026-02-11 08:43:24.642907+00	t	\\x4d09bc813fe94a60e04beb6205051fdb36ee2971851682109574fa7b22cdf87e0d87f5327d6adb2bcfc57f07b3019bfa	1959625
27	add scrape urls to sources	2026-02-11 08:43:24.645083+00	t	\\x388f75143dad8f7fdf9101062c1d03e80b735d1c1bea455e2004be235e88b5fb0194acba1a4b3e5b0097b6e673ea3cd8	802875
31	refactor to domains and organizations	2026-02-11 08:43:24.646122+00	t	\\x03b16fb5bf9d9c36a66379a722f2d95d09a46f85b602c355b7413eb841554c937a8ca1ceaf7ddcc560f6c4029317e683	18100375
32	refactor needs to listings	2026-02-11 08:43:24.664579+00	t	\\x02639b6b671c293aad982ca84c9f3f39eede8d2d87c6ca1b923829208fbb359bd71c80c2c9af65dc3f11e5a81930f079	24468666
33	create universal tagging system	2026-02-11 08:43:24.689446+00	t	\\xe8be8eb508261401acc6376239af2cb0a77d9c6a31c9238eaade520d075680f6ddd27f803d6b711e5e605788849be409	10218959
34	create multi language system	2026-02-11 08:43:24.699988+00	t	\\x3574421f63d965cf72e9605d15e0fd2c37f06cd3022bc24a08fcf70fb024c4d3f47b8bf522caa2a2009440ef548f85d4	7338042
35	create chatrooms and documents	2026-02-11 08:43:24.707678+00	t	\\x14faaecb55ea47b95ce98736f7d3407d50f78655581c5918b1e52e5d226b75bac004d839e49ec1ab16490bdab69fc3b6	22151583
36	cleanup old tables	2026-02-11 08:43:24.730203+00	t	\\xf930ddee7a660ef8419eceb504b994d8aad21b17f936cddbd2c242020422ecd12b37c90e1c90185916d4d0d1e3e7eea0	1834250
37	genericize service listings	2026-02-11 08:43:24.732482+00	t	\\x037a80dc63fbef38a8f75d5cd4e6c495a883b351f1ae84049c7386fc393a8601df3125f9f77cb157c1761e484574f6fc	5177291
38	add organization embeddings	2026-02-11 08:43:24.737881+00	t	\\x876edec4e267135339c9bebe45a02fa72ac93c4a71c1fd3d400645ed57012e07638ac12d61355e8f0394ac4bf0f45cc4	19523500
39	fix embedding dimensions	2026-02-11 08:43:24.757867+00	t	\\xa34383250abe38437c3f468f4cbc280200be46dda6e1dc5354db2b0b7aa246d1e66f49a60377c38fdf111e88bde98b04	1486208
40	add content hash unique constraint	2026-02-11 08:43:24.759663+00	t	\\x77884d89daf26aafa829fe11d5e17cf30cafa776222b129e37f4f87b7b68baf2424fe5994db2ddfc29c739953441d353	1749459
41	upgrade to hnsw indexes	2026-02-11 08:43:24.761661+00	t	\\x33a7a390f83a3ccf32e3c6c18da8c7483d370cd3a32a9b5b37f33d3337f2587a7716ea4f105fdb335ef0d3978b7bbefa	1598000
42	rename posts need id to listing id	2026-02-11 08:43:24.763535+00	t	\\x33e7e5a0187d93bcd3ebc2254f36c372f5df25b9dcf34eb45b6eec035d0415a0ad64e5c022411c2920bdb292ee609d59	2511042
44	remove government org type	2026-02-11 08:43:24.766309+00	t	\\xe33b8923dd7259d17b6aec88e59578ef9e00d3992b67a97615e944c753ab2696ec6bb66e42452dbb30a5c38e4a1ca8e5	991333
45	create business organizations minimal	2026-02-11 08:43:24.767497+00	t	\\x8a4882d132ea02d6c9d71b20a742106f8d47522cb2df7c25be4f36412f50987c564e3486229d2093f61b5a918881a3c8	4547042
46	generalize containers and messages	2026-02-11 08:43:24.772352+00	t	\\x2101405ff63a41eeb04d6109f9d85331b15d1e2e806d356a7527656dc751af5b03df8cace931625ab308b0024d56663a	8143291
47	add domain approval workflow	2026-02-11 08:43:24.78075+00	t	\\xc729740cb70deef021386c6bf28cd1b14a29692e43d525b8c26f9a661e2eb011ed49d00f5a8adb15baa62604568c2e05	6426542
48	add domain snapshots	2026-02-11 08:43:24.787493+00	t	\\x8fd2757dbb6003684cf56482d3108e3baf019089053f8e84d6253e2896b273f338f95aa65db487985c3cb4cb6a4f1689	7772917
49	wire up snapshot traceability	2026-02-11 08:43:24.795514+00	t	\\xcd01374614afdaa87c344e1921af8c9b20ccee9e6d8785cafbbecc65c6438a0384428d3d4aa88a638a5e96050f6c96a1	6949750
50	add tavily search topics	2026-02-11 08:43:24.802746+00	t	\\xa3f90b8536b7d30cb3b3dd51350f71ac24e2de4b484e2fafee46cd448cf57dbda22fa8149e142499729eac51f1f32958	6436500
52	migrate search topics to agents	2026-02-11 08:43:24.809495+00	t	\\xdb54a8cc5b902c54d356bf58ea6477843a8ffa8c90076b13f8bccc630177623ee3b6b05819d469fe1c6178f1dc7a5c8a	3217000
54	fix agent score type	2026-02-11 08:43:24.813122+00	t	\\x50bee2d6c3d3f52fd9218a5961998adf228dedc10110be4025406080bbdc825d015b6847a4f296def7bad613b5199f18	17104667
55	add listing reports	2026-02-11 08:43:24.830563+00	t	\\xee1f1d25c6304f3f5c9c6d473bda5e576a0c38b1ee0b0669e40ed39c78aa035762004e226dd4f5826cad3c893e1f8334	6392833
56	add domain assessment tables	2026-02-11 08:43:24.837208+00	t	\\x5abac5789f51f6a0fcfbc700297ca0d3700f8a864c63e18836ed564148204cfbcbe9cd36a1e48c95e9f6158e8e6954cb	17081167
57	rename domains to websites	2026-02-11 08:43:24.854613+00	t	\\x6b26ae6c693d8f3919f3d94cb65bbae4c9285f623124d519a7d156ff323467fe46d3048e2784cd2819b3ce540db6585a	12663541
58	fix website research homepage rename	2026-02-11 08:43:24.867674+00	t	\\xc3e709a8355757ba7b5812981b7842749ecab3439423f1b5cf10be8b6f343e9b602ccfeeadf6409aa89f704f0d5146e4	2309625
59	fix tavily tables rename	2026-02-11 08:43:24.870246+00	t	\\x3de0024277438747c50bcfb74593055f26f42d3deb82d3f2b1707b0d6dc7120022355afdd1d143157de682adf67523d1	1778375
60	fix website assessments rename	2026-02-11 08:43:24.872255+00	t	\\xa1a7a80576df640e6ba07811b1ac233623995053f845bedd71fd535287ae788c44edd8baf53bae72916a84d847e5a20d	2162833
61	fix confidence score type	2026-02-11 08:43:24.874716+00	t	\\x75495c06a1a4279400143db4f66ca7d870af9c9db84195d6312b3527c037391e9427851f6ab1db026b4e5df6db32f33c	5443541
62	fix tavily score type	2026-02-11 08:43:24.880429+00	t	\\xe48537dc3f3c2e8014df68acd883b40360e3dad900dba129c6de9d7b79293e331c25e3ff51ff714c3f38ffa409724b93	4320625
63	add website assessment embeddings	2026-02-11 08:43:24.885068+00	t	\\xa957cb16c6fc8ed27b8a00c3e50ef5a8107098c0139d323228ce422d6a2de2a679ff6aa59751c42e3588375200a66f90	29579625
64	rename agent domain columns to website	2026-02-11 08:43:24.91515+00	t	\\xea7a3ef8464c182516ed9b3c3da3c3a91ffb1a39f5c1c4c9bf4dd3f38335e6c1a6411825ff682d8fa1f179df850bf8f7	1042583
65	add website crawl tracking	2026-02-11 08:43:24.916549+00	t	\\xd2e74e51c7ec44e06cb630a702d4e11d39c3136f0ff54a2ec648c248d6b20fbb7e1f0d0346f0e30ca259d117e02f60ae	10001500
66	normalize website urls	2026-02-11 08:43:24.927003+00	t	\\xa7bde077cb6ba78efd11ead796834677e4a777fd09d8de98d9d9edf54ff1424d30307c84ade9af7de3cf39ddb10a5950	1009000
67	add migration workflows	2026-02-11 08:43:24.928265+00	t	\\x4f2c61b89bc2d74bfbf74999ab77bb8d3d50a14958f6f6abbdfdb39bba8c6a08ae5406ddf1fd428e0cab19044021174d	5222875
68	rename website url to domain	2026-02-11 08:43:24.933781+00	t	\\xb27dd0c80473cfd91ab281033e4bca3abd673c21f9d661bf2b5cd847fd571a4e2f9e005155d5fbb7ae0009a3e21bf973	2313667
71	create contacts	2026-02-11 08:43:24.936351+00	t	\\x809baa67798a95592654b2df50700427daa97bbbced9dbefaa4b2670b58995c4ba635a708d6514392b3013537a96ac5c	4434208
72	create providers	2026-02-11 08:43:24.941117+00	t	\\x287a640e2a881f28d1e9f6bc8c0e2f87f21581864af0a533b5f4896f12e14dd7d9ad2360acda8bd23196785e5c176dac	5841792
73	seed provider tags	2026-02-11 08:43:24.947198+00	t	\\x912f7481322058e328bb2dd038e376f72d675ded2fe0505a62d4cbed59089995079ae8a3d56878d7950f915b4b79426f	981083
74	seed audience role tags	2026-02-11 08:43:24.948422+00	t	\\x3818bc4c058166f631a3c082a476a8954c290041a5c378f043e534e0a941c839de1ac16cadc436462cce15280346e10b	896375
75	add container tags	2026-02-11 08:43:24.94955+00	t	\\x8082be72c5e7e75437a614ab2cf04f1332c03bffa71d43c131035a93b030ce183b380c06994623c1b61184aad7804b75	869375
77	fix migration bugs	2026-02-11 08:43:24.950649+00	t	\\xebe8423fe705fed4aec821a39274273f8939ab757a9b3ea775116616018317736c1175d82577956fd73a58db7d5e753e	828125
78	remove agent infrastructure	2026-02-11 08:43:24.951703+00	t	\\x867322137a007256cc8ccee2a51434149cb317c71becca37641b898e8d17e60f43f15dc107438b47fc22c0b1714eb09a	2503166
79	add service and business tags	2026-02-11 08:43:24.954461+00	t	\\x439f24af01a439289a37f9d66d20b8bf70d83c24ef988336dc0e7fcde394377ee789eef27ea827e1763cdebaed746da9	876209
80	create page summaries	2026-02-11 08:43:24.955562+00	t	\\x7f03da412578bc1b40c2720885b712d9138ceeae6e46025e76f55f7cb5ff103adcba28b9575ec059d6f07173063020d3	4844917
81	add listing deduplication	2026-02-11 08:43:24.960653+00	t	\\x5a27b6da5d30167ae02d7184eb54e4a56543b0090c0a8e6cad16fbfc2c934fac90c4f6621e764ad616d57c2decda0874	29168458
82	create listing page sources	2026-02-11 08:43:24.990362+00	t	\\xfa70abfb94575658d9731a40cb9fefdf051cd96c2209e499205e5e6809f2be863644908f0bc589736abb790abd997180	3709500
83	add population and listing type tags	2026-02-11 08:43:24.994295+00	t	\\x0af962cbdefdff5d3991ae21dc71a9ab186559d7d1fb9402e18644683b0f299ef00bb53d2383802e6d2207cd4e87e732	840042
84	add listings disappeared at	2026-02-11 08:43:24.99535+00	t	\\x3354a10668f97a4ce616ca087ba020a786846cc629ba1bd84956a7fb3c6694d7f6385bc8a8bb0dbdf665fc2388ffd0de	1443208
85	create resources	2026-02-11 08:43:24.997015+00	t	\\x8aef6d63bfededb9af199f0746e0864c5b4fdfd7c3b470900610826e07e10901c41c8c2717041838ea06fed4080c9d92	15335125
86	migrate listings to resources	2026-02-11 08:43:25.012709+00	t	\\x92388399b1ecc4568815e9011aea0359e57cc1f26c1973ac25a25dc6b6a2a7c7ef3b93d7c4473f6677970baac73d30f6	2215083
87	rename listings to posts	2026-02-11 08:43:25.015173+00	t	\\xc9348b9296bc74ec0581425dbfa93c74d14bb40e8e2e02978c8c16db435fdddf163000511b4e53515275f4d855216eb0	2213459
88	rename listing type to post type	2026-02-11 08:43:25.017692+00	t	\\x47fdda496a3413c63915d82a3f8a6f635ef7777cf239d41717d8da7446c1a04083d6d134c5f625eaca83c62f499f03b4	3469083
89	fix listing reports view	2026-02-11 08:43:25.021478+00	t	\\x778e2f7562cf2175d7561cbfb2b593c9c7ebeae1375595074afd8b4a27e4dfcc580fb04b7ccddcf827473ef578abf9e0	1614000
90	create post website sync	2026-02-11 08:43:25.02343+00	t	\\x7f9118971459d203fbd19370444a2c97d607de097cb5d739c060b43b15abd3712b7f8a8800a193ea8b73de2b9b98f016	6433417
91	add post soft delete	2026-02-11 08:43:25.03014+00	t	\\x81bf9dd1af564b4b9a8162d8d6881717840c37e4debf6da09ae162f75ad8140d1fcb1b6ee0f3c55f5e6af72ce8a53750	1427333
92	create post contacts	2026-02-11 08:43:25.031948+00	t	\\x6b8944a9b8026df84c7459d64cf4f0b822ad16f9e83c6bb7075973c4fd3192e1f145e4edc74024879d579e0547d30724	4133208
93	drop posts embedding	2026-02-11 08:43:25.03631+00	t	\\x517937750c9e524096ca77a9788559c75e5f95fabab2a8ddd82a00c7ef2e472ef816cadbcfeca09fa91a06a7fc1b0ee4	1076125
94	rename org domain to website	2026-02-11 08:43:25.037658+00	t	\\x80f7fc46e46e538fed44a7df2cdf097fa9d9b724bc002cc2c972fcd316d68dda7429ac0d4cdd72502040156127bf96ba	2440917
95	create page extractions	2026-02-11 08:43:25.040394+00	t	\\x29399fa3a6c51984b872af7629e8555c4ec835f9924326a49b9c3000681e8d27afd901b1284d4372e7007e1755dba6e6	4873250
96	add website snapshot sync tracking	2026-02-11 08:43:25.045488+00	t	\\xe9c5330e21a6de559e587d5595b3cafd74e764d44bf459c855d824cb066e3900418d5a3809a757688bc66801ce258917	1318750
97	drop unused crawler tables	2026-02-11 08:43:25.047038+00	t	\\x2fd65fcd47888afd96517932d0e53932d2689e34efffcc71b6b01d728efbf7615d5a71d6e0739fd341fd94bab88d4c69	546292
98	cleanup job status	2026-02-11 08:43:25.047783+00	t	\\xf42a8641bc1a0ba89b4f6066b43c941bcb297d57954f6a383780da028db2c3994cf09bd6b1eab3c979a7ac54243bb237	3137500
99	sync jobs table with rust	2026-02-11 08:43:25.05117+00	t	\\x44bc26aadf1015830ede2e30c4e05c4d02367d284a10da2b9e3f90cbf63abe238cc2ed902cc32dd459d9508f6a4190c3	5340375
100	fix jobs unique constraint	2026-02-11 08:43:25.056866+00	t	\\x293e954405f6616e93fe0ec6b471c57740c5807dd457543f8428c15ea69984e5ded9873da6804dc658d9d08aa6d2fbc7	1661250
101	add post embeddings	2026-02-11 08:43:25.058744+00	t	\\xb0c9e94cf053e90fc69da8e21f81403631a7421aa25c55e06d0de125e62f3d317b11a247600f6ef63146330e46f7cf1e	1125084
102	add post revisions	2026-02-11 08:43:25.060166+00	t	\\x72ec65ba7b100ac0293fe88cc904948d75e217f69878eb2309337f1d14aefcfcbe7863f5434978e4d7de9d50a2a4cc6a	1804000
103	create discovery queries	2026-02-11 08:43:25.062181+00	t	\\xdc648edcd216297bc64f0eb7c2d15a32dc6509cae80e1f3b6237ec2397bd46fb7d9a45182758ce1d9a61e28967f240a4	12454417
104	seed discovery queries	2026-02-11 08:43:25.074921+00	t	\\x66c96db56524cee4bef01a7fec692cbaae466503fe6dfd748cca31525b635a90f9de37ab1ae4c84b3c272d859fe1cb57	1058625
105	create seesaw tables	2026-02-11 08:43:25.076206+00	t	\\x2e5ee3d39f61859b14cbca4eb834ddcd271f96595417c8bbec7e892d786d2ab89bd7425c1c40669aab3b9b9e657517cb	13170125
106	create locations	2026-02-11 08:43:25.089686+00	t	\\x0b4bd9d3de62509da1763ba5cbef38e9439060ac00b245eaba27853c89dc9189c0494e268b7551bad9c76f940c46a0e3	6173125
107	create post locations schedules service areas	2026-02-11 08:43:25.096152+00	t	\\xbfe7edb795519b2fe15b559edfc2598f7613f49a8933a4cabdf7cb8f54c22e3f1f2b762bc27ccde20f5c53d4517b6692	17222375
108	create sync proposals	2026-02-11 08:43:25.113715+00	t	\\xa95c0f78ffd17351b72a344b167c33a65167c0f1725a94161dda9b3e6ac40e46bfe58cc08d8c02b177f17099a094783c	11100750
109	add tag hierarchy and external mapping	2026-02-11 08:43:25.125103+00	t	\\x1d60aeab768636a0f6fcf8ad347293dc98c8ab6de9f44b766e338494069526d5986b871385d48dd866fcb487d6e0e977	2605459
110	fix seesaw notify payload size	2026-02-11 08:43:25.127926+00	t	\\x0e281a854e5f87fcef2c2760e1b799e5a447be43f78527334aa355b59e0d63bf4cc492a79e12614a4b001892f7bbd3fc	571417
111	create agents	2026-02-11 08:43:25.128677+00	t	\\x6e9f872cc79d8a77396ab8cf737bd1b9dfbc5f025b2ba9d1b90756288542e92ac29a64c232c259cae51c65776d6488a1	3784583
112	add agent config name	2026-02-11 08:43:25.132757+00	t	\\xb6bc3c5ef3d090274eebf8e697a6bbc0f3281fa6d35018092630afd322aa1134fb0952f6b5cded0a3e01312446a520ec	1673541
113	seesaw batch join	2026-02-11 08:43:25.134663+00	t	\\xf093298be44576f05850b071aa37524ac09e00412388b3a30bd3773d6b29ec3904584c406dc83e42dfaa60b61642c61f	4552208
114	evolve schedules to calendar	2026-02-11 08:43:25.139502+00	t	\\xa87dca0d3ef8a297176dac20f0a1c565d8fcab724caad13720ba635bd4d607bb3609ae2bb1e9e2704bd0b6e366866479	1433709
115	create zip codes	2026-02-11 08:43:25.141236+00	t	\\x587bff327c45f30b8c2c82ec58e73775b62c51c147771079e6edc1e24c82c104691be9819dd9d0bcefb6b25f652d01a2	5350750
116	seed mn zip codes	2026-02-11 08:43:25.14689+00	t	\\xe5faf7261e3336b2846dcf6bdb314e0674e664769fbafeaf52d8df69b14efa3c4117c1f7d11e3fe093c13851593406fb	8717667
117	rename listing to post in polymorphic	2026-02-11 08:43:25.155907+00	t	\\x6ed9f5c38bfe112e59c368f232fb68c6441ef6cf0263dbe5c6d3215fa628ba5ca4fe7319675d7a26549ee826e2f209d6	1000541
118	drop polymorphic contacts	2026-02-11 08:43:25.157212+00	t	\\x05e8db32004f97905d92aa3f65d3e0c845bf26c0075752b4794ebbc8512fc8f094e651fd69f0c12f0d769cf1c6e54b99	1380666
119	invert container fk	2026-02-11 08:43:25.158806+00	t	\\xd391d04c22a63f75404c2fc65ec7fd0c68c5eece5b8221e906c6b69197d5c9495eec656063786c45534948be213caf57	2544958
120	restore polymorphic contacts	2026-02-11 08:43:25.161597+00	t	\\x02b2918e1825f7208da7b499dde4b9e11cd0263253b615b963885576d9e652c53d9857e0659fbbbba38cc820e12ef204	5397167
121	drop deprecated tables	2026-02-11 08:43:25.16726+00	t	\\xd0f699e69947e2b9f8d23d1ecb05e48856728df3d8bbee17fd496cd5c83ee9492d19747b1ad7444d9ad3e6d28b3fa715	4472584
122	drop organizations	2026-02-11 08:43:25.172075+00	t	\\xfd9da390a69fdf4a26de5101082773c3d7f8c2692cb80d94a2feff973603afe2dff27b0ae4603fa79f55347ec44be70b	3506209
123	add post translation of id	2026-02-11 08:43:25.175926+00	t	\\x70d8be655b5a8cfd9915d6347f41fbb8ccff0c63e2307663443ae57c71562891cb9472dd0537c746ccf00a5fdd5fa25c	2056000
124	fix stale function references	2026-02-11 08:43:25.178255+00	t	\\x4effe734fb13c8bb1de9b668f98c57b4697381a1678d6384f7f084f474786f418f05d8edfdb9242f39a3b8905aaecf4f	598584
125	create tag kinds	2026-02-11 08:43:25.179046+00	t	\\x3adf5d100efaee6d73e7d29834d9f82caf9b9d860d7e34674b21c5cb3d1fc4e26f3dcac965e827b006a2ad25a6af99e2	3451292
126	restructure agents	2026-02-11 08:43:25.182811+00	t	\\x4c1b02f93d55ff296635f9ecf4725977c0013c736921dd4bd6be478dae39379ef699d60e634dd5769387d77fea355199	6085000
127	create agent curator tables	2026-02-11 08:43:25.189143+00	t	\\x6eb061cb19a87031b0212d4f0e75eef3e4ee0d784102458e5e68b68e93133777d5a72d97fc0c8088d4473006098b720c	16411417
128	add agent id to posts	2026-02-11 08:43:25.205844+00	t	\\xdf4f5ce3f1d6aa9d81c71ef0adb7d3c8d4ba250ca79b5a256747c3366a24930915fcb7ed28b498d5ec85e7e08e3c2ba9	2134583
129	add color to tags	2026-02-11 08:43:25.20828+00	t	\\x8b4abe9023df76ccad8cb78c8e9133a50b09d7bac3a9ec483541a731d52898827d5e3972e701bf36b3018c7df485ccc3	603875
130	add description to tags and seed post types	2026-02-11 08:43:25.209102+00	t	\\x85d335b0ff7b15386c9ea6b1e9899d2398de2bb0222bab35bd8378cccfae222f3a1d04472c65adab21dffbf59c7347f7	818208
131	drop discovery tables	2026-02-11 08:43:25.210265+00	t	\\x997866ccbaf921b5ed3817a5236c884422655d0c3af0a19f3a270db81581d3cea7e965e28e2319c7c13b7d412cb03d31	3123042
132	add emoji to tags	2026-02-11 08:43:25.213723+00	t	\\x4371a93e2c3434d7b4d63b5d47225dd66078e0e37254d608469531c05480b0053136e77bac7503ea3aa7e714e933ec01	595333
133	add agent submitter type	2026-02-11 08:43:25.214524+00	t	\\x7a50198c39765e4e25eef3c916619e11d921087bddf0d806911a0cb62e502040f2aa9ed893e5b0c2bf7b6fbb9aae1461	943250
134	unify post submitted by	2026-02-11 08:43:25.215791+00	t	\\xc92dcd0f3f9d88fc915c3d35f9f4c63bab2e47c03740ddc6e0a61e92a16f7eaea0bafbaed5225d975d807ba7f58ff1ae	3003750
135	fix taggable type check constraint	2026-02-11 08:43:25.219071+00	t	\\xe8cb0c0a85fe904252b15be5678f67bcea44ef7b4de06cccbad1cf3dde8c35fb4e0b8200d86682d00674f85427be8b2b	648583
136	drop curator tables add search queries	2026-02-11 08:43:25.219898+00	t	\\xc9a8fbf33a109e7c6a1420b3817a9193c78d2a79bbe35819d1568146b2df4084c95f9405977ee548ca92f30c9041169e	5649458
137	rename tldr to summary	2026-02-11 08:43:25.225886+00	t	\\x07faf3926079cf3bbbdd75328621ddc3e12073b40180ae6999481449ff7a1d1f77f43c20ec06fc2976e6991749b7ef2e	3307625
138	drop tag taxonomy columns	2026-02-11 08:43:25.22945+00	t	\\xda07d074a7b0e38527d0f5b6b675834f5c83ea2f1c921b37901741b19c6d37aed8f38e9da6485fc4f52fbbac780a2da6	862084
139	add required to tag kinds	2026-02-11 08:43:25.230531+00	t	\\xa9ed010c7a64de96779c0dac1ddb0e08e49cbd70e695b45968ad05ba7d95fcf6fbf4d54b97fc89d770c7a3fd11542a1f	734541
140	drop website crawl tracking columns	2026-02-11 08:43:25.231472+00	t	\\x79ad841b3daace742974aa8d389cebc7a1020bd6a5d18da0e193be7b0f00189c3accaeab012423506df8148e137f8371	1369458
141	create organizations	2026-02-11 08:43:25.233127+00	t	\\xebf709a29ba0910a6d9d4e3c105f54af35e07880ce9fae163718d74c5be416fcb060307d90fbe88792bc8fec159a6ebc	3047250
142	add organization id to websites	2026-02-11 08:43:25.236387+00	t	\\xe694d7694135b720ad87355407fe4cdfc5e7065446b899c1bb361784cf5e89aac9312872e97a28c067e8ca177986a85a	1621292
143	create social profiles	2026-02-11 08:43:25.238217+00	t	\\x03f42f42e9a2c8f57d181c99251d5f4cfbd93c84058adfb07f26802478e6cb4290487d2af97ee60f1a418ede3dfa38c5	4594209
144	add social profile id to posts	2026-02-11 08:43:25.243077+00	t	\\x10e08248df36c0b389ca010124f409aea22d9907883e1074498b3cc64267b538e9a92352096960e6a39066f42959326a	1846709
145	add is public to tags	2026-02-11 08:43:25.245147+00	t	\\x5426e3194452318f73e16227a46fa1d355231ae16e28156198a6901e0500b59f41327c91d4c07fb127caf7760e7e0f14	628792
146	add org name unique and cascade deletes	2026-02-11 08:43:25.246075+00	t	\\x3990f220600013fc90dad3f9cd2e1cc75e0c76149e97272a76e81ac81314af5dab4a1059f13ae0b31dabb8db8616a7a3	2636083
147	create notes	2026-02-11 08:43:25.249072+00	t	\\xed7f7b9b1db2b81dc568ac7280d2a1100ed0251fa49715c780ac9396c441a7ffc15e8f409086b782616faada6c3dcdbf	9393834
148	create post sources	2026-02-11 08:43:25.258722+00	t	\\x3e5da5273e5281dc579a3d38f64052dc3028031de52647609e290baa7501528a2fe35302f6c45721ddbb5523af7f6dcb	10250167
149	create unified sources	2026-02-11 08:43:25.269315+00	t	\\x10b11979db30498f8bf6f919f00ad818b95042b4a612fc2878733127353618ef0b2a1f07208c46905cb38b160c823406	27885083
150	add organization approval	2026-02-11 08:43:25.297719+00	t	\\xd422eeff4cb2fac9a4e015e6c1314d7e315b915ccd3cddf1633c47850a921763e0e839a04776fa1899d0576592d924be	2724458
151	add archived post status	2026-02-11 08:43:25.300653+00	t	\\xd0b4cc2611ffeb28cbc4fed88e372b88f9a828594cd1d18a337335fc4bbfe1d56da9f034fa9268b30bde121fafabf6a6	950667
152	fix assessment embedding dimensions	2026-02-11 08:43:25.301912+00	t	\\x8764a6cadf0afda3b53df4679e6b414735be1a771453a310724cc5898fd1bb5b1c19f6e8f78f2a83408a22d7ebf2ff04	15025875
153	add note embeddings	2026-02-11 08:43:25.317359+00	t	\\x172c1739dda2c94d4200905fdcddcebb59cf8abbace94a7cef21a2bd81d96f4dc625751ccd2fb290ca4a729b1e462752	972583
154	add published at to posts	2026-02-11 08:43:25.318951+00	t	\\x08a70304cffd00b76dc6fc2e260c2420066d7b33c23d7fa599a9da4f0e1cdd26827fdb5f8ed07a5ab7d69066bfa3ab62	1068167
155	rename warn severity to urgent	2026-02-11 08:43:25.320387+00	t	\\xc270664c0ab0cf06008285d6e1b94e56404765ec1ca05235f84f8eb141c7b6cba3692c567554b87ff0a3a8db4c7a07ef	974458
156	add cta text to notes	2026-02-11 08:43:25.321577+00	t	\\x2b9d664600581f96eee9827a74fd05ee2c0bcbed0116ef007faac9af418e05ab40ab6a9a0b937ab81ef01580d14dbcc5	694000
157	add duplicate of id to posts	2026-02-11 08:43:25.3225+00	t	\\x8922cb9a1f118226423b070d675e5397ba16fe01c52f3439e1eb86f0bc28aff0f6281bdac7140fcdc97800e3fe0b554d	2493625
\.


--
-- Data for Name: active_languages; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.active_languages (language_code, language_name, native_name, enabled, added_at) FROM stdin;
en	English	English	t	2026-02-11 08:43:24.699988+00
es	Spanish	Español	t	2026-02-11 08:43:24.699988+00
so	Somali	Soomaali	t	2026-02-11 08:43:24.699988+00
\.


--
-- Data for Name: agent_assistant_configs; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.agent_assistant_configs (agent_id, preamble, config_name) FROM stdin;
8dfd01e8-1a16-452f-b84f-502fa1952981	You are an admin assistant for MN Together, a resource-sharing platform.\nYou can help administrators:\n- Approve or reject listings\n- Scrape websites for new resources\n- Generate website assessments\n- Search and filter listings\n- Manage organizations\n\nBe helpful and proactive. If an admin asks to do something, use the appropriate tool.	admin
\.


--
-- Data for Name: agents; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.agents (id, member_id, display_name, created_at, updated_at, role, status) FROM stdin;
8dfd01e8-1a16-452f-b84f-502fa1952981	4bd4c43b-6332-457e-92c8-7ff13bade627	MN Together Assistant	2026-02-11 08:43:25.128677+00	2026-02-11 08:43:25.128677+00	assistant	active
\.


--
-- Data for Name: business_listings; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.business_listings (listing_id, business_type, support_needed, current_situation, accepts_donations, donation_link, gift_cards_available, gift_card_link, remote_ok, delivery_available, online_ordering_link) FROM stdin;
\.


--
-- Data for Name: contacts; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.contacts (id, contactable_type, contactable_id, contact_type, contact_value, contact_label, is_public, display_order, created_at) FROM stdin;
\.


--
-- Data for Name: containers; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.containers (id, language, created_at, last_activity_at, tags) FROM stdin;
\.


--
-- Data for Name: document_references; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.document_references (id, document_id, reference_kind, reference_id, referenced_at, display_order) FROM stdin;
\.


--
-- Data for Name: extraction_embeddings; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.extraction_embeddings (url, site_url, embedding) FROM stdin;
\.


--
-- Data for Name: extraction_pages; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.extraction_pages (url, site_url, content, content_hash, fetched_at, title, http_headers, metadata) FROM stdin;
\.


--
-- Data for Name: extraction_summaries; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.extraction_summaries (url, site_url, text, signals, language, created_at, prompt_hash, content_hash) FROM stdin;
\.


--
-- Data for Name: identifiers; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.identifiers (id, member_id, phone_hash, is_admin, created_at, updated_at) FROM stdin;
b0a272d0-6664-47c9-b280-302cd700e0e4	9d39612e-4391-41ad-b7d2-5fab97be32dd	422ce82c6fc1724ac878042f7d055653ab5e983d186e616826a72d4384b68af8	t	2026-02-11 08:43:25.128677+00	\N
\.


--
-- Data for Name: jobs; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.jobs (id, status, job_type, args, next_run_at, last_run_at, max_retries, retry_count, version, idempotency_key, reference_id, priority, error_message, error_kind, created_at, updated_at, frequency, timezone, overlap_policy, misfire_policy, timeout_ms, lease_duration_ms, lease_expires_at, worker_id, enabled, container_id, workflow_id, dead_lettered_at, dead_letter_reason, replay_count, resolved_at, resolution_note, root_job_id, dedupe_key, attempt, command_version) FROM stdin;
\.


--
-- Data for Name: listing_contacts; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.listing_contacts (id, listing_id, contact_type, contact_value, contact_label, display_order) FROM stdin;
1a8d4df1-05fb-4c85-8a02-4b94a28ee941	6fc40dc5-cb6b-4293-9574-fa0b427c8792	website	https://iglesiavina.org/	\N	0
6244643d-9228-4793-ac2f-44d48d83db5c	d04d41ea-33ca-4a12-9f92-02dd532f87d3	website	https://iglesiavina.org/	\N	0
af2acbfe-6604-4fc9-b339-3c08fe978597	1b6a8e74-da96-4b7b-87c3-5f20ac45544e	phone	612-615-9294	\N	0
a2f624c3-1a29-4947-8233-b3dd5d6d2c2b	1b6a8e74-da96-4b7b-87c3-5f20ac45544e	email	miguel@smvineyard.org	\N	0
2a4ab414-bc2c-4021-9d9b-9eeee610af8d	1b6a8e74-da96-4b7b-87c3-5f20ac45544e	website	https://iglesiavina.org	\N	0
90bffd34-692f-47cf-9696-058b04fe5c10	a088ec91-8f8a-4890-90af-b1136d4af7cb	email	communityaidnetworkmn@gmail.com	\N	0
66796b23-257a-42f9-a5c4-48471e494987	a088ec91-8f8a-4890-90af-b1136d4af7cb	website	https://www.canmn.org/	\N	0
a9937709-4356-4874-8c16-b3386bf9e23a	3a2bd62d-b30f-473a-8086-43459cc55a63	website	https://www.dhhmn.com/operations	\N	0
834ed33b-f0ee-40ab-a44c-6dff0f753dd9	63d565c8-3d71-4995-bae1-4ee6d2b8954f	website	https://www.dhhmn.com/operations	\N	0
23db925e-c61b-4fbf-bd89-fafe37a02244	e3907820-9b49-4501-b7d0-3af6bdf11510	email	communityaidnetworkmn@gmail.com	\N	0
8991b666-3b5c-4d4f-8905-083946e75ba3	6c822dc7-817c-4efa-84a4-43061c75df13	email	communityaidnetworkmn@gmail.com	\N	0
f10b4dac-a837-49dd-b561-e17483f5d437	6c822dc7-817c-4efa-84a4-43061c75df13	website	https://canmn.org/donate-supplies	\N	0
c5c239b1-3b44-4201-9a98-565c7f303188	02c26b95-c99c-4bfb-8654-6fb5e9f83e2c	website	https://canmn.org/volunteer	\N	0
323dfed8-23a4-42ae-9949-9765fdd222ff	2205c422-dcec-491a-936f-72b3eafae128	email	communityaidnetworkmn@gmail.com	\N	0
b0800c5c-1032-4b8e-9d19-d3b0007a0713	2205c422-dcec-491a-936f-72b3eafae128	website	https://canmn.org/receive	\N	0
088450ba-c092-492b-9571-332442c1479e	96f79ce7-3d4f-44c9-bf7c-58b40e4c6f6e	phone	651-641-1011	\N	0
5b0b4854-234c-447e-adcd-ff93d4144035	693f22d8-653c-410c-985c-bc42a25f0b72	phone	(612) 874-8605	\N	0
25ad8c46-068b-45bc-8dde-b9c7d76e5d0a	693f22d8-653c-410c-985c-bc42a25f0b72	website	https://mnchurches.org/what-we-do/refugee-services/immigration-legal-services	\N	0
31e93faa-2c74-4154-9f60-69d4874abc44	eb0a0ce0-67c4-4c1f-832f-f85061c51c24	phone	1-833-870-4111	\N	0
3503f680-8794-47c6-a3a9-12f45307f18a	eb0a0ce0-67c4-4c1f-832f-f85061c51c24	email	tell@mpr.org	\N	0
dc2a7ba6-5562-44da-81d9-ebf2dbb97db3	0bca8293-5a39-431c-b004-7a7bb81c7e48	phone	651-290-1212	\N	0
88c09051-fc9e-47dd-b47c-acb5f9084a49	0bca8293-5a39-431c-b004-7a7bb81c7e48	email	tell@mpr.org	\N	0
d08185ee-069a-4fa3-aa03-d475c10758ef	0bca8293-5a39-431c-b004-7a7bb81c7e48	website	https://www.mprnews.org/story/2026/02/07/indigenousled-organizations-serve-ice-symbolic-eviction-notice	\N	0
a9b7adcb-8431-4533-99a0-873eea4e18ba	d62719b9-2903-48cb-a350-2522b5873f2b	phone	612-341-3302	\N	0
ef94da2a-3ac0-4689-bb5c-e2cd7d108cd8	d62719b9-2903-48cb-a350-2522b5873f2b	email	hrights@advrights.org	\N	0
c35964ba-179b-4479-91ee-839b88fdafc7	1fd84bad-7621-4d1b-a044-16e4331145cb	phone	612-341-3302	\N	0
e164c1e2-b075-43b7-b61e-0f4aa0417411	1fd84bad-7621-4d1b-a044-16e4331145cb	email	hrights@advrights.org	\N	0
130a31bf-eb80-4d3b-a2bb-e0d24f9513cc	1fd84bad-7621-4d1b-a044-16e4331145cb	website	https://www.theadvocatesforhumanrights.org/News/A/Index?id=595	\N	0
53558fcc-ed28-4a8c-81d3-e1ae866b84f7	c56152bc-ce43-4fe5-8cb3-78ca19ce62b3	email	volunteer@advrights.org	\N	0
94fdf782-e59b-4261-9c17-2949b7a602c5	c56152bc-ce43-4fe5-8cb3-78ca19ce62b3	website	https://www.theadvocatesforhumanrights.org/Volunteer/Immigration_Court	\N	0
0ea9d7c0-f032-4d89-9519-e79c8d41155b	af2d26d5-43a5-4259-a361-0e5e787671d1	website	https://www.iceoutnowmn.com/wearemn	\N	0
3c36339d-d3f8-424d-80c9-ba511fa2e7e5	ebc43799-9f5e-4a8a-8801-ecf51aa965c8	website	https://sahanjournal.com/immigration/minneapolis-ice-observers-mothers-cedar-riverside-protection-alliance/	\N	0
\.


--
-- Data for Name: listing_delivery_modes; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.listing_delivery_modes (listing_id, delivery_mode) FROM stdin;
\.


--
-- Data for Name: listing_reports; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.listing_reports (id, listing_id, reported_by, reporter_email, reason, category, status, resolved_by, resolved_at, resolution_notes, action_taken, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: locations; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.locations (id, name, address_line_1, address_line_2, city, state, postal_code, latitude, longitude, location_type, accessibility_notes, transportation_notes, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: members; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.members (id, expo_push_token, searchable_text, active, notification_count_this_week, paused_until, created_at, latitude, longitude, location_name, embedding) FROM stdin;
4bd4c43b-6332-457e-92c8-7ff13bade627	agent:default	AI Admin Assistant	t	0	\N	2026-02-11 08:43:25.128677+00	\N	\N	\N	\N
9d39612e-4391-41ad-b7d2-5fab97be32dd	test:admin	Test Admin User	t	0	\N	2026-02-11 08:43:25.128677+00	44.98	-93.27	Minneapolis, MN	\N
\.


--
-- Data for Name: messages; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.messages (id, container_id, role, content, created_at, sequence_number, author_id, moderation_status, parent_message_id, updated_at, edited_at) FROM stdin;
\.


--
-- Data for Name: migration_workflows; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.migration_workflows (id, name, phase, total_items, completed_items, failed_items, skipped_items, last_processed_id, dry_run, error_budget, started_at, paused_at, completed_at, created_at) FROM stdin;
\.


--
-- Data for Name: noteables; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.noteables (id, note_id, noteable_type, noteable_id, added_at) FROM stdin;
\.


--
-- Data for Name: notes; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.notes (id, content, severity, source_url, source_id, source_type, is_public, created_by, expired_at, created_at, updated_at, embedding, cta_text) FROM stdin;
\.


--
-- Data for Name: opportunity_listings; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.opportunity_listings (listing_id, opportunity_type, time_commitment, requires_background_check, minimum_age, skills_needed, remote_ok) FROM stdin;
\.


--
-- Data for Name: organization_tags; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.organization_tags (id, organization_id, kind, value, created_at) FROM stdin;
\.


--
-- Data for Name: organizations; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.organizations (id, name, description, created_at, updated_at, status, submitted_by, submitter_type, submission_context, reviewed_by, reviewed_at, rejection_reason) FROM stdin;
c5b70ffe-1864-48d9-8383-c3d24d3e1e09	Iglesia Cristiana La Viña Burnsville	Faith-based community organization in Burnsville providing food assistance, community programs, and volunteer opportunities.	2026-02-11 09:20:04.420441+00	2026-02-11 09:20:04.420441+00	approved	\N	\N	\N	\N	\N	\N
0e6a7133-44de-47ee-a07b-c6093fc6e14d	Community Aid Network	Community organization providing food distribution, supply donations, and volunteer coordination across the Twin Cities.	2026-02-11 09:20:04.420441+00	2026-02-11 09:20:04.420441+00	approved	\N	\N	\N	\N	\N	\N
ff2e040e-2b5b-4a92-ae86-eab00e95d801	Dios Habla Hoy (DHH Church)	Faith-based community providing food packing, delivery services, and food box registration for families in need.	2026-02-11 09:20:04.420441+00	2026-02-11 09:20:04.420441+00	approved	\N	\N	\N	\N	\N	\N
\.


--
-- Data for Name: page_extractions; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.page_extractions (id, page_snapshot_id, extraction_type, content, model, prompt_version, tokens_used, created_at, is_current) FROM stdin;
\.


--
-- Data for Name: page_snapshots; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.page_snapshots (id, url, content_hash, html, markdown, fetched_via, metadata, crawled_at, listings_extracted_count, extraction_completed_at, extraction_status) FROM stdin;
\.


--
-- Data for Name: page_summaries; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.page_summaries (id, page_snapshot_id, content_hash, content, created_at) FROM stdin;
\.


--
-- Data for Name: post_locations; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.post_locations (id, post_id, location_id, is_primary, notes, created_at) FROM stdin;
\.


--
-- Data for Name: post_page_sources; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.post_page_sources (post_id, page_snapshot_id, created_at) FROM stdin;
\.


--
-- Data for Name: post_sources; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.post_sources (id, post_id, source_type, source_id, source_url, first_seen_at, last_seen_at, disappeared_at, created_at, updated_at) FROM stdin;
6a07dffa-183e-4fe9-93c9-49548058e5e5	3b5f0866-8d96-445d-bc7f-c461225dd38a	website	03fe0602-0582-4696-9d1b-8eb4d236bdb2	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00
822f9d53-b5b7-41bc-afce-f1eab0eec161	881c452a-213a-4f94-9155-a77840415a3a	website	03fe0602-0582-4696-9d1b-8eb4d236bdb2	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00
235c69c9-5450-441b-806d-24a7cc502e42	3d7eaee8-4d7d-45fc-ac82-5733647ee8db	website	03fe0602-0582-4696-9d1b-8eb4d236bdb2	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00
1ebcf6ff-ebb5-4519-bab3-a4b1c2a0b3d5	6fc40dc5-cb6b-4293-9574-fa0b427c8792	website	03fe0602-0582-4696-9d1b-8eb4d236bdb2	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00
a12afeab-925f-4f97-8e79-ddfb3473eb5b	d04d41ea-33ca-4a12-9f92-02dd532f87d3	website	03fe0602-0582-4696-9d1b-8eb4d236bdb2	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00
5b2c3eec-4481-4c2a-a2af-9c8114047e8a	1b6a8e74-da96-4b7b-87c3-5f20ac45544e	website	03fe0602-0582-4696-9d1b-8eb4d236bdb2	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00
d90c71e3-488e-47b5-a841-54a237d47703	a088ec91-8f8a-4890-90af-b1136d4af7cb	website	39285d8d-d16a-4ef4-9f0d-922b8d0f42cf	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00
04735492-d5e1-4c57-b555-7caae93333f1	e3907820-9b49-4501-b7d0-3af6bdf11510	website	39285d8d-d16a-4ef4-9f0d-922b8d0f42cf	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00
408b8aa8-e807-420e-9a81-a4dc37b74b78	df2a20b0-7fd7-4da2-b8eb-83a9b4d34d86	website	39285d8d-d16a-4ef4-9f0d-922b8d0f42cf	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00
a7abe9ec-8c2f-4679-b625-593ad5fd7660	6c822dc7-817c-4efa-84a4-43061c75df13	website	39285d8d-d16a-4ef4-9f0d-922b8d0f42cf	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00
3fa4dfc2-9e05-40af-8699-1b13e19ab0ec	02c26b95-c99c-4bfb-8654-6fb5e9f83e2c	website	39285d8d-d16a-4ef4-9f0d-922b8d0f42cf	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00
f07ff410-2f08-4af0-be15-3380a37628b3	2205c422-dcec-491a-936f-72b3eafae128	website	39285d8d-d16a-4ef4-9f0d-922b8d0f42cf	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00
3e99772d-8418-45ae-b311-cd1606df4ad1	0c388522-9556-41cd-9c5d-e59624184ea7	website	aec4d22d-e117-4e99-8b57-544752bd24bc	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00
7270fca6-0c80-4bba-a570-089912ef6f22	3a2bd62d-b30f-473a-8086-43459cc55a63	website	aec4d22d-e117-4e99-8b57-544752bd24bc	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00
3324d780-328d-4388-a88f-a8cbf6e93f29	63d565c8-3d71-4995-bae1-4ee6d2b8954f	website	aec4d22d-e117-4e99-8b57-544752bd24bc	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00	\N	2026-02-11 09:20:26.49313+00	2026-02-11 09:20:26.49313+00
\.


--
-- Data for Name: posts; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.posts (id, title, description, description_markdown, summary, urgency, status, created_at, updated_at, submission_type, submitted_by_member_id, location, submitted_from_ip, fingerprint, extraction_confidence, latitude, longitude, source_url, post_type, category, capacity_status, verified_at, source_language, page_snapshot_id, disappeared_at, deleted_at, deleted_reason, embedding, revision_of_post_id, comments_container_id, translation_of_id, submitted_by_id, published_at, duplicate_of_id) FROM stdin;
3b5f0866-8d96-445d-bc7f-c461225dd38a	Donate Food or Funds to Support Families	**Donate Food or Funds to Support Families**  \nHelp us provide essential food and supplies to families in crisis. We need donations of specific food items and financial contributions to support rent assistance and food purchases.  \n\n**Donation Drop-off:**  \n- **Location:** 13798 Parkwood Drive, Burnsville, MN 55337  \n- **Days & Hours:** Mondays & Tuesdays: 12 PM – 7 PM, Fridays: 12 PM – 5 PM, Saturdays: 10 AM – 4 PM  \n\n**Items Needed:**  \n- Proteins, dairy, grains, fruits, vegetables, household essentials  \n\n**Donate Online:** [give.tithe.ly](https://give.tithe.ly/?formId=9adcb3ba-202c-4e6e-b045-bbb8704b7164)  \n\nYour generosity helps prevent evictions and feeds families in need.	\N	Donate food or funds to support families in crisis. Drop off items at 13798 Parkwood Drive, Burnsville, MN, or donate online at give.tithe.ly.	medium	active	2026-02-11 04:01:00.937986+00	2026-02-11 09:02:09.498995+00	admin	\N	13798 Parkwood Drive, Burnsville, MN 55337	\N	\N	\N	\N	\N	https://www.instagram.com/p/DUCBvuLgPkn/	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
881c452a-213a-4f94-9155-a77840415a3a	Volunteer to Pack and Deliver Food	**Volunteer to Pack and Deliver Food**  \nJoin us in supporting families in crisis by volunteering to pack food orders, load vehicles, and deliver food to homes. We urgently need more volunteers this Monday and Tuesday, February 9 and 10, from 12:00 PM to 7:00 PM.  \n\n**Volunteer Schedule:**  \n- **Days:** Mondays, Tuesdays, Fridays, Saturdays  \n- **Hours:** Mondays & Tuesdays: 12 PM – 7 PM, Fridays: 12 PM – 5 PM, Saturdays: 10 AM – 4 PM  \n\n**How to Volunteer:**  \n- **Sign Up:** [volunteer.lavinaburnsville.org](https://volunteer.lavinaburnsville.org)  \n- **Requirements:** Photo ID for delivery volunteers  \n\nYour help brings hope and food to our community. Your presence and compassion make a significant impact. Join us in serving the community.	\N	Volunteer to pack and deliver food to families in need. Urgent help needed this Monday and Tuesday. Sign up at volunteer.lavinaburnsville.org.	high	active	2026-02-11 04:01:00.87434+00	2026-02-11 09:02:09.498995+00	admin	\N	\N	\N	\N	\N	\N	\N	https://www.instagram.com/p/DUCBvuLgPkn/,https://www.instagram.com/p/DUhn0UcDXt2/	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
3d7eaee8-4d7d-45fc-ac82-5733647ee8db	Register for Free Home Food Deliveries	**Free Home Food Deliveries**  \nIf you or someone you know is struggling and unable to leave home due to the crisis, La Viña Burnsville offers free food deliveries directly to your doorstep. If you are in crisis and have run out of food, you can place another order immediately without waiting a week.  \n\n**How to Register:**  \n- **Registration Form:** [request.lavinaburnsville.org](https://request.lavinaburnsville.org)  \n- **Eligibility:** Open to families in need who cannot leave their homes  \n\nWe are here to serve you with love and dignity. Please share this information with anyone who might need it to ensure no family goes without help.	\N	Register for free home food deliveries if you're unable to leave home due to the crisis. Sign up at request.lavinaburnsville.org to receive support. Immediate orders available for those in crisis.	high	active	2026-02-11 04:01:00.820096+00	2026-02-11 09:02:09.498995+00	admin	\N	\N	\N	\N	\N	\N	\N	https://www.instagram.com/p/DUjiZBtDeo0/,https://www.instagram.com/p/DUlHl_NAHWw/	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
6fc40dc5-cb6b-4293-9574-fa0b427c8792	Donate to Support Community Programs	**Donate to Support Community Programs**\n\n- **How to Donate:** [Donation Link](https://give.tithe.ly/?formId=9adcb3ba-202c-4e6e-b045-bbb8704b7164)\n\nSupport the mission of helping the community and fostering hope by donating funds. Your contributions help sustain various programs and services.\n\n**Contact:** Visit the website or call for more information.\n\n**Source:** [Iglesia Vina Home](https://iglesiavina.org/)	\N	Donate to support community programs at La Viña. Your contributions help sustain various services and initiatives. Donate online today.	medium	active	2026-02-11 04:00:25.58961+00	2026-02-11 09:02:09.498995+00	admin	\N	\N	\N	\N	\N	\N	\N	https://iglesiavina.org/	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
d04d41ea-33ca-4a12-9f92-02dd532f87d3	Volunteer at La Viña	**Volunteer at La Viña**\n\n- **How to Sign Up:** [Volunteer Registration](https://volunteer.lavinaburnsville.org/)\n\nJoin the mission to help the community and foster hope by volunteering your time and skills. Sign up online to get involved.\n\n**Contact:** Visit the website or call for more information.\n\n**Source:** [Iglesia Vina Home](https://iglesiavina.org/)	\N	Sign up to volunteer at La Viña and help the community. Register online to get involved and make a difference.	medium	active	2026-02-11 04:00:25.500588+00	2026-02-11 09:02:09.498995+00	admin	\N	13798 Parkwood Dr, Burnsville, MN 55337, USA	\N	\N	\N	\N	\N	https://iglesiavina.org/	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
1b6a8e74-da96-4b7b-87c3-5f20ac45544e	Access Free Food at the Community Pantry	**Despensa Comunitaria (Food Pantry)**\n\n- **When:** Mondays from 4:00 PM to 6:30 PM\n- **Location:** 13798 Parkwood Dr, Burnsville, MN 55337\n- **Services:** Free food distribution for families in need\n- **Home Delivery:** Available on Mondays, Tuesdays, and Saturdays for families in crisis\n\nOur community pantry is open to everyone. Families can visit in person to receive food supplies. For those unable to visit, we offer home delivery services. \n\n**Contact:** Visit the pantry during open hours or call for more information.\n\n**Source:** [Iglesia Vina Services](https://iglesiavina.org/services%2Fservicios)	\N	Visit the community pantry on Mondays from 4:00 PM to 6:30 PM at 13798 Parkwood Dr, Burnsville, MN. Free food is available for all families, with home delivery options on select days.	medium	active	2026-02-11 04:00:24.860604+00	2026-02-11 09:02:09.498995+00	admin	\N	13798 Parkwood Dr, Burnsville, MN 55337	\N	\N	\N	\N	\N	https://iglesiavina.org/services%2Fservicios	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
a088ec91-8f8a-4890-90af-b1136d4af7cb	Donate Visa Gift Cards and Winter Essentials	**Donate Visa Gift Cards and Winter Essentials**\n\nHelp @communityaidnetworkmn by donating Visa gift cards, hats, gloves, and socks. \n\n- **Biggest Need**: Visa gift cards ($25 and $50)\n- **When**: This weekend\n\nYour donations will support community members in need during challenging times. Every contribution helps provide essential resources to those affected by the ongoing situation in Minneapolis.	\N	Donate Visa gift cards, hats, gloves, and socks to support @communityaidnetworkmn this weekend. Visa gift cards ($25 and $50) are the biggest need. Help provide essential resources to those in need.	medium	active	2026-02-10 22:44:43.908974+00	2026-02-11 09:02:09.498995+00	admin	\N	2400 3rd Ave South, Minneapolis, MN	\N	\N	\N	\N	\N	https://www.instagram.com/p/DUWXeMcDhNf/	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
0c388522-9556-41cd-9c5d-e59624184ea7	Donate Food or Funds to Support Families	**Donate Food or Funds to Support Families**\n\nSupport DHH Operation by donating money or select physical items to help provide consistent, nourishing food to families in need. Monetary donations allow the purchase of essential groceries and supplies.\n\n- **Donation Drop-Off:**\n  - **Location:** 5728 Cedar Ave S, Minneapolis, MN 55417\n  - **Times:** Tuesday & Thursday, 10:00 AM – 4:00 PM\n\n**Items Needed:**\n- Diapers, baby wipes, infant formula\n- Toothpaste, toothbrushes, deodorant\n- Laundry detergent sheets, trash bags\n\nDonate online via [Zeffy](https://www.zeffy.com/en-US/donation-form/despensas-food-drive) or drop off items during specified hours. Your support makes a big impact!	\N	Donate money or select items to support DHH Operation. Drop off items at 5728 Cedar Ave S, Minneapolis, on Tuesdays and Thursdays, 10 AM - 4 PM. Monetary donations can be made online via Zeffy.	medium	active	2026-02-09 22:50:53.460236+00	2026-02-11 09:02:09.498995+00	admin	\N	5728 Cedar Ave S, Minneapolis, MN 55417	\N	\N	\N	\N	\N	https://dhhmn.com/operations	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
3a2bd62d-b30f-473a-8086-43459cc55a63	Volunteer for Food Packing and Delivery	**Volunteer for Food Packing and Delivery**\n\nJoin DHH Operation as a volunteer to help pack and deliver food boxes to families in need. Volunteers are needed at The Food Group in New Hope and Manna Market in Spring Lake Park.\n\n- **Schedule:** Monday to Friday, 9:30 AM - 12:30 PM\n- **Locations:** The Food Group, New Hope & Manna Market, Spring Lake Park\n- **Roles:** Receiving, unloading, sorting donations, and deliveries\n\n**Requirements:**\n- Minimum age 18+\n- Valid license & insurance for drivers\n- US Citizens or Legal Residents only\n\nSign up in person or contact [DHH Operation](https://www.dhhmn.com/operations) for more details.	\N	Help pack and deliver food boxes at The Food Group and Manna Market. Volunteer shifts are Monday to Friday, 9:30 AM - 12:30 PM. Must be 18+, with a valid license for drivers. Sign up in person.	medium	active	2026-02-09 22:50:53.430323+00	2026-02-11 09:02:09.498995+00	admin	\N	\N	\N	\N	\N	\N	\N	https://dhhmn.com/operations	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
63d565c8-3d71-4995-bae1-4ee6d2b8954f	Register for Free Food Boxes	**Receive Free Food Boxes**\n\nIf you are in need of food assistance, you can register to receive a food box from DHH Operation. This service is available to families in the Twin Cities area who are facing financial strain or are unable to safely access food. \n\n- **Location:** 5728 Cedar Ave S, Minneapolis, MN 55417\n- **Registration:** Required online via [this link](https://dhhmn.elvanto.net/form/aaee1106-c325-4f3f-b193-bb6fc0118f62/)\n- **Contact:** Keep your phone on for delivery coordination\n\n**Eligibility:**\n- Families must register online to receive food\n- No ID required for food donations or volunteering registration\n\nFor more information, visit the [DHH Operation page](https://www.dhhmn.com/operations).	\N	Register online to receive a free food box from DHH Operation. Available to families in the Twin Cities area. No ID required. Keep your phone on for delivery coordination. Register via the provided link.	medium	active	2026-02-09 22:50:53.375858+00	2026-02-11 09:02:09.498995+00	admin	\N	5728 Cedar Ave S, Minneapolis, MN 55417	\N	\N	\N	\N	\N	https://dhhmn.com/operations	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
e3907820-9b49-4501-b7d0-3af6bdf11510	Get Free Food and Supplies Every Thursday	**Get Free Food and Supplies**  \nVisit Bethel Lutheran Church at 4120 17th Ave S, Minneapolis, MN 55407 for free food and supplies every Thursday from 5 PM to 7 PM. No identification or membership is required. Simply show up during distribution hours to receive what you need.  \n\n**Free Food Delivery**  \nIf you live within the designated delivery area, you can have food and supplies delivered to your home. Deliveries occur on Thursdays from 5-7 PM and Saturdays from 12-2 PM. To place an order, email [communityaidnetworkmn@gmail.com](mailto:communityaidnetworkmn@gmail.com).	\N	Pick up free food and supplies at Bethel Lutheran Church on Thursdays 5-7 PM. No ID required. For delivery, email to schedule on Thursdays or Saturdays.	medium	active	2026-02-09 22:46:33.929409+00	2026-02-11 09:02:09.498995+00	admin	\N	Bethel Lutheran Church, 4120 17th Ave S, Minneapolis, MN 55407	\N	\N	\N	\N	\N	https://canmn.org/receive	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
df2a20b0-7fd7-4da2-b8eb-83a9b4d34d86	Volunteer for Food Distribution and Delivery	**Volunteer On-Site**  \nHelp with food and supply distribution at Bethel Lutheran Church, 4120 17th Ave S, Minneapolis, MN 55407. Volunteer shifts are available for food intake every other Monday from 11 AM to 2 PM, community distribution on Thursdays from 4:30 PM to 7 PM, and delivery on Saturdays from 12 PM to 1:30 PM. New volunteers should arrive 10 minutes early for orientation.  \n\n**Volunteer as a Delivery Driver**  \nAssist with delivering supplies to community members on Thursdays and Saturdays. You must have your own vehicle and current auto insurance. Contact CANMN if you have previously volunteered for delivery work to sign up.	\N	Volunteer on-site or as a delivery driver at Bethel Lutheran Church. Shifts available on Mondays, Thursdays, and Saturdays. Sign up online.	medium	active	2026-02-09 22:46:33.887782+00	2026-02-11 09:02:09.498995+00	admin	\N	Bethel Lutheran Church, 4120 17th Ave S, Minneapolis, MN 55407	\N	\N	\N	\N	\N	https://canmn.org/volunteer	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
6c822dc7-817c-4efa-84a4-43061c75df13	Donate Essential Supplies	**What:** Donate supplies to help keep CANMN's inventory stocked.\n\n**Priority Items:**\n- Diapers (Sizes 1-7, T2-T4)\n- Baby wipes\n- Liquid dish soap\n- Laundry detergent pods\n- Shampoo, conditioner\n- Toilet paper, flour, cooking oil, pads\n\n**How to Donate:**\n- Email [communityaidnetworkmn@gmail.com](mailto:communityaidnetworkmn@gmail.com) to coordinate a drop-off time.\n- Drop-off hours: Every other Monday 10:30 AM - 1 PM, Thursday 3:30 - 7 PM, Saturday 10:30 AM - 12:30 PM\n\n**Location:** Bethel Lutheran Church, 4120 17th Ave S, Minneapolis, MN 55407\n\n**Source:** [CANMN Donate Supplies](https://canmn.org/donate-supplies)	\N	Contribute supplies to support community needs.	medium	active	2026-02-09 22:08:02.181256+00	2026-02-11 09:02:09.498995+00	admin	\N	Bethel Lutheran Church, 4120 17th Ave S, Minneapolis, MN 55407	\N	\N	\N	\N	\N	https://canmn.org/donate-supplies	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
02c26b95-c99c-4bfb-8654-6fb5e9f83e2c	Volunteer for On-Site Distribution	**What:** Volunteer to help with on-site food and supplies distribution.\n\n**Where:** Bethel Lutheran Church, 4120 17th Ave S, Minneapolis, MN 55407\n\n**When:**\n- Food intake every other Monday 11 AM - 2 PM\n- Community distribution on Thursdays 4:30 PM - 7 PM\n- Delivery on Saturdays 12 PM - 1:30 PM\n\n**Requirements:**\n- Arrive 10 minutes early for orientation if it's your first time.\n- Follow COVID-19 safety procedures.\n\n**How to Sign Up:** [Sign-up for a Volunteer Shift](https://calendly.com/communityaidnetworkmn-1)\n\n**Source:** [CANMN Volunteer](https://canmn.org/volunteer)	\N	Help distribute food and supplies on-site.	medium	active	2026-02-09 22:08:02.050899+00	2026-02-11 09:02:09.498995+00	admin	\N	Bethel Lutheran Church, 4120 17th Ave S, Minneapolis, MN 55407	\N	\N	\N	\N	\N	https://canmn.org/volunteer	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
2205c422-dcec-491a-936f-72b3eafae128	Pick Up Free Food and Supplies	**What:** Free food and supplies distribution for anyone in need.\n\n**Where:** Bethel Lutheran Church, 4120 17th Ave S, Minneapolis, MN 55407\n\n**When:** Thursdays from 5 PM to 7 PM\n\n**Eligibility:** No identification or membership required. Open to everyone.\n\n**Contact:** Just show up during distribution hours to receive supplies.\n\n**Source:** [CANMN Receive Aid](https://canmn.org/receive)	\N	Get free food and supplies every Thursday evening.	medium	active	2026-02-09 22:08:01.865653+00	2026-02-11 09:02:09.498995+00	admin	\N	Bethel Lutheran Church, 4120 17th Ave S, Minneapolis, MN 55407	\N	\N	\N	\N	\N	https://canmn.org/receive	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
96f79ce7-3d4f-44c9-bf7c-58b40e4c6f6e	Get Help if Detained by ICE	If you or someone you know is detained by Immigration and Customs Enforcement (ICE), call **651-641-1011** during the following hours for assistance:\n\n- **Monday-Thursday:** 1 – 3 p.m.\n\nThis service provides general information about court proceedings, the Minnesota Detention Project, and services available at a first court hearing. Note that volunteers operating the phone line are not attorneys and cannot provide legal advice.	\N	Call for assistance if detained by ICE.	medium	active	2026-02-09 05:46:12.177581+00	2026-02-11 09:02:09.498995+00	admin	\N	\N	\N	\N	\N	\N	\N	https://ilcm.org/immigration-help	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
98b6a300-c1f9-4256-b6e7-d2acd13c0e5e	Join ICE Out MN Community Briefings	Join regular community briefings to stay updated on ICE activity in Minnesota, hear from organizers and leaders, and learn how you can take meaningful action. These briefings are open to everyone who wants trusted updates, next steps, and ways to support impacted neighbors.\n\n- **When:** Next call, Sunday Feb 8th, 3pm (special time); Ongoing calls continue every Sunday at 8pm starting Feb 15th.\n- **Where:** Virtual (Zoom)\n- **Why attend?**\n  - Stay informed with reliable, regularly updated information.\n  - Get action steps and ways to support community safety and advocacy.\n  - Connect with organizers, leaders, and neighbors working together across the state.\n\n[Register Here](https://bit.ly/mnbriefing)	\N	Stay informed and take action with weekly briefings.	medium	active	2026-02-09 05:22:17.848901+00	2026-02-11 09:02:09.498995+00	admin	\N	\N	\N	\N	\N	\N	\N	https://iceoutnowmn.com	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
693f22d8-653c-410c-985c-bc42a25f0b72	Access Immigration Legal Services	MCC Refugee Services offers **Immigration Legal Services** to assist refugees, asylees, and eligible immigrants in Minnesota. Services include:\n\n- **Adjustment of Status** (Form I-485) for refugees, asylees, and eligible Cubans\n- **US Citizenship Applications** (Form N-400)\n- **Work Permits** (Form I-765) for eligible populations\n\n**Family Reunification Options**:\n- **Affidavit of Relationship (AOR)** for eligible refugees and asylees\n- **Lautenberg Program** for religious minorities from former Soviet Union countries\n- **Central American Minor (CAM)** for at-risk children in El Salvador, Guatemala, and Honduras\n- **Petition for Alien Relative (Form I-130)** for permanent residents and US citizens (fee-based)\n\n**Walk-In Hours**: Every Thursday, 9:00am - Noon (Minneapolis location)\n- **Note**: Walk-ins are for case intake, consultation, and follow-up for MCC cases only. No new applications or follow-ups for cases filed elsewhere.\n\n**Contact**: Call (612) 874-8605 for assistance.\n\n**Locations**: Twin Cities and Mankato\n\n**Eligibility**: Services are for refugees, asylees, and eligible immigrant populations.\n\nFor more information, visit the [source page](https://mnchurches.org/what-we-do/refugee-services/immigration-legal-services).	\N	Get immigration help for refugees and asylees in Minnesota.	medium	active	2026-02-09 05:32:34.973493+00	2026-02-11 09:02:09.498995+00	admin	\N	Twin Cities and Mankato	\N	\N	\N	\N	\N	https://mnchurches.org/what-we-do/refugee-services/immigration-legal-services	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
eb0a0ce0-67c4-4c1f-832f-f85061c51c24	Join the Rally Against Immigration Enforcement at Target HQ	**What:** Advocates are organizing a rally outside Target headquarters in Minneapolis.\n\n**Purpose:** To call on Target to speak out against immigration enforcement operations in Minnesota.\n\n**Location:** Target headquarters, Minneapolis.\n\n**Date and Time:** The rally took place on February 2, 2026.\n\n**Source:** [Advocates rally outside Target headquarters](https://www.mprnews.org/episode/2026/02/02/minnesotatodaypm)	\N	Rally at Target HQ against immigration enforcement.	low	active	2026-02-09 05:26:35.740628+00	2026-02-11 09:02:09.498995+00	admin	\N	Target headquarters, Minneapolis	\N	\N	\N	\N	\N	https://www.mprnews.org/episode/2026/02/02/minnesotatodaypm	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
0bca8293-5a39-431c-b004-7a7bb81c7e48	Join Indigenous Demonstration Against ICE	Join Indigenous activists at the Henry Whipple Federal Building for a demonstration against ICE operations. This event highlights the historical significance of the site, where Dakota families were once imprisoned, and the irony of its current use. The building is named after a man who advocated for Dakota rights during the U.S. Dakota war.\n\n- **Location:** Henry Whipple Federal Building\n- **Date:** Saturday\n- **Purpose:** Serve a symbolic 'eviction notice' to ICE\n\nFor more details, visit the [event page](https://www.mprnews.org/story/2026/02/07/indigenousled-organizations-serve-ice-symbolic-eviction-notice).	\N	Participate in a protest against ICE at a historic site.	medium	active	2026-02-09 05:26:35.561827+00	2026-02-11 09:02:09.498995+00	admin	\N	Henry Whipple Federal Building, Minneapolis	\N	\N	\N	\N	\N	https://www.mprnews.org/story/2026/02/07/indigenousled-organizations-serve-ice-symbolic-eviction-notice	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
d62719b9-2903-48cb-a350-2522b5873f2b	Save Asylum - Urgent Action Needed	Act urgently to save asylum and protect the rights of those seeking refuge. **Date:** Friday, December 8, 2023.\n\n**How to Participate:**\n- Visit the [action page](https://www.theadvocatesforhumanrights.org/News/URGENT_Take_Action_to_Save_Asylum) to learn more and take action.\n\n**Contact Information:**\n- **Phone:** 612-341-3302\n- **Email:** [hrights@advrights.org](mailto:hrights@advrights.org)\n\n**Location:**\n- The Advocates for Human Rights\n- 330 Second Avenue South, Suite 800, Minneapolis, MN 55401	\N	Urgent action to save asylum rights.	high	active	2026-02-09 05:23:52.253847+00	2026-02-11 09:02:09.498995+00	admin	\N	330 Second Avenue South, Suite 800, Minneapolis, MN 55401	\N	\N	\N	\N	\N	https://theadvocatesforhumanrights.org/Migrant_Rights/Advocacy	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
1fd84bad-7621-4d1b-a044-16e4331145cb	Demand Due Process in Immigration	Join the call to ensure the immigration system provides due process and does not facilitate coercion and abuse. **Date:** Monday, August 18, 2025.\n\n**How to Participate:**\n- Visit the [action page](https://www.theadvocatesforhumanrights.org/News/A/Index?id=595) to learn more and take action.\n\n**Contact Information:**\n- **Phone:** 612-341-3302\n- **Email:** [hrights@advrights.org](mailto:hrights@advrights.org)\n\n**Location:**\n- The Advocates for Human Rights\n- 330 Second Avenue South, Suite 800, Minneapolis, MN 55401	\N	Ensure due process in immigration system.	medium	active	2026-02-09 05:23:52.196729+00	2026-02-11 09:02:09.498995+00	admin	\N	330 Second Avenue South, Suite 800, Minneapolis, MN 55401	\N	\N	\N	\N	\N	https://theadvocatesforhumanrights.org/Migrant_Rights/Advocacy	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
c56152bc-ce43-4fe5-8cb3-78ca19ce62b3	Monitor Immigration Court Proceedings	**Monitor Immigration Court Proceedings**\n\nBring a public eye to immigration court proceedings by volunteering as a court monitor. This role helps ensure transparency and accountability in the legal process.\n\n- **Location:** 330 Second Avenue South, Suite 800, Minneapolis, MN 55401\n- **Contact:** [volunteer@advrights.org](mailto:volunteer@advrights.org)\n\n[Learn More](https://www.theadvocatesforhumanrights.org/Volunteer/Immigration_Court)	\N	Observe and report on immigration court cases.	medium	active	2026-02-09 05:23:52.077913+00	2026-02-11 09:02:09.498995+00	admin	\N	330 Second Avenue South, Suite 800, Minneapolis, MN 55401	\N	\N	\N	\N	\N	https://theadvocatesforhumanrights.org/Volunteer	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
af2d26d5-43a5-4259-a361-0e5e787671d1	Participate in 'Shop Local for Truth & Freedom' Week	Participate in a week-long event to support local businesses and push for corporate accountability.\n\n- **When:** Monday February 9 - Saturday, February 14\n- **Why:**\n  - Uplift small businesses and entrepreneurs.\n  - Redirect economic activity toward local communities.\n  - Push major corporations to take a public stand.\n  - Increase pressure on Congress.\n\n### How to Participate\n\n**For Small Businesses:**\n- Sign on to the [Congressional business letter](https://docs.google.com/forms/d/e/1FAIpQLSdJzKbu1gk9ORWJoHS7L77q6DCJLM9_he96oYuilHd2J3bhaQ/viewform?usp=header)\n- Display a [Shop Local sign](https://www.iceoutnowmn.com/wearemn)\n- Share your story publicly\n- Host mutual aid efforts\n\n**For Consumers:**\n- Sign the [public pledge](https://secure.ngpvan.com/iHxXf5VLo0uzXMmT78-QWQ2)\n- Redirect spending to local businesses\n- Support mutual aid efforts\n\n**For Associations & Organizations:**\n- Share business lists\n- Promote the campaign\n- Sponsor a day during the week\n\n**Week-at-a-Glance: Commercial Corridors**\n- Monday, Feb 9: Payne-Phalen / East St. Paul\n- Tuesday, Feb 10: Cedar-Riverside / Grand Avenue & Rice St.\n- Wednesday, Feb 11: Lake Street/ University Avenue\n- Thursday, Feb 12: Uptown & Whittier / Brooklyn Center & Brooklyn Park\n- Friday, Feb 13: North Minneapolis\n- Saturday, Feb 14: Bring your Valentine to your neighborhood business!	\N	Support local businesses and push for corporate accountability.	medium	active	2026-02-09 05:22:17.868156+00	2026-02-11 09:02:09.498995+00	admin	\N	\N	\N	\N	\N	\N	\N	https://iceoutnowmn.com/shop-local-for-truth-freedom	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
ebc43799-9f5e-4a8a-8801-ecf51aa965c8	Join Mothers Protecting Neighbors from ICE	Join a group of mothers in the Cedar Riverside neighborhood of Minneapolis who are actively working to protect their neighbors from ICE operations. This grassroots initiative is focused on community safety and support.\n\n**Location:** Cedar Riverside, Minneapolis\n\n**How to Get Involved:**\n- **Contact:** Reach out through the [Sahan Journal](https://sahanjournal.com/immigration/minneapolis-ice-observers-mothers-cedar-riverside-protection-alliance/) for more information on how to participate.\n\n**Eligibility:** Open to community members interested in supporting immigrant neighbors.\n\n**Source:** [Sahan Journal](https://sahanjournal.com/immigration/minneapolis-ice-observers-mothers-cedar-riverside-protection-alliance/)	\N	Mothers in Cedar Riverside protect neighbors from ICE.	medium	active	2026-02-09 05:20:47.367323+00	2026-02-11 09:02:09.498995+00	admin	\N	Cedar Riverside, Minneapolis	\N	\N	\N	\N	\N	https://standwithminnesota.com/stay-informed/sahan-in-minneapolis-cedar-riverside-a-group-of-mothers-protect-neighbors-from-ice	opportunity	general	accepting	\N	en	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
\.


--
-- Data for Name: providers; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.providers (id, name, bio, why_statement, headline, profile_image_url, member_id, source_id, location, latitude, longitude, service_radius_km, offers_in_person, offers_remote, accepting_clients, status, submitted_by, reviewed_by, reviewed_at, rejection_reason, embedding, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: referral_document_translations; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.referral_document_translations (id, document_id, language_code, content, title, translated_at, translation_model) FROM stdin;
\.


--
-- Data for Name: referral_documents; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.referral_documents (id, container_id, source_language, content, slug, title, status, edit_token, view_count, last_viewed_at, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: schedules; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.schedules (id, schedulable_type, schedulable_id, day_of_week, opens_at, closes_at, timezone, valid_from, valid_to, notes, created_at, dtstart, dtend, rrule, exdates, is_all_day, duration_minutes, updated_at) FROM stdin;
dcb595bb-dfbf-443f-aa5a-6248d657749a	post	3b5f0866-8d96-445d-bc7f-c461225dd38a	1	12:00:00	19:00:00	America/Chicago	\N	\N	\N	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=MO	\N	f	\N	2026-02-11 09:02:09.498995+00
b49a34ed-a3a3-4d77-9c62-fe29b78efc38	post	3b5f0866-8d96-445d-bc7f-c461225dd38a	2	12:00:00	19:00:00	America/Chicago	\N	\N	\N	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=TU	\N	f	\N	2026-02-11 09:02:09.498995+00
1e8c364c-005b-4a48-80e1-69501148a692	post	3b5f0866-8d96-445d-bc7f-c461225dd38a	5	12:00:00	17:00:00	America/Chicago	\N	\N	\N	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=FR	\N	f	\N	2026-02-11 09:02:09.498995+00
d8d6f471-22ac-4d9c-b473-0db006f37bf3	post	3b5f0866-8d96-445d-bc7f-c461225dd38a	6	10:00:00	16:00:00	America/Chicago	\N	\N	\N	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=SA	\N	f	\N	2026-02-11 09:02:09.498995+00
9a0c13f7-352b-4448-92ce-e9706516bb2a	post	881c452a-213a-4f94-9155-a77840415a3a	1	12:00:00	19:00:00	America/Chicago	\N	\N	\N	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=MO	\N	f	\N	2026-02-11 09:02:09.498995+00
73ea0be4-4891-4edf-96e7-8cae86650567	post	881c452a-213a-4f94-9155-a77840415a3a	2	12:00:00	19:00:00	America/Chicago	\N	\N	\N	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=TU	\N	f	\N	2026-02-11 09:02:09.498995+00
a2d62fc8-95ed-4497-9fb7-e7c1ece6fbd7	post	881c452a-213a-4f94-9155-a77840415a3a	5	12:00:00	17:00:00	America/Chicago	\N	\N	\N	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=FR	\N	f	\N	2026-02-11 09:02:09.498995+00
16861d19-cd3d-46d9-8eeb-1879be09bcfa	post	881c452a-213a-4f94-9155-a77840415a3a	6	10:00:00	16:00:00	America/Chicago	\N	\N	\N	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=SA	\N	f	\N	2026-02-11 09:02:09.498995+00
73be6c77-aeb6-403c-b58c-ef26a1d9f66e	post	1b6a8e74-da96-4b7b-87c3-5f20ac45544e	1	16:00:00	18:30:00	America/Chicago	\N	\N	\N	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=MO	\N	f	\N	2026-02-11 09:02:09.498995+00
878d0bb7-d573-473b-a486-ad6ed9c9443c	post	0c388522-9556-41cd-9c5d-e59624184ea7	2	10:00:00	16:00:00	America/Chicago	\N	\N	\N	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=TU	\N	f	\N	2026-02-11 09:02:09.498995+00
e155c370-4f1a-4679-a4c0-07b629877444	post	0c388522-9556-41cd-9c5d-e59624184ea7	4	10:00:00	16:00:00	America/Chicago	\N	\N	\N	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=TH	\N	f	\N	2026-02-11 09:02:09.498995+00
12ceb7b3-d7c4-43f7-9d02-d815ede83e84	post	e3907820-9b49-4501-b7d0-3af6bdf11510	4	17:00:00	19:00:00	America/Chicago	\N	\N	In-person pickup	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=TH	\N	f	\N	2026-02-11 09:02:09.498995+00
166c026a-c011-40a9-9137-2f9555bf7b37	post	e3907820-9b49-4501-b7d0-3af6bdf11510	4	17:00:00	19:00:00	America/Chicago	\N	\N	Delivery	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=TH	\N	f	\N	2026-02-11 09:02:09.498995+00
132ac051-775a-4e2d-8ef1-39f1b6b3d173	post	e3907820-9b49-4501-b7d0-3af6bdf11510	6	12:00:00	14:00:00	America/Chicago	\N	\N	Delivery	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=SA	\N	f	\N	2026-02-11 09:02:09.498995+00
59a999ca-5c97-4285-8a7e-74d4eef3e9ad	post	df2a20b0-7fd7-4da2-b8eb-83a9b4d34d86	1	11:00:00	14:00:00	America/Chicago	\N	\N	Volunteers should arrive 10 minutes early for orientation	2026-02-11 09:02:09.498995+00	2026-02-09 22:46:33.923966+00	\N	FREQ=WEEKLY;INTERVAL=2;BYDAY=MO	\N	f	180	2026-02-11 09:02:09.498995+00
b9fafc43-76ea-4052-861a-11768a9aaae5	post	df2a20b0-7fd7-4da2-b8eb-83a9b4d34d86	4	16:30:00	19:00:00	America/Chicago	\N	\N	Volunteers should arrive 10 minutes early for orientation	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=TH	\N	f	\N	2026-02-11 09:02:09.498995+00
f51aa944-508c-44cd-980a-2ba4325e8f05	post	df2a20b0-7fd7-4da2-b8eb-83a9b4d34d86	6	12:00:00	13:30:00	America/Chicago	\N	\N	Volunteers should arrive 10 minutes early for orientation	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=SA	\N	f	\N	2026-02-11 09:02:09.498995+00
36bc33ef-f09e-490c-98e1-0acd08ae29ee	post	6c822dc7-817c-4efa-84a4-43061c75df13	1	10:30:00	13:00:00	America/Chicago	\N	\N	Every other Monday	2026-02-11 09:02:09.498995+00	2026-02-09 22:08:02.253409+00	\N	FREQ=WEEKLY;INTERVAL=2;BYDAY=MO	\N	f	150	2026-02-11 09:02:09.498995+00
2193a146-a4b2-44ce-ad19-aa629608a830	post	6c822dc7-817c-4efa-84a4-43061c75df13	4	15:30:00	19:00:00	America/Chicago	\N	\N	\N	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=TH	\N	f	\N	2026-02-11 09:02:09.498995+00
1740d28d-3943-4fe6-bf77-8bab8710c7ed	post	6c822dc7-817c-4efa-84a4-43061c75df13	6	10:30:00	12:30:00	America/Chicago	\N	\N	\N	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=SA	\N	f	\N	2026-02-11 09:02:09.498995+00
1b1869a0-e8dc-4837-99c8-fa5c2e59efbe	post	02c26b95-c99c-4bfb-8654-6fb5e9f83e2c	1	11:00:00	14:00:00	America/Chicago	\N	\N	Food intake every other Monday	2026-02-11 09:02:09.498995+00	2026-02-09 22:08:02.118041+00	\N	FREQ=WEEKLY;INTERVAL=2;BYDAY=MO	\N	f	180	2026-02-11 09:02:09.498995+00
45f63de2-859b-4fbf-8c83-2b19a67f861e	post	02c26b95-c99c-4bfb-8654-6fb5e9f83e2c	4	16:30:00	19:00:00	America/Chicago	\N	\N	Community distribution	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=TH	\N	f	\N	2026-02-11 09:02:09.498995+00
012ccb3c-b8cc-4245-90b6-9b85c9ae96f0	post	02c26b95-c99c-4bfb-8654-6fb5e9f83e2c	6	12:00:00	13:30:00	America/Chicago	\N	\N	Delivery	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=SA	\N	f	\N	2026-02-11 09:02:09.498995+00
64a79a2a-8a69-4447-97f2-a715320f7570	post	2205c422-dcec-491a-936f-72b3eafae128	4	17:00:00	19:00:00	America/Chicago	\N	\N	In-person distribution	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=TH	\N	f	\N	2026-02-11 09:02:09.498995+00
07b3a763-031d-411c-b5dc-d24950a0996f	post	2205c422-dcec-491a-936f-72b3eafae128	4	17:00:00	19:00:00	America/Chicago	\N	\N	Delivery	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=TH	\N	f	\N	2026-02-11 09:02:09.498995+00
6ccb3696-4619-47c5-869b-6a2527853e18	post	2205c422-dcec-491a-936f-72b3eafae128	6	12:00:00	14:00:00	America/Chicago	\N	\N	Delivery	2026-02-11 09:02:09.498995+00	\N	\N	FREQ=WEEKLY;BYDAY=SA	\N	f	\N	2026-02-11 09:02:09.498995+00
\.


--
-- Data for Name: scrape_jobs; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.scrape_jobs (id, source_id, status, error_message, scraped_at, extracted_at, synced_at, new_needs_count, changed_needs_count, disappeared_needs_count, created_at, updated_at, completed_at) FROM stdin;
\.


--
-- Data for Name: search_queries; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.search_queries (id, query_text, is_active, sort_order, created_at) FROM stdin;
\.


--
-- Data for Name: seesaw_dlq; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.seesaw_dlq (id, event_id, effect_id, correlation_id, error, event_type, event_payload, reason, attempts, created_at) FROM stdin;
\.


--
-- Data for Name: seesaw_effect_executions; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.seesaw_effect_executions (event_id, effect_id, correlation_id, status, event_type, event_payload, parent_event_id, execute_at, timeout_seconds, max_attempts, priority, attempts, result, error, claimed_at, last_attempted_at, completed_at, created_at, batch_id, batch_index, batch_size) FROM stdin;
\.


--
-- Data for Name: seesaw_events_default; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.seesaw_events_default (id, event_id, parent_id, correlation_id, event_type, payload, hops, retry_count, locked_until, processed_at, created_at, batch_id, batch_index, batch_size) FROM stdin;
\.


--
-- Data for Name: seesaw_join_entries; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.seesaw_join_entries (join_effect_id, correlation_id, source_event_id, source_event_type, source_payload, source_created_at, batch_id, batch_index, batch_size, created_at) FROM stdin;
\.


--
-- Data for Name: seesaw_join_windows; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.seesaw_join_windows (join_effect_id, correlation_id, mode, batch_id, target_count, status, sealed_at, processing_started_at, completed_at, last_error, updated_at, created_at) FROM stdin;
\.


--
-- Data for Name: seesaw_processed; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.seesaw_processed (event_id, correlation_id, created_at) FROM stdin;
\.


--
-- Data for Name: seesaw_state; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.seesaw_state (correlation_id, state, version, updated_at) FROM stdin;
\.


--
-- Data for Name: service_listings; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.service_listings (listing_id, requires_identification, remote_available, requires_appointment, walk_ins_accepted, in_person_available, home_visits_available, wheelchair_accessible, interpretation_available, free_service, sliding_scale_fees, accepts_insurance, evening_hours, weekend_hours) FROM stdin;
\.


--
-- Data for Name: social_sources; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.social_sources (id, source_id, source_type, handle) FROM stdin;
\.


--
-- Data for Name: sources; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.sources (id, source_type, url, organization_id, status, active, scrape_frequency_hours, last_scraped_at, submitted_by, submitter_type, submission_context, reviewed_by, reviewed_at, rejection_reason, created_at, updated_at) FROM stdin;
03fe0602-0582-4696-9d1b-8eb4d236bdb2	website	https://www.instagram.com/lavinaburnsville/	c5b70ffe-1864-48d9-8383-c3d24d3e1e09	approved	t	24	\N	\N	\N	\N	\N	\N	\N	2026-02-11 09:20:04.420441+00	2026-02-11 09:20:04.420441+00
39285d8d-d16a-4ef4-9f0d-922b8d0f42cf	website	https://communityaidnetwork.org/	0e6a7133-44de-47ee-a07b-c6093fc6e14d	approved	t	24	\N	\N	\N	\N	\N	\N	\N	2026-02-11 09:20:04.420441+00	2026-02-11 09:20:04.420441+00
aec4d22d-e117-4e99-8b57-544752bd24bc	website	https://www.dioshablahoychurch.org/	ff2e040e-2b5b-4a92-ae86-eab00e95d801	approved	t	24	\N	\N	\N	\N	\N	\N	\N	2026-02-11 09:20:04.420441+00	2026-02-11 09:20:04.420441+00
\.


--
-- Data for Name: sync_batches; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.sync_batches (id, resource_type, source_id, status, summary, proposal_count, approved_count, rejected_count, created_at, reviewed_at, expires_at) FROM stdin;
\.


--
-- Data for Name: sync_proposal_merge_sources; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.sync_proposal_merge_sources (id, proposal_id, source_entity_id) FROM stdin;
\.


--
-- Data for Name: sync_proposals; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.sync_proposals (id, batch_id, operation, status, entity_type, draft_entity_id, target_entity_id, reason, reviewed_by, reviewed_at, created_at) FROM stdin;
\.


--
-- Data for Name: tag_kinds; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.tag_kinds (id, slug, display_name, description, allowed_resource_types, created_at, required, is_public) FROM stdin;
3bb6d5da-be26-46dc-825f-23cb6e4c9b5d	population	Population	Target populations served (e.g., seniors, refugees, youth)	{post,website,provider}	2026-02-11 08:43:25.179046+00	f	f
5b3d8884-daff-4609-84aa-71d1c4b81165	community_served	Community Served	Cultural communities served (e.g., Somali, Hmong, Latino)	{post,website,provider}	2026-02-11 08:43:25.179046+00	f	f
f8f79827-b11e-4f8e-af2a-8d1453e771be	service_offered	Service Offered	Types of services offered (e.g., legal aid, food assistance)	{post,website}	2026-02-11 08:43:25.179046+00	f	f
8f7b23d0-e542-4698-85b3-cccf26a4c149	org_leadership	Organization Leadership	Leadership identity (e.g., immigrant-owned, woman-owned)	{post,website}	2026-02-11 08:43:25.179046+00	f	f
0b3140b4-48a7-4f1b-8802-9dee6861304e	business_model	Business Model	Business structure (e.g., nonprofit, social enterprise)	{post,website}	2026-02-11 08:43:25.179046+00	f	f
9c082247-8205-45bb-8b00-8164b0cfa469	service_area	Service Area	Geographic areas served (e.g., Twin Cities, statewide)	{post,website,provider}	2026-02-11 08:43:25.179046+00	f	f
4321800e-1b01-4ae8-95bd-6d2583e15cbe	provider_category	Provider Category	Provider role type (e.g., therapist, wellness coach)	{provider}	2026-02-11 08:43:25.179046+00	f	f
47ac9a2d-b135-4d0f-ae29-0d4104c3a012	provider_specialty	Provider Specialty	Provider specialization areas (e.g., grief, anxiety)	{provider}	2026-02-11 08:43:25.179046+00	f	f
1608e18e-259b-4f30-9d04-cb865017567b	provider_language	Provider Language	Languages spoken by provider	{provider}	2026-02-11 08:43:25.179046+00	f	f
4eae8f3b-a457-4b71-aaf7-f0b8aee83a05	service_language	Service Language	Languages offered by a service	{post,website}	2026-02-11 08:43:25.179046+00	f	f
09816d75-c56f-44f7-bc8d-c3458a76227f	verification_source	Verification Source	Source of verification for organizations	{website}	2026-02-11 08:43:25.179046+00	f	f
c9118f3f-60e0-4652-bdc7-0ff340b0fa38	with_agent	With Agent	AI agent configuration for containers	{container}	2026-02-11 08:43:25.179046+00	f	f
ea8f5513-a286-44f5-a715-f5a1fc4c0b55	audience_role	Audience Role	Who is this resource for (e.g., recipient, donor, volunteer)	{post,website}	2026-02-11 08:43:25.179046+00	t	f
2020fd11-1090-4e5b-ac2b-e6390db83768	post_type	Post Type	Classification of post (e.g., service, business, event)	{post}	2026-02-11 08:43:25.179046+00	t	f
43fadef1-1226-4862-8b88-7d1c11074723	public	Public Tags	User-visible tags shown on public post listings	{post}	2026-02-11 09:33:49.122465+00	f	t
\.


--
-- Data for Name: taggables; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.taggables (id, tag_id, taggable_type, taggable_id, added_at) FROM stdin;
96f1c09e-8e54-4de4-9212-55382bab9b9b	6a557476-66b9-4b6b-87e0-c677b8aec3f9	post	3b5f0866-8d96-445d-bc7f-c461225dd38a	2026-02-11 09:02:09.498995+00
8bbed3e3-1f5e-4e23-a069-85da8e2ee33d	81da7cfa-49f5-44b6-9b5d-7104862d0a84	post	881c452a-213a-4f94-9155-a77840415a3a	2026-02-11 09:02:09.498995+00
e0e25a0a-d9c5-4d4e-a41f-fa2571d02c54	7e621a60-fa28-4599-a9cd-e9f6f0103c05	post	3d7eaee8-4d7d-45fc-ac82-5733647ee8db	2026-02-11 09:02:09.498995+00
45f7a481-bf18-470f-9be2-99fcab62914f	13d5edbb-8b6a-434f-80d6-45dbeafcd473	post	3d7eaee8-4d7d-45fc-ac82-5733647ee8db	2026-02-11 09:02:09.498995+00
263858a2-c69b-44b8-bcd1-19f859512793	6a557476-66b9-4b6b-87e0-c677b8aec3f9	post	a088ec91-8f8a-4890-90af-b1136d4af7cb	2026-02-11 09:02:09.498995+00
14ce2acc-3832-44d6-a6af-12e90dbe5516	a8c2a6fd-c5c7-43b1-ba01-a7113cfe2b65	post	3b5f0866-8d96-445d-bc7f-c461225dd38a	2026-02-11 09:37:55.176408+00
1c6bc6aa-2309-4dd0-9530-54971b7485c3	a8c2a6fd-c5c7-43b1-ba01-a7113cfe2b65	post	881c452a-213a-4f94-9155-a77840415a3a	2026-02-11 09:37:55.176408+00
119d3faa-f047-4eac-abe7-aff4edb27d7f	a8c2a6fd-c5c7-43b1-ba01-a7113cfe2b65	post	3d7eaee8-4d7d-45fc-ac82-5733647ee8db	2026-02-11 09:37:55.176408+00
3452831b-08e5-4ad0-b5a9-978c5f1e0e00	a8c2a6fd-c5c7-43b1-ba01-a7113cfe2b65	post	6fc40dc5-cb6b-4293-9574-fa0b427c8792	2026-02-11 09:37:55.176408+00
7c1d080a-e7e0-4671-85ab-b835f64dd7c0	a8c2a6fd-c5c7-43b1-ba01-a7113cfe2b65	post	d04d41ea-33ca-4a12-9f92-02dd532f87d3	2026-02-11 09:37:55.176408+00
4cda7127-5e23-4e10-a156-392be2cecdc2	a8c2a6fd-c5c7-43b1-ba01-a7113cfe2b65	post	1b6a8e74-da96-4b7b-87c3-5f20ac45544e	2026-02-11 09:37:55.176408+00
bc48f716-0030-4ddc-a7b4-c0b29793e322	a8c2a6fd-c5c7-43b1-ba01-a7113cfe2b65	post	a088ec91-8f8a-4890-90af-b1136d4af7cb	2026-02-11 09:37:55.176408+00
5f70dfa3-3ff6-4610-99b6-573675016c64	a8c2a6fd-c5c7-43b1-ba01-a7113cfe2b65	post	0c388522-9556-41cd-9c5d-e59624184ea7	2026-02-11 09:37:55.176408+00
b9cb0421-ac2f-4789-accb-14e94cac859b	a8c2a6fd-c5c7-43b1-ba01-a7113cfe2b65	post	3a2bd62d-b30f-473a-8086-43459cc55a63	2026-02-11 09:37:55.176408+00
bc47518f-976a-47f4-ba57-bf3804bdec56	a8c2a6fd-c5c7-43b1-ba01-a7113cfe2b65	post	e3907820-9b49-4501-b7d0-3af6bdf11510	2026-02-11 09:37:55.176408+00
223ae34c-19bf-420f-920e-49bf84481037	a8c2a6fd-c5c7-43b1-ba01-a7113cfe2b65	post	df2a20b0-7fd7-4da2-b8eb-83a9b4d34d86	2026-02-11 09:37:55.176408+00
3486d762-64a2-4a45-9558-896e80190cb5	a8c2a6fd-c5c7-43b1-ba01-a7113cfe2b65	post	6c822dc7-817c-4efa-84a4-43061c75df13	2026-02-11 09:37:55.176408+00
a0fdeee7-1575-4eff-a896-1c3c0b535d7c	a8c2a6fd-c5c7-43b1-ba01-a7113cfe2b65	post	02c26b95-c99c-4bfb-8654-6fb5e9f83e2c	2026-02-11 09:37:55.176408+00
26a5a8b5-dc47-42a6-a143-021b7d56fecb	a8c2a6fd-c5c7-43b1-ba01-a7113cfe2b65	post	2205c422-dcec-491a-936f-72b3eafae128	2026-02-11 09:37:55.176408+00
eb9834eb-a526-470d-ac98-8d1a3d2fb916	c2e7fd59-be12-4004-8349-7159fb8247fa	post	63d565c8-3d71-4995-bae1-4ee6d2b8954f	2026-02-11 09:37:55.176408+00
400e0ef0-40d4-4355-b801-cc54643a2e7f	c2e7fd59-be12-4004-8349-7159fb8247fa	post	96f79ce7-3d4f-44c9-bf7c-58b40e4c6f6e	2026-02-11 09:37:55.176408+00
64ab88b4-b06a-425f-a940-f5cde31c338c	676ff3d6-be1b-4dc9-8db9-692f7a93f7c7	post	3b5f0866-8d96-445d-bc7f-c461225dd38a	2026-02-11 09:37:55.176408+00
02c3c8fa-1f47-4428-9a44-55782a6b5a55	676ff3d6-be1b-4dc9-8db9-692f7a93f7c7	post	3d7eaee8-4d7d-45fc-ac82-5733647ee8db	2026-02-11 09:37:55.176408+00
f965d273-c9cc-4e43-b741-732b3f7cddce	676ff3d6-be1b-4dc9-8db9-692f7a93f7c7	post	1b6a8e74-da96-4b7b-87c3-5f20ac45544e	2026-02-11 09:37:55.176408+00
6552bc2f-528c-4c5f-a9f9-cd2fa9f21ffd	676ff3d6-be1b-4dc9-8db9-692f7a93f7c7	post	3a2bd62d-b30f-473a-8086-43459cc55a63	2026-02-11 09:37:55.176408+00
8c249337-4805-474d-9bc3-3d0dbd00727f	676ff3d6-be1b-4dc9-8db9-692f7a93f7c7	post	63d565c8-3d71-4995-bae1-4ee6d2b8954f	2026-02-11 09:37:55.176408+00
500c8a7f-d36e-448f-9de5-48adadcd4c58	676ff3d6-be1b-4dc9-8db9-692f7a93f7c7	post	e3907820-9b49-4501-b7d0-3af6bdf11510	2026-02-11 09:37:55.176408+00
1b0f36cd-bd99-40d4-a2f1-a34900b9ba91	676ff3d6-be1b-4dc9-8db9-692f7a93f7c7	post	df2a20b0-7fd7-4da2-b8eb-83a9b4d34d86	2026-02-11 09:37:55.176408+00
c7662cc4-e8df-4060-9262-9a306b048ee6	676ff3d6-be1b-4dc9-8db9-692f7a93f7c7	post	6c822dc7-817c-4efa-84a4-43061c75df13	2026-02-11 09:37:55.176408+00
070456bc-42df-4f44-9572-2334652b67aa	676ff3d6-be1b-4dc9-8db9-692f7a93f7c7	post	02c26b95-c99c-4bfb-8654-6fb5e9f83e2c	2026-02-11 09:37:55.176408+00
bd2e7b7e-37ea-4060-acd5-6ad5aadf7131	676ff3d6-be1b-4dc9-8db9-692f7a93f7c7	post	2205c422-dcec-491a-936f-72b3eafae128	2026-02-11 09:37:55.176408+00
fc920e24-d606-4002-b0fb-a66773621623	032af949-ff64-4785-848a-0e3047e9678b	post	0c388522-9556-41cd-9c5d-e59624184ea7	2026-02-11 09:37:55.176408+00
0b72fb9a-9a2d-4e6a-9146-63ed585495e3	935ab389-f2d7-4a3a-8a85-6b344787e49b	post	96f79ce7-3d4f-44c9-bf7c-58b40e4c6f6e	2026-02-11 09:37:55.176408+00
\.


--
-- Data for Name: tags; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.tags (id, kind, value, display_name, created_at, parent_tag_id, color, description, emoji) FROM stdin;
fb96511a-6a69-4d16-812f-ad69b1987a8e	community_served	somali	Somali	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
cc31cf33-85ab-40ec-ac55-b0b5d52ed6b3	community_served	ethiopian	Ethiopian	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
7f595d04-4281-43ad-91b5-bb6545c89288	community_served	latino	Latino	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
52973b40-5ba4-4e90-852c-4350e8c3d0f1	community_served	hmong	Hmong	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
8fff6b4e-91d2-4f91-bc05-5d5808fba2c5	community_served	karen	Karen	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
128da887-b1be-426a-8cbc-20f9dd85271c	community_served	oromo	Oromo	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
4ca9c544-6f00-4c41-ae38-7497973d1f62	service_area	minneapolis	Minneapolis	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
a6137585-081f-470b-a976-180ac388e656	service_area	st_paul	St. Paul	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
bcf08295-8d69-46ec-bad5-c858381a9c6e	service_area	bloomington	Bloomington	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
6416bb21-d3c7-4487-af2c-1d0aa4baf855	service_area	brooklyn_park	Brooklyn Park	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
41599348-ae7c-4f90-bd8b-46e5f65780e2	service_area	statewide	Statewide	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
b05763bd-93cb-4ac0-a1ef-e80fb6a1d7cd	population	seniors	Seniors	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
4748b0f8-8805-4eab-a43d-2ee5d9e3c55c	population	youth	Youth	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
9d1b064e-889a-4302-8d0d-2e1ea594cb28	population	families	Families with Children	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
be0eaa7f-e804-47c9-8215-97243d852fde	population	veterans	Veterans	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
ee07bffe-69f4-44ae-8174-c270ecf92506	population	lgbtq	LGBTQ+	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
660ca06c-dcf3-4d1a-bf05-f15e6347971e	org_leadership	community_led	Community-Led	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
c5e55866-3792-4420-ac81-54333224bbd3	org_leadership	immigrant_founded	Immigrant-Founded	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
17328de1-c747-405f-b717-715c06eb6bea	org_leadership	bipoc_led	BIPOC-Led	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
e57e94f6-9000-40ca-86aa-23e3950c7c96	verification_source	admin_verified	Admin Verified	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
cb5107fa-e5ec-49a2-951d-8eae0ff52ce7	verification_source	community_vouched	Community Vouched	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
546aa875-9fe1-41a4-9d2b-7610acc5e432	verification_source	self_reported	Self-Reported	2026-02-11 08:43:24.689446+00	\N	\N	\N	\N
7ca19de0-186f-46a4-9d05-26a65132d889	safety	no_id_required	No ID Required	2026-02-11 08:43:24.732482+00	\N	\N	\N	\N
f19afe18-1692-4754-bb0f-6f74aa8930c6	safety	no_authority_contact	Does Not Contact Authorities	2026-02-11 08:43:24.732482+00	\N	\N	\N	\N
6a485ff8-ba0a-41e3-9211-84dd92055d9c	safety	ice_safe	ICE Safe	2026-02-11 08:43:24.732482+00	\N	\N	\N	\N
70b3ad50-7e3d-43cf-aa16-7dac3978b3d3	safety	community_based	Community-Based	2026-02-11 08:43:24.732482+00	\N	\N	\N	\N
7a7695e3-3926-4e96-86c4-ce23dbaa5e42	safety	confidential	Confidential Service	2026-02-11 08:43:24.732482+00	\N	\N	\N	\N
04a6f287-75ba-4c73-ac6c-6a215f18e55c	safety	anonymous_ok	Anonymous Service Available	2026-02-11 08:43:24.732482+00	\N	\N	\N	\N
c6b57656-17b4-4ef3-9fd0-b2181792f1c2	safety	no_status_check	No Immigration Status Check	2026-02-11 08:43:24.732482+00	\N	\N	\N	\N
c4244064-bda7-40eb-8e45-abceeee9e3e1	safety	know_your_rights	Know Your Rights Info Provided	2026-02-11 08:43:24.732482+00	\N	\N	\N	\N
ce29936b-a287-4036-bcd1-1f95bb75b68c	ownership	minority_owned	Minority-Owned	2026-02-11 08:43:24.767497+00	\N	\N	\N	\N
77ce36d4-754f-4844-ab6b-2ee59efb9d89	ownership	women_owned	Women-Owned	2026-02-11 08:43:24.767497+00	\N	\N	\N	\N
9a842282-6d63-4279-9838-0faf21efe21b	ownership	lgbtq_owned	LGBTQ+-Owned	2026-02-11 08:43:24.767497+00	\N	\N	\N	\N
3c2d09bb-6399-435a-bdbe-628400f6400e	ownership	veteran_owned	Veteran-Owned	2026-02-11 08:43:24.767497+00	\N	\N	\N	\N
f36288b4-fdc9-4581-8fd9-f3b16ce9017f	ownership	immigrant_owned	Immigrant-Owned	2026-02-11 08:43:24.767497+00	\N	\N	\N	\N
add36f5f-fb80-4166-ba8e-4630299fafe3	ownership	bipoc_owned	BIPOC-Owned	2026-02-11 08:43:24.767497+00	\N	\N	\N	\N
41b14af0-c070-4493-a94f-7cfd70edf238	certification	b_corp	Certified B Corp	2026-02-11 08:43:24.767497+00	\N	\N	\N	\N
9d6f69b7-732f-4ae5-8edb-b2a3d9917f22	certification	benefit_corp	Benefit Corporation	2026-02-11 08:43:24.767497+00	\N	\N	\N	\N
007986b1-a04c-41e4-a544-b8fa3badb08c	worker_structure	worker_owned	Worker-Owned	2026-02-11 08:43:24.767497+00	\N	\N	\N	\N
41530222-d629-4f78-bbab-bdf767775bd2	worker_structure	cooperative	Worker Cooperative	2026-02-11 08:43:24.767497+00	\N	\N	\N	\N
9a2900bf-4caa-4995-a027-813c2f38a5f8	business_model	cause_driven	Cause-Driven	2026-02-11 08:43:24.767497+00	\N	\N	\N	\N
6e442e72-de05-43c6-aef4-5b8faa5da30f	business_model	social_enterprise	Social Enterprise	2026-02-11 08:43:24.767497+00	\N	\N	\N	\N
3918ed00-3cbe-4c28-aa60-fca01336f130	provider_category	wellness_coach	Wellness Coach	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
b0577d2b-6136-4a7e-a20f-612366f9fb23	provider_category	therapist	Therapist	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
82aee6b0-e000-4010-b232-86c24a997f70	provider_category	counselor	Counselor	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
67e54549-8fdc-48ef-a3c2-67d469c91606	provider_category	career_coach	Career Coach	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
901d5900-df44-4280-930c-fb2f790294bf	provider_category	peer_support	Peer Support Specialist	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
94a09db2-bc83-472a-8777-971becd5201a	provider_category	life_coach	Life Coach	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
b3ed2c2d-d12b-44b5-9d2e-567c9f9824c8	provider_category	financial_coach	Financial Coach	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
a6d1803b-0161-4500-a820-376225e92787	provider_category	doula	Doula	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
0b176477-879e-475e-a075-eaab1778fabe	provider_category	social_worker	Social Worker	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
70c2f8c4-f8c8-41b9-ac31-68f5123dd49c	provider_category	navigator	Navigator	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
3b4ce8eb-19a1-416b-a50a-5ea737cdb931	provider_specialty	anxiety	Anxiety	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
684b9690-c4a7-4d1e-9f3c-27c81818c2d7	provider_specialty	depression	Depression	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
ddce9202-9701-4ed1-ae74-974dacb15c83	provider_specialty	grief	Grief & Loss	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
ae0c8c7b-f2e4-4be6-b305-a0b4ae6dcc95	provider_specialty	trauma	Trauma & PTSD	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
d5e96885-d878-4ed0-940e-116dbf5ce2ca	provider_specialty	addiction	Addiction & Recovery	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
9b5248f1-7f4d-4156-ac00-6acd5aec7ea7	provider_specialty	relationships	Relationships	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
c969a2ec-152a-46d2-88f8-9db270cf6704	provider_specialty	career_transition	Career Transitions	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
b0739039-091e-46fb-aa92-fb058664f01f	provider_specialty	stress_management	Stress Management	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
f2cc8b25-c828-417a-9ab8-0f6fc46b7fa6	provider_specialty	self_esteem	Self-Esteem	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
fc6a7401-74b1-4286-ae70-ee8a6a0c0c9a	provider_specialty	parenting	Parenting	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
813e1356-92af-41f6-8bd9-da876211ceb5	provider_specialty	life_transition	Life Transitions	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
2b5c0f57-00f0-42a9-85fb-3a9c7ffdbafa	provider_specialty	burnout	Burnout	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
9ba7280e-df77-4e8a-8a59-8194382001fe	provider_specialty	immigration	Immigration Support	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
e3364067-04db-4f3d-a012-b285d61eae7b	provider_specialty	youth	Youth & Adolescents	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
bc0fc66b-0170-4130-9062-2ff3b648fe81	provider_specialty	seniors	Seniors & Aging	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
1cfc3e37-09f5-470d-80f8-9b72fe9d8c9f	provider_specialty	lgbtq	LGBTQ+	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
009121b3-5ff4-4bf4-abcd-421b6c11864e	provider_specialty	veterans	Veterans	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
820d65f1-3d89-4e1c-bc62-1363f0a63e84	provider_specialty	cultural_identity	Cultural Identity	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
0f72955b-504d-473b-b2dd-754c5a1be978	provider_language	en	English	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
beabc8f2-3fa2-442b-aeb5-eb7d873669e2	provider_language	es	Spanish	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
00d9c49d-b3e7-4672-b95e-9b96dbe41d9c	provider_language	hmn	Hmong	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
5e1115ca-c1d7-4fc5-8cf0-67dff74e0d9f	provider_language	so	Somali	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
09a94d38-d331-4a62-9c92-5b55a79ec90c	provider_language	vi	Vietnamese	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
129f9f9c-3982-4e7b-acb3-736e17d2c9df	provider_language	am	Amharic	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
fcbff762-4d54-4971-bf33-a22c38fb8244	provider_language	or	Oromo	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
9c584086-1930-4c23-bb5f-53e7b6332d9f	provider_language	kar	Karen	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
bb2095bd-7f8f-4617-9987-5afa6a2cbeff	provider_language	ar	Arabic	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
4424ce9e-c00b-4871-8d5d-667ccf3b7e3d	provider_language	zh	Chinese (Mandarin)	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
86681da3-d81e-4d87-adb2-0c6152c9cea2	provider_language	ko	Korean	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
315d69f1-9368-4b5a-8be7-de1b46dec2d2	provider_language	ru	Russian	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
a7f0ff2f-03c3-4e53-931f-44bef685df68	provider_language	fr	French	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
04ccde27-af37-4597-8c2c-1e1d9a40ade3	provider_language	pt	Portuguese	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
6aff3882-5917-4d3e-9599-c3c221466b1b	provider_language	sw	Swahili	2026-02-11 08:43:24.947198+00	\N	\N	\N	\N
a0e9735d-cd4e-45c8-ba1c-e11d43622cdc	audience_role	recipient	Recipient	2026-02-11 08:43:24.948422+00	\N	\N	\N	\N
1288dd6d-1d01-4874-84bd-7f2139dad5ac	audience_role	donor	Donor	2026-02-11 08:43:24.948422+00	\N	\N	\N	\N
cfea1ad4-ca97-466a-845a-e2fb0cbbebe2	audience_role	volunteer	Volunteer	2026-02-11 08:43:24.948422+00	\N	\N	\N	\N
16c21e00-6a17-4608-9bc8-23a45db22164	audience_role	participant	Participant	2026-02-11 08:43:24.948422+00	\N	\N	\N	\N
b877ad66-ec8d-45bc-837d-55e742c0436f	service_offered	meal-planning	Meal Planning	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
180e4d78-31d2-4474-a5cc-0ec95a6b3751	service_offered	financial-skills	Financial Skills	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
86606730-4c84-440c-ab75-e8387a0d7aeb	service_offered	self-care	Self-Care	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
6bb44f40-463e-47d9-bf16-23384f91bc85	service_offered	transportation	Transportation	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
032af949-ff64-4785-848a-0e3047e9678b	service_offered	housing	Housing	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
935ab389-f2d7-4a3a-8a85-6b344787e49b	service_offered	legal-aid	Legal Aid	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
676ff3d6-be1b-4dc9-8db9-692f7a93f7c7	service_offered	food-assistance	Food Assistance	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
24fe5431-7741-45f9-b0ea-bb01602e6465	service_offered	job-training	Job Training	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
0a51828d-153c-449a-99e0-cdb2d1a21e74	service_offered	tutoring	Tutoring	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
6e8f55a8-fcd0-47ba-987b-f4016e693ed6	service_offered	mentoring	Mentoring	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
601f9af9-5f72-4991-9606-4f8b22aaf3eb	service_offered	childcare	Childcare	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
c836e1a1-fd7e-4674-a089-cfabe3a5adc6	service_offered	healthcare	Healthcare	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
60387e06-614a-4ca6-af9d-8749ab95a9e3	service_offered	mental-health	Mental Health	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
1d069044-2b43-4425-8171-8801fdd3b34a	service_offered	immigration	Immigration	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
a27ab066-454a-4b53-97b0-998477e6aecd	service_offered	language-classes	Language Classes	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
a817e516-d2c6-47be-97d0-b0f33cc59412	service_offered	citizenship	Citizenship	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
2b64c87e-6e09-4fc5-9e75-26f6060ce70f	service_offered	employment	Employment	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
0d599110-6593-4ed0-9be2-b54061fb6732	business_model	nonprofit	Nonprofit	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
f0e39939-d81e-4d3e-bb33-d4910bad85d3	business_model	social-enterprise	Social Enterprise	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
62555357-3c42-464e-8f4d-d7435bf6cbe1	business_model	donate-proceeds	Donates Proceeds	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
2c9a41ce-534e-4864-8849-71880d678361	business_model	community-owned	Community Owned	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
fcaddb15-e488-4bc0-8301-df19c91ad73d	org_leadership	immigrant-owned	Immigrant-Owned	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
c95dc593-d3a7-4a5b-94dc-0ae0669620e5	org_leadership	refugee-owned	Refugee-Owned	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
33e8d073-3b26-48a3-8801-3eb94239d8d7	org_leadership	woman-owned	Woman-Owned	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
64509b1b-8807-4df9-8f5d-2033b1995d08	org_leadership	veteran-owned	Veteran-Owned	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
257c598f-47eb-4fb4-a487-8d3994f5e70f	org_leadership	bipoc-owned	BIPOC-Owned	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
61f8a926-be46-4916-ac25-f9f616c3e4c2	audience_role	customer	Customer	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
21fad11d-c77a-4c4f-996d-b51ad0b82520	audience_role	job-seeker	Job Seeker	2026-02-11 08:43:24.954461+00	\N	\N	\N	\N
bfd48d7d-fdd3-4781-bc72-caf7cd9cc49d	population	disabilities	People with Disabilities	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
df5e2b02-cb8f-4a51-858c-07ec06de410b	population	brain-injury	Brain Injury	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
2ca06f7d-0fe1-41b8-b7d6-24a43fd0a4d7	population	refugees	Refugees	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
5fa0d814-8c7d-4530-909a-4ae96802aba0	population	immigrants	Immigrants	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
aa15b6fb-6ecb-4f26-ba07-1fb84dff5561	population	homeless	People Experiencing Homelessness	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
cd120dfb-9736-464d-a24d-d2cbc79a90b5	population	low-income	Low Income	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
a84c0464-c84e-4e06-8154-640bcd8b1b31	listing_type	service	Service	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
a9f958a2-ff7e-41fd-bae0-6394e9d7dde2	listing_type	business	Business	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
f593c167-4a0b-4a8b-a879-ff5c2d79f4e9	listing_type	event	Event	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
4ea5d4e1-1041-4aa6-8e14-0d47d37b042f	listing_type	opportunity	Opportunity	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
129f4f24-8017-4379-ae00-db76e06b9664	service_area	twin-cities	Twin Cities Metro	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
241bcd48-33b4-4173-a5fa-55c20ec93b38	service_area	st-paul	St. Paul	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
732b5dd1-8c47-4da3-aadb-d621b41c70bd	service_area	st-cloud	St. Cloud Area	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
23b34dda-a513-4322-9702-6c9257308f16	service_area	rochester	Rochester Area	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
7f0a86f7-ac74-4552-9d75-160a18231f30	service_area	duluth	Duluth Area	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
ec94daa8-ecb3-408f-b492-705a0cf167fb	service_area	central-mn	Central Minnesota	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
ece7807d-5026-4967-b74c-3f7243d84bcc	community_served	east-african	East African	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
c8fdb39d-20c1-4aee-a3de-0ad4937c69e7	community_served	southeast-asian	Southeast Asian	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
3a7b8394-b327-437b-932d-628f67e18340	community_served	arabic-speaking	Arabic Speaking	2026-02-11 08:43:24.994295+00	\N	\N	\N	\N
c2e7fd59-be12-4004-8349-7159fb8247fa	post_type	seeking	I Need Help	2026-02-11 08:43:25.209102+00	\N	\N	Find resources, services, and support available to you	\N
a8c2a6fd-c5c7-43b1-ba01-a7113cfe2b65	post_type	offering	I Want to Support	2026-02-11 08:43:25.209102+00	\N	\N	Discover ways to volunteer, donate, or contribute	\N
bc94b350-5abe-487b-a6a8-2fe601153a81	post_type	announcement	Community Bulletin	2026-02-11 08:43:25.209102+00	\N	\N	Stay informed about community news and events	\N
81da7cfa-49f5-44b6-9b5d-7104862d0a84	public	Volunteer	Volunteer	2026-02-11 09:02:09.498995+00	\N	#2a00a6	\N	\N
7e621a60-fa28-4599-a9cd-e9f6f0103c05	public	food assistance	Food	2026-02-11 09:02:09.498995+00	\N	#00a3a6	\N	\N
13d5edbb-8b6a-434f-80d6-45dbeafcd473	public	Help	Help	2026-02-11 09:02:09.498995+00	\N	#00a630	\N	\N
6a557476-66b9-4b6b-87e0-c677b8aec3f9	public	Donate	Donate	2026-02-11 09:02:09.498995+00	\N	#a30ea6	\N	\N
\.


--
-- Data for Name: tavily_search_queries; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.tavily_search_queries (id, website_research_id, query, search_depth, max_results, days_filter, executed_at) FROM stdin;
\.


--
-- Data for Name: tavily_search_results; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.tavily_search_results (id, query_id, title, url, content, score, published_date, created_at) FROM stdin;
\.


--
-- Data for Name: taxonomy_crosswalks; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.taxonomy_crosswalks (id, tag_id, external_system, external_code, external_name, created_at) FROM stdin;
\.


--
-- Data for Name: website_assessments; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.website_assessments (id, website_id, website_research_id, assessment_markdown, recommendation, confidence_score, organization_name, founded_year, generated_by, generated_at, model_used, reviewed_by_human, human_notes, created_at, updated_at, embedding) FROM stdin;
\.


--
-- Data for Name: website_research; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.website_research (id, website_id, homepage_url, homepage_fetched_at, tavily_searches_completed_at, created_by, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: website_research_homepage; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.website_research_homepage (id, website_research_id, html, markdown, created_at) FROM stdin;
\.


--
-- Data for Name: website_snapshot_listings; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.website_snapshot_listings (id, website_snapshot_id, listing_id, extracted_at) FROM stdin;
\.


--
-- Data for Name: website_snapshots; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.website_snapshots (id, website_id, page_url, page_snapshot_id, submitted_by, submitted_at, last_scraped_at, scrape_status, scrape_error, created_at, updated_at, last_synced_at) FROM stdin;
\.


--
-- Data for Name: website_sources; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.website_sources (id, source_id, domain, max_crawl_depth, crawl_rate_limit_seconds, is_trusted) FROM stdin;
\.


--
-- Data for Name: zip_codes; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.zip_codes (zip_code, city, state, latitude, longitude) FROM stdin;
55001	Afton	MN	44.90717	-92.816193
55002	Almelund	MN	45.513447	-92.894239
55003	Bayport	MN	45.013918	-92.781055
55005	Bethel	MN	45.388716	-93.231538
55006	Braham	MN	45.69146	-93.278348
55007	Brook Park	MN	45.947444	-93.073628
55008	Cambridge	MN	45.602457	-93.263457
55009	Cannon Falls	MN	44.483189	-92.885609
55010	Castle Rock	MN	44.547215	-93.153115
55011	Cedar	MN	45.341448	-93.235026
55012	Center City	MN	45.448211	-92.789369
55013	Chisago City	MN	45.362621	-92.902261
55014	Circle Pines	MN	45.185565	-93.129272
55016	Cottage Grove	MN	44.818216	-92.92861
55017	Dalbo	MN	45.660954	-93.430806
55018	Dennison	MN	44.426537	-92.955359
55019	Dundas	MN	44.398613	-93.194444
55020	Elko	MN	44.588963	-93.369473
55021	Faribault	MN	44.36287	-93.267456
55024	Farmington	MN	44.640515	-93.14196
55025	Forest Lake	MN	45.246466	-92.949266
55026	Frontenac	MN	44.520038	-92.358158
55027	Goodhue	MN	44.427157	-92.620355
55029	Grandy	MN	45.642235	-93.201107
55030	Grasston	MN	45.856598	-93.081255
55031	Hampton	MN	44.608467	-92.955479
55032	Harris	MN	45.593629	-92.998865
55033	Hastings	MN	44.737166	-93.032909
55036	Henriette	MN	45.854335	-93.124235
55037	Hinckley	MN	46.014689	-92.938103
55038	Hugo	MN	45.182366	-92.945218
55040	Isanti	MN	45.565289	-93.287101
55041	Lake City	MN	44.421753	-92.232221
55042	Lake Elmo	MN	44.992866	-92.898177
55043	Lakeland	MN	44.940859	-92.789447
55044	Lakeville	MN	44.633421	-93.25812
55045	Lindstrom	MN	45.404477	-92.823088
55046	Lonsdale	MN	44.447704	-93.425156
55047	Marine On Saint Croix	MN	45.198774	-92.825767
55049	Medford	MN	44.160283	-93.246471
55051	Mora	MN	45.918782	-93.297792
55052	Morristown	MN	44.328365	-93.342652
55053	Nerstrand	MN	44.32738	-93.242155
55054	New Market	MN	44.571056	-93.354267
55055	Newport	MN	44.872395	-92.993263
55056	North Branch	MN	45.556522	-92.885433
55057	Northfield	MN	44.376816	-93.241838
55060	Owatonna	MN	44.047613	-93.223724
55063	Pine City	MN	45.949599	-92.892997
55065	Randolph	MN	44.545066	-93.02496
55066	Red Wing	MN	44.521909	-92.537774
55067	Rock Creek	MN	46.074687	-92.718004
55068	Rosemount	MN	44.661216	-93.076163
55069	Rush City	MN	45.67987	-92.978158
55070	Saint Francis	MN	45.393554	-93.357614
55071	Saint Paul Park	MN	44.822826	-92.989204
55072	Sandstone	MN	46.132453	-92.588959
55073	Scandia	MN	45.253166	-92.837344
55074	Shafer	MN	45.382835	-92.726135
55075	South Saint Paul	MN	44.890315	-93.049879
55076	Inver Grove Heights	MN	44.828816	-93.039064
55077	Inver Grove Heights	MN	44.828265	-93.093969
55078	Stacy	MN	45.513447	-92.894239
55079	Stacy	MN	45.405278	-92.969118
55080	Stanchfield	MN	45.651313	-93.275622
55082	Stillwater	MN	45.061416	-92.84736
55083	Stillwater	MN	45.021016	-92.983726
55084	Taylors Falls	MN	45.457557	-92.733042
55085	Vermillion	MN	44.674799	-92.968309
55087	Warsaw	MN	44.239946	-93.397354
55088	Webster	MN	44.500341	-93.382574
55089	Welch	MN	44.575372	-92.704731
55090	Willernie	MN	45.053466	-92.957034
55092	Wyoming	MN	45.336417	-92.967517
55101	Saint Paul	MN	44.967965	-93.092168
55102	Saint Paul	MN	44.932929	-93.118968
55103	Saint Paul	MN	44.967215	-93.125319
55104	Saint Paul	MN	44.955615	-93.16702
55105	Saint Paul	MN	44.934515	-93.16097
55106	Saint Paul	MN	44.957065	-93.041616
55107	Saint Paul	MN	44.932465	-93.087967
55108	Saint Paul	MN	44.980614	-93.177122
55109	Saint Paul	MN	45.013234	-93.029667
55110	Saint Paul	MN	45.079965	-93.022317
55111	Saint Paul	MN	44.882838	-93.200671
55112	Saint Paul	MN	45.078815	-93.187223
55113	Saint Paul	MN	45.013895	-93.157071
55114	Saint Paul	MN	44.964115	-93.194872
55115	Saint Paul	MN	45.070951	-92.939113
55116	Saint Paul	MN	44.911215	-93.165356
55117	Saint Paul	MN	45.002115	-93.106269
55118	Saint Paul	MN	44.856615	-93.113028
55119	Saint Paul	MN	44.941415	-93.010714
55120	Saint Paul	MN	44.870365	-93.143369
55121	Saint Paul	MN	44.822093	-93.15908
55122	Saint Paul	MN	44.786018	-93.220205
55123	Saint Paul	MN	44.805989	-93.140878
55124	Saint Paul	MN	44.749701	-93.202881
55125	Saint Paul	MN	44.919716	-92.943876
55126	Saint Paul	MN	45.073561	-93.138022
55127	Saint Paul	MN	45.080265	-93.08752
55128	Saint Paul	MN	44.991316	-92.948738
55129	Saint Paul	MN	44.898516	-92.92301
55130	Saint Paul	MN	44.94	-93
55133	Saint Paul	MN	45.005902	-93.105869
55144	Saint Paul	MN	45.005902	-93.105869
55145	Saint Paul	MN	45.005902	-93.105869
55146	Saint Paul	MN	44.942656	-93.082793
55150	Mendota	MN	44.886554	-93.161258
55155	Saint Paul	MN	44.952165	-93.095518
55161	Saint Paul	MN	45.005902	-93.105869
55164	Saint Paul	MN	44.990915	-93.106593
55165	Saint Paul	MN	45.005902	-93.105869
55166	Saint Paul	MN	45.005902	-93.105869
55168	Saint Paul	MN	45.005902	-93.105869
55169	Saint Paul	MN	45.005902	-93.105869
55170	Saint Paul	MN	45.005902	-93.105869
55171	Saint Paul	MN	45.005902	-93.105869
55172	Saint Paul	MN	45.005902	-93.105869
55175	Saint Paul	MN	45.005902	-93.105869
55177	Saint Paul	MN	45.005902	-93.105869
55182	Saint Paul	MN	45.005902	-93.105869
55187	Saint Paul	MN	45.005902	-93.105869
55188	Saint Paul	MN	45.005902	-93.105869
55190	Saint Paul	MN	45.005902	-93.105869
55191	Saint Paul	MN	45.005902	-93.105869
55301	Albertville	MN	45.258673	-93.664342
55302	Annandale	MN	45.218652	-94.105948
55303	Anoka	MN	45.282482	-93.418574
55304	Andover	MN	45.237661	-93.272428
55305	Hopkins	MN	44.952763	-93.43723
55306	Burnsville	MN	44.762208	-93.221535
55307	Arlington	MN	44.597449	-94.105481
55308	Becker	MN	45.451864	-93.842187
55309	Big Lake	MN	45.367294	-93.686916
55310	Bird Island	MN	44.753182	-94.848215
55311	Osseo	MN	45.124263	-93.499583
55312	Brownton	MN	44.758102	-94.251718
55313	Buffalo	MN	45.180732	-93.927556
55314	Buffalo Lake	MN	44.71031	-94.752767
55315	Carver	MN	44.723162	-93.701637
55316	Champlin	MN	45.176914	-93.397481
55317	Chanhassen	MN	44.853364	-93.572584
55318	Chaska	MN	44.810062	-93.653336
55319	Clear Lake	MN	45.466197	-93.952504
55320	Clearwater	MN	45.226627	-93.92441
55321	Cokato	MN	45.086044	-94.185096
55322	Cologne	MN	44.768262	-93.787041
55323	Crystal Bay	MN	45.015914	-93.47188
55324	Darwin	MN	45.070558	-94.420292
55325	Dassel	MN	45.102011	-94.368691
55327	Dayton	MN	45.201514	-93.482833
55328	Delano	MN	45.041472	-93.97792
55329	Eden Valley	MN	45.282592	-94.524712
55330	Elk River	MN	45.403289	-93.644522
55331	Excelsior	MN	44.912712	-93.623186
55332	Fairfax	MN	44.595338	-94.726284
55333	Franklin	MN	44.564953	-94.891273
55334	Gaylord	MN	44.572583	-94.191699
55335	Gibbon	MN	44.55017	-94.543258
55336	Glencoe	MN	44.770238	-94.195685
55337	Burnsville	MN	44.771547	-93.226258
55338	Green Isle	MN	44.63575	-94.057781
55339	Hamburg	MN	44.785812	-93.837873
55340	Hamel	MN	45.073663	-93.568735
55341	Hanover	MN	45.160196	-93.673418
55342	Hector	MN	44.70976	-94.756704
55343	Hopkins	MN	44.913969	-93.44813
55344	Eden Prairie	MN	44.850563	-93.440429
55345	Minnetonka	MN	44.916963	-93.481749
55346	Eden Prairie	MN	44.875463	-93.47808
55347	Eden Prairie	MN	44.841713	-93.459829
55348	Maple Plain	MN	44.848263	-93.398727
55349	Howard Lake	MN	45.101679	-94.047497
55350	Hutchinson	MN	44.875565	-94.266332
55352	Jordan	MN	44.658362	-93.601183
55353	Kimball	MN	45.36261	-94.309928
55354	Lester Prairie	MN	44.880454	-94.100616
55355	Litchfield	MN	45.122737	-94.529861
55356	Long Lake	MN	44.995129	-93.593836
55357	Loretto	MN	45.100098	-93.657256
55358	Maple Lake	MN	45.214072	-94.060149
55359	Maple Plain	MN	44.983312	-93.689162
55360	Mayer	MN	44.905962	-93.913561
55361	Minnetonka Beach	MN	44.940212	-93.592735
55362	Monticello	MN	45.249636	-93.851533
55363	Montrose	MN	45.048202	-94.056543
55364	Mound	MN	44.935062	-93.662938
55365	Monticello	MN	45.200875	-93.888099
55366	New Auburn	MN	44.673454	-94.229311
55367	New Germany	MN	44.899612	-93.970832
55368	Norwood	MN	44.738862	-93.89995
55369	Osseo	MN	45.128414	-93.458932
55370	Plato	MN	44.839939	-94.050518
55371	Princeton	MN	45.740703	-93.63663
55372	Prior Lake	MN	44.682763	-93.464428
55373	Rockford	MN	45.155019	-93.865168
55374	Rogers	MN	45.168896	-93.574586
55375	Saint Bonifacius	MN	44.904062	-93.74904
55376	Saint Michael	MN	45.16826	-93.893628
55377	Santiago	MN	45.540181	-93.815434
55378	Savage	MN	44.751113	-93.367975
55379	Shakopee	MN	44.731113	-93.474144
55380	Silver Creek	MN	45.315823	-93.979766
55381	Silver Lake	MN	44.921134	-94.195425
55382	South Haven	MN	45.265248	-94.165984
55383	Norwood	MN	44.805487	-93.766524
55384	Spring Park	MN	44.936862	-93.630286
55385	Stewart	MN	44.766671	-94.344376
55386	Victoria	MN	44.846645	-93.661737
55387	Waconia	MN	44.844847	-93.746148
55388	Watertown	MN	44.924416	-93.853894
55389	Watkins	MN	45.268619	-94.444561
55390	Waverly	MN	45.060676	-93.974555
55391	Wayzata	MN	44.984663	-93.542233
55392	Navarre	MN	45.015914	-93.47188
55393	Maple Plain	MN	45.200875	-93.888099
55394	Young America	MN	44.805487	-93.766524
55395	Winsted	MN	44.946121	-94.07572
55396	Winthrop	MN	44.550833	-94.347525
55397	Young America	MN	44.800912	-93.919675
55398	Zimmerman	MN	45.467503	-93.602475
55399	Young America	MN	44.805487	-93.766524
55401	Minneapolis	MN	44.979265	-93.273024
55402	Minneapolis	MN	44.975915	-93.271825
55403	Minneapolis	MN	44.972615	-93.287275
55404	Minneapolis	MN	44.948614	-93.329926
55405	Minneapolis	MN	44.970114	-93.300275
55406	Minneapolis	MN	44.976015	-93.278975
55407	Minneapolis	MN	44.935465	-93.254023
55408	Minneapolis	MN	44.947515	-93.288975
55409	Minneapolis	MN	44.925014	-93.289224
55410	Minneapolis	MN	44.912364	-93.318825
55411	Minneapolis	MN	44.999514	-93.297393
55412	Minneapolis	MN	45.025115	-93.298876
55413	Minneapolis	MN	44.994365	-93.240774
55414	Minneapolis	MN	44.974515	-93.234173
55415	Minneapolis	MN	44.974215	-93.258474
55416	Minneapolis	MN	44.949714	-93.337326
55417	Minneapolis	MN	44.962965	-93.253624
55418	Minneapolis	MN	45.017765	-93.244524
55419	Minneapolis	MN	44.890914	-93.282724
55420	Minneapolis	MN	44.835164	-93.255222
55421	Minneapolis	MN	45.052315	-93.254075
55422	Minneapolis	MN	45.009601	-93.342428
55423	Minneapolis	MN	44.875614	-93.255272
55424	Minneapolis	MN	44.905164	-93.340326
55425	Minneapolis	MN	44.842664	-93.236286
55426	Minneapolis	MN	44.955014	-93.382928
55427	Minneapolis	MN	44.999964	-93.390979
55428	Minneapolis	MN	44.981413	-93.372979
55429	Minneapolis	MN	44.975664	-93.336926
55430	Minneapolis	MN	45.063923	-93.302227
55431	Minneapolis	MN	44.828764	-93.311823
55432	Minneapolis	MN	45.094965	-93.23957
55433	Minneapolis	MN	45.164263	-93.319278
55434	Minneapolis	MN	45.170399	-93.226925
55435	Minneapolis	MN	44.932864	-93.367327
55436	Minneapolis	MN	44.901163	-93.42267
55437	Minneapolis	MN	44.826064	-93.353791
55438	Minneapolis	MN	44.826613	-93.375027
55439	Minneapolis	MN	44.874414	-93.375277
55440	Minneapolis	MN	45.015914	-93.47188
55441	Minneapolis	MN	45.005804	-93.419323
55442	Minneapolis	MN	45.04674	-93.431047
55443	Minneapolis	MN	45.119364	-93.34312
55444	Minneapolis	MN	45.117765	-93.305378
55445	Minneapolis	MN	45.123064	-93.352439
55446	Minneapolis	MN	45.040013	-93.486482
55447	Minneapolis	MN	45.003335	-93.487482
55448	Minneapolis	MN	45.174056	-93.313274
55449	Minneapolis	MN	45.169739	-93.188924
55450	Minneapolis	MN	44.881113	-93.220658
55454	Minneapolis	MN	44.980859	-93.252524
55455	Minneapolis	MN	45.038364	-93.298376
55458	Minneapolis	MN	45.015914	-93.47188
55459	Minneapolis	MN	45.015914	-93.47188
55460	Minneapolis	MN	45.015914	-93.47188
55467	Wells Fargo Home Mortgage	MN	44.98	-93.26
55468	Minneapolis	MN	45.015914	-93.47188
55470	Minneapolis	MN	45.015914	-93.47188
55472	Minneapolis	MN	45.015914	-93.47188
55473	Minneapolis	MN	44.805487	-93.766524
55474	Minneapolis	MN	45.015914	-93.47188
55478	Minneapolis	MN	45.015914	-93.47188
55479	Minneapolis	MN	45.015914	-93.47188
55480	Minneapolis	MN	45.015914	-93.47188
55483	Minneapolis	MN	45.015914	-93.47188
55484	Minneapolis	MN	45.015914	-93.47188
55485	Minneapolis	MN	45.015914	-93.47188
55486	Minneapolis	MN	45.015914	-93.47188
55487	Minneapolis	MN	45.015914	-93.47188
55488	Minneapolis	MN	45.015914	-93.47188
55550	Young America	MN	44.805487	-93.766524
55551	Young America	MN	44.805487	-93.766524
55552	Young America	MN	44.805487	-93.766524
55553	Young America	MN	44.805487	-93.766524
55554	Norwood	MN	44.805487	-93.766524
55555	Young America	MN	44.805487	-93.766524
55556	Young America	MN	44.805487	-93.766524
55557	Young America	MN	44.805487	-93.766524
55558	Young America	MN	44.805487	-93.766524
55559	Young America	MN	44.805487	-93.766524
55560	Young America	MN	44.805487	-93.766524
55561	Monticello	MN	44.805487	-93.766524
55562	Young America	MN	44.805487	-93.766524
55563	Monticello	MN	44.805487	-93.766524
55564	Young America	MN	44.805487	-93.766524
55565	Monticello	MN	45.200875	-93.888099
55566	Young America	MN	44.805487	-93.766524
55567	Young America	MN	44.805487	-93.766524
55568	Young America	MN	44.805487	-93.766524
55569	Osseo	MN	45.015914	-93.47188
55570	Maple Plain	MN	45.015914	-93.47188
55571	Maple Plain	MN	45.015914	-93.47188
55572	Maple Plain	MN	45.015914	-93.47188
55573	Young America	MN	45.015914	-93.47188
55574	Maple Plain	MN	45.015914	-93.47188
55575	Howard Lake	MN	45.015914	-93.47188
55576	Maple Plain	MN	45.015914	-93.47188
55577	Maple Plain	MN	45.015914	-93.47188
55578	Maple Plain	MN	45.015914	-93.47188
55579	Maple Plain	MN	45.015914	-93.47188
55580	Monticello	MN	45.200875	-93.888099
55581	Monticello	MN	45.200875	-93.888099
55582	Monticello	MN	45.200875	-93.888099
55583	Norwood	MN	44.805487	-93.766524
55584	Monticello	MN	45.200875	-93.888099
55585	Monticello	MN	45.200875	-93.888099
55586	Monticello	MN	45.200875	-93.888099
55587	Monticello	MN	45.200875	-93.888099
55588	Monticello	MN	44.989512	-93.880245
55589	Monticello	MN	45.200875	-93.888099
55590	Monticello	MN	45.200875	-93.888099
55591	Monticello	MN	45.200875	-93.888099
55592	Maple Plain	MN	45.200875	-93.888099
55593	Maple Plain	MN	45.015914	-93.47188
55594	Young America	MN	44.805487	-93.766524
55595	Loretto	MN	45.015914	-93.47188
55596	Loretto	MN	45.015914	-93.47188
55597	Loretto	MN	45.015914	-93.47188
55598	Loretto	MN	45.015914	-93.47188
55599	Loretto	MN	45.015914	-93.47188
55601	Beaver Bay	MN	47.256021	-91.356586
55602	Brimson	MN	47.256933	-92.00427
55603	Finland	MN	47.497114	-91.320571
55604	Grand Marais	MN	47.872285	-90.42294
55605	Grand Portage	MN	47.923022	-89.851983
55606	Hovland	MN	47.851669	-90.001214
55607	Isabella	MN	47.660406	-91.498861
55609	Knife River	MN	46.95388	-91.777997
55612	Lutsen	MN	47.7059	-90.682372
55613	Schroeder	MN	47.518552	-90.949997
55614	Silver Bay	MN	47.358488	-91.220483
55615	Tofte	MN	47.648636	-90.801861
55616	Two Harbors	MN	47.134891	-91.545363
55701	Adolph	MN	47.640367	-92.442797
55702	Alborn	MN	47.014861	-92.612312
55703	Angora	MN	47.753747	-92.756769
55704	Askov	MN	46.215511	-92.759076
55705	Aurora	MN	47.634557	-92.071317
55706	Babbitt	MN	47.742305	-91.953532
55707	Barnum	MN	46.556833	-92.720097
55708	Biwabik	MN	47.532826	-92.340774
55709	Bovey	MN	47.347269	-93.388826
55710	Britt	MN	47.645047	-92.651923
55711	Brookston	MN	46.837747	-92.680451
55712	Bruno	MN	46.2263	-92.705581
55713	Buhl	MN	47.493197	-92.764262
55716	Calumet	MN	47.322883	-93.276267
55717	Canyon	MN	47.0686	-92.442794
55718	Carlton	MN	46.622795	-92.675569
55719	Chisholm	MN	47.563308	-92.443251
55720	Cloquet	MN	46.592512	-92.549564
55721	Cohasset	MN	47.238241	-93.516501
55722	Coleraine	MN	47.377503	-93.385597
55723	Cook	MN	47.877563	-92.768568
55724	Cotton	MN	47.152067	-92.435223
55725	Crane Lake	MN	48.241245	-92.525385
55726	Cromwell	MN	46.654061	-92.836499
55730	Grand Rapids	MN	47.087782	-93.921429
55731	Ely	MN	47.918943	-92.020778
55732	Embarrass	MN	47.662641	-92.228145
55733	Esko	MN	46.712582	-92.364896
55734	Eveleth	MN	47.386893	-92.452058
55735	Finlayson	MN	46.23792	-92.950358
55736	Floodwood	MN	46.937895	-92.837735
55738	Forbes	MN	47.273191	-92.675563
55741	Gilbert	MN	47.447686	-92.366335
55742	Goodland	MN	47.167838	-93.132367
55744	Grand Rapids	MN	47.232889	-93.393555
55745	Grand Rapids	MN	47.087782	-93.921429
55746	Hibbing	MN	47.413263	-92.87621
55747	Hibbing	MN	47.640367	-92.442797
55748	Hill City	MN	46.671645	-93.432392
55749	Holyoke	MN	46.482697	-92.410931
55750	Hoyt Lakes	MN	47.507262	-92.112844
55751	Iron	MN	47.420472	-92.681078
55752	Jacobson	MN	46.592204	-93.433078
55753	Keewatin	MN	47.398025	-93.078443
55756	Kerrick	MN	46.331356	-92.662237
55757	Kettle River	MN	46.52049	-92.908622
55758	Kinney	MN	47.512132	-92.740216
55760	Mcgregor	MN	46.607188	-93.30756
55763	Makinen	MN	47.264753	-92.181209
55764	Marble	MN	47.321916	-93.29388
55765	Meadowlands	MN	47.116076	-92.803506
55766	Melrude	MN	47.249626	-92.412343
55767	Moose Lake	MN	46.552827	-92.756405
55768	Mountain Iron	MN	47.454993	-92.686192
55769	Nashwauk	MN	47.441216	-93.243688
55771	Orr	MN	47.742195	-92.757902
55772	Nett Lake	MN	48.081686	-93.083438
55775	Pengilly	MN	47.287391	-93.212906
55777	Virginia	MN	47.640367	-92.442797
55779	Saginaw	MN	46.910179	-92.448123
55780	Sawyer	MN	46.701794	-92.639079
55781	Side Lake	MN	47.551459	-92.994885
55782	Soudan	MN	47.821007	-92.246359
55783	Sturgeon Lake	MN	46.367039	-92.824278
55784	Swan River	MN	47.06717	-93.190708
55785	Swatara	MN	46.696774	-93.645502
55786	Taconite	MN	47.316395	-93.342118
55787	Tamarack	MN	46.630318	-93.213416
55790	Tower	MN	47.787247	-92.338233
55791	Twig	MN	47.640367	-92.442797
55792	Virginia	MN	47.646075	-92.499975
55793	Warba	MN	47.241169	-93.228104
55795	Willow River	MN	46.310484	-92.863863
55796	Winton	MN	47.720643	-92.266525
55797	Wrenshall	MN	46.565172	-92.657895
55798	Wright	MN	46.614151	-92.735406
55801	Duluth	MN	47.005566	-92.001934
55802	Duluth	MN	46.904912	-92.039109
55803	Duluth	MN	47.217311	-92.1184
55804	Duluth	MN	46.886239	-92.005488
55805	Duluth	MN	46.800389	-92.094589
55806	Duluth	MN	46.774939	-92.133189
55807	Duluth	MN	46.735978	-92.17764
55808	Duluth	MN	46.683891	-92.242241
55810	Duluth	MN	46.76062	-92.266038
55811	Duluth	MN	46.814712	-92.199825
55812	Duluth	MN	46.810788	-92.072288
55814	Duluth	MN	47.640367	-92.442797
55815	Duluth	MN	47.640367	-92.442797
55816	Duluth	MN	47.640367	-92.442797
55901	Rochester	MN	44.075285	-92.516916
55902	Rochester	MN	43.972494	-92.389901
55903	Rochester	MN	43.996613	-92.540929
55904	Rochester	MN	43.98622	-92.302649
55905	Rochester	MN	44.022513	-92.466826
55906	Rochester	MN	44.107815	-92.405294
55909	Adams	MN	43.565168	-92.74389
55910	Altura	MN	44.085616	-91.946134
55912	Austin	MN	43.699305	-92.976818
55917	Blooming Prairie	MN	44.011627	-93.144007
55918	Brownsdale	MN	43.746834	-92.866996
55919	Brownsville	MN	43.648232	-91.410977
55920	Byron	MN	43.988227	-92.599372
55921	Caledonia	MN	43.635474	-91.458938
55922	Canton	MN	43.527479	-91.85297
55923	Chatfield	MN	43.758684	-92.139962
55924	Claremont	MN	44.046168	-92.975313
55925	Dakota	MN	43.935613	-91.606021
55926	Dexter	MN	43.616293	-92.786355
55927	Dodge Center	MN	44.045362	-92.910808
55929	Dover	MN	43.989637	-92.138889
55931	Eitzen	MN	43.508371	-91.463204
55932	Elgin	MN	44.123683	-92.252177
55933	Elkton	MN	43.664334	-92.682753
55934	Eyota	MN	44.009932	-92.264837
55935	Fountain	MN	43.651181	-92.075216
55936	Grand Meadow	MN	43.668417	-92.544587
55939	Harmony	MN	43.534993	-92.069594
55940	Hayfield	MN	43.896909	-92.797679
55941	Hokah	MN	43.759533	-91.398755
55942	Homer	MN	44.019989	-91.68187
55943	Houston	MN	43.781431	-91.571198
55944	Kasson	MN	44.017216	-92.790593
55945	Kellogg	MN	44.273871	-92.109479
55946	Kenyon	MN	44.294333	-92.905937
55947	La Crescent	MN	43.770564	-91.352968
55949	Lanesboro	MN	43.721194	-91.977384
55950	Lansing	MN	43.762936	-92.965279
55951	Le Roy	MN	43.546515	-92.532554
55952	Lewiston	MN	43.944412	-91.880535
55953	Lyle	MN	43.506952	-92.942939
55954	Mabel	MN	43.521277	-91.768082
55955	Mantorville	MN	44.065741	-92.760046
55956	Mazeppa	MN	44.241752	-92.513947
55957	Millville	MN	44.234483	-92.336216
55959	Minnesota City	MN	44.081907	-91.73508
55960	Oronoco	MN	44.084556	-92.373869
55961	Ostrander	MN	43.714209	-92.087863
55962	Peterson	MN	43.747262	-92.048539
55963	Pine Island	MN	44.261029	-92.710905
55964	Plainview	MN	44.15121	-92.202044
55965	Preston	MN	43.706377	-92.09459
55967	Racine	MN	43.784072	-92.483567
55968	Reads Landing	MN	44.340826	-92.282467
55969	Rollingstone	MN	44.099266	-91.819882
55970	Rose Creek	MN	43.669317	-92.830439
55971	Rushford	MN	43.809873	-91.793376
55972	Saint Charles	MN	43.960809	-91.922346
55973	Sargeant	MN	43.804657	-92.802913
55974	Spring Grove	MN	43.571029	-91.635822
55975	Spring Valley	MN	43.689711	-92.334603
55976	Stewartville	MN	43.884346	-92.503744
55977	Taopi	MN	43.557786	-92.660555
55979	Utica	MN	43.92098	-91.969704
55981	Wabasha	MN	44.3579	-92.087925
55982	Waltham	MN	43.694738	-92.79693
55983	Wanamingo	MN	44.272099	-92.812034
55985	West Concord	MN	44.148244	-92.903452
55987	Winona	MN	44.029975	-91.700889
55988	Stockton	MN	44.025217	-91.770781
55990	Wykoff	MN	43.704566	-92.237117
55991	Zumbro Falls	MN	44.242705	-92.425643
55992	Zumbrota	MN	44.287597	-92.693235
56001	Mankato	MN	44.061451	-94.003112
56002	Mankato	MN	44.056047	-94.069828
56003	Mankato	MN	44.217193	-94.094192
56006	Mankato	MN	44.056047	-94.069828
56007	Albert Lea	MN	43.686288	-93.389838
56009	Alden	MN	43.733525	-93.532143
56010	Amboy	MN	43.886884	-94.15839
56011	Belle Plaine	MN	44.608912	-93.757888
56013	Blue Earth	MN	43.6503	-93.977974
56014	Bricelyn	MN	43.669878	-93.826733
56016	Clarks Grove	MN	43.761669	-93.326712
56017	Cleveland	MN	44.298188	-93.817622
56019	Comfrey	MN	44.111351	-94.907833
56020	Conger	MN	43.62105	-93.548214
56021	Courtland	MN	44.265888	-94.272911
56022	Darfur	MN	44.05483	-94.790185
56023	Delavan	MN	43.768062	-94.007655
56024	Eagle Lake	MN	44.163231	-93.882127
56025	Easton	MN	43.760823	-93.897589
56026	Ellendale	MN	43.927861	-93.286367
56027	Elmore	MN	43.575712	-93.96984
56028	Elysian	MN	44.199317	-93.68198
56029	Emmons	MN	43.652544	-93.403429
56030	Essig	MN	44.325833	-94.605226
56031	Fairmont	MN	43.674049	-94.51078
56032	Freeborn	MN	43.783807	-93.525396
56033	Frost	MN	43.564879	-93.908248
56034	Garden City	MN	44.052118	-94.165036
56035	Geneva	MN	43.673904	-93.348869
56036	Glenville	MN	43.664991	-93.36173
56037	Good Thunder	MN	44.029182	-94.112395
56039	Granada	MN	43.659429	-94.440978
56041	Hanska	MN	44.133457	-94.499485
56042	Hartland	MN	43.803384	-93.485456
56043	Hayward	MN	43.646968	-93.244932
56044	Henderson	MN	44.564912	-93.962668
56045	Hollandale	MN	43.759484	-93.204246
56046	Hope	MN	43.955103	-93.274017
56047	Huntley	MN	43.738397	-94.228897
56048	Janesville	MN	44.051012	-93.58735
56050	Kasota	MN	44.273281	-93.931119
56051	Kiester	MN	43.550446	-93.708504
56052	Kilkenny	MN	44.313417	-93.574653
56054	Lafayette	MN	44.361224	-94.293887
56055	Lake Crystal	MN	44.147701	-94.212574
56056	La Salle	MN	43.978335	-94.614361
56057	Le Center	MN	44.360047	-93.781405
56058	Le Sueur	MN	44.390864	-93.903348
56060	Lewisville	MN	43.923423	-94.434135
56062	Madelia	MN	44.050715	-94.41548
56063	Madison Lake	MN	44.08391	-93.862052
56064	Manchester	MN	43.763839	-93.468959
56065	Mapleton	MN	43.925112	-93.952056
56068	Minnesota Lake	MN	43.811561	-93.817817
56069	Montgomery	MN	44.349609	-93.580277
56071	New Prague	MN	44.536713	-93.55598
56072	New Richland	MN	43.981294	-93.561426
56073	New Ulm	MN	44.259924	-94.511407
56074	Nicollet	MN	44.273214	-94.188233
56075	Northrop	MN	43.735278	-94.435705
56076	Oakland	MN	43.673904	-93.348869
56078	Pemberton	MN	44.007674	-93.783274
56080	Saint Clair	MN	44.081669	-93.857123
56081	Saint James	MN	43.982851	-94.604116
56082	Saint Peter	MN	44.337793	-94.070153
56083	Sanborn	MN	44.282403	-95.167551
56084	Searles	MN	44.302893	-94.738827
56085	Sleepy Eye	MN	44.317309	-94.777163
56087	Springfield	MN	44.253427	-94.903534
56088	Truman	MN	43.789856	-94.430809
56089	Twin Lakes	MN	43.559269	-93.420578
56090	Vernon Center	MN	43.925892	-94.233683
56091	Waldorf	MN	43.907326	-93.682513
56093	Waseca	MN	44.065547	-93.550495
56096	Waterville	MN	44.324235	-93.569726
56097	Wells	MN	43.702814	-93.912041
56098	Winnebago	MN	43.673848	-93.948241
56101	Windom	MN	43.900192	-95.046828
56110	Adrian	MN	43.620754	-95.953225
56111	Alpha	MN	43.674174	-95.154494
56113	Arco	MN	44.382668	-96.1842
56114	Avoca	MN	43.960084	-95.60317
56115	Balaton	MN	44.261464	-95.889647
56116	Beaver Creek	MN	43.612344	-96.364663
56117	Bigelow	MN	43.540643	-95.687951
56118	Bingham Lake	MN	43.895565	-95.048996
56119	Brewster	MN	43.732427	-95.512519
56120	Butterfield	MN	44.002201	-94.814932
56121	Ceylon	MN	43.584995	-94.606517
56122	Chandler	MN	43.92199	-95.81997
56123	Currie	MN	44.048028	-95.704397
56125	Dovray	MN	44.053323	-95.549899
56127	Dunnell	MN	43.674184	-94.550932
56128	Edgerton	MN	43.930332	-96.149676
56129	Ellsworth	MN	43.536342	-95.983538
56131	Fulda	MN	43.905962	-95.593288
56132	Garvin	MN	44.305584	-95.86177
56134	Hardwick	MN	43.810378	-96.218283
56136	Hendricks	MN	44.460984	-96.33771
56137	Heron Lake	MN	43.795977	-95.320571
56138	Hills	MN	43.525185	-96.358365
56139	Holland	MN	44.09239	-96.188124
56140	Ihlen	MN	43.895464	-96.364032
56141	Iona	MN	43.891474	-95.784003
56142	Ivanhoe	MN	44.460411	-96.246374
56143	Jackson	MN	43.650174	-95.021954
56144	Jasper	MN	43.879857	-96.342955
56145	Jeffers	MN	44.055769	-95.195219
56146	Kanaranzi	MN	43.674883	-96.252794
56147	Kenneth	MN	43.674883	-96.252794
56149	Lake Benton	MN	44.294793	-96.270936
56150	Lakefield	MN	43.678125	-95.171548
56151	Lake Wilson	MN	44.00692	-95.825082
56152	Lamberton	MN	44.282497	-95.269338
56153	Leota	MN	43.840423	-96.012811
56155	Lismore	MN	43.682995	-95.942501
56156	Luverne	MN	43.698546	-96.163242
56157	Lynd	MN	44.399913	-95.937984
56158	Magnolia	MN	43.644047	-96.07695
56159	Mountain Lake	MN	43.939276	-94.924319
56160	Odin	MN	43.867848	-94.742716
56161	Okabena	MN	43.738986	-95.316815
56162	Ormsby	MN	43.826503	-94.663493
56164	Pipestone	MN	43.989267	-96.265153
56165	Reading	MN	43.732304	-95.703514
56166	Revere	MN	44.23913	-95.355744
56167	Round Lake	MN	43.631841	-95.640477
56168	Rushmore	MN	43.623088	-95.803869
56169	Russell	MN	44.335085	-95.97804
56170	Ruthton	MN	44.153916	-96.275913
56171	Sherburn	MN	43.667369	-94.759431
56172	Slayton	MN	43.993479	-95.763493
56173	Steen	MN	43.674883	-96.252794
56174	Storden	MN	44.039624	-95.319366
56175	Tracy	MN	44.290092	-95.773754
56176	Trimont	MN	43.783229	-94.713525
56177	Trosky	MN	43.889477	-96.260066
56178	Tyler	MN	44.275371	-96.141967
56180	Walnut Grove	MN	44.283007	-95.48189
56181	Welcome	MN	43.66727	-94.673433
56183	Westbrook	MN	44.007813	-95.196562
56185	Wilmont	MN	43.790084	-95.826712
56186	Woodstock	MN	44.009283	-96.09925
56187	Worthington	MN	43.645207	-95.735375
56201	Willmar	MN	45.147104	-94.977723
56207	Alberta	MN	45.585961	-96.000761
56208	Appleton	MN	45.282008	-95.95757
56209	Atwater	MN	45.109205	-94.968572
56210	Barry	MN	45.559291	-96.558886
56211	Beardsley	MN	45.381402	-96.469532
56212	Bellingham	MN	45.053253	-96.095554
56214	Belview	MN	44.553001	-95.324839
56215	Benson	MN	45.281669	-95.672102
56216	Blomkest	MN	44.950076	-95.058849
56218	Boyd	MN	45.053253	-96.095554
56219	Browns Valley	MN	45.59432	-96.834959
56220	Canby	MN	44.775235	-95.916433
56221	Chokio	MN	45.573876	-96.172979
56222	Clara City	MN	44.992549	-95.36042
56223	Clarkfield	MN	44.790853	-95.806933
56224	Clements	MN	44.418189	-95.261177
56225	Clinton	MN	45.461092	-96.431538
56226	Clontarf	MN	45.305865	-95.838919
56227	Correll	MN	45.381402	-96.469532
56228	Cosmos	MN	45.022087	-94.660884
56229	Cottonwood	MN	44.56956	-95.744921
56230	Danube	MN	44.76007	-95.09754
56231	Danvers	MN	45.281751	-95.721936
56232	Dawson	MN	44.929289	-96.056499
56235	Donnelly	MN	45.690959	-96.010121
56236	Dumont	MN	45.718556	-96.422981
56237	Echo	MN	44.617739	-95.411535
56239	Ghent	MN	44.485432	-95.907811
56240	Graceville	MN	45.521519	-96.440429
56241	Granite Falls	MN	44.780794	-95.670577
56243	Grove City	MN	45.146898	-94.674112
56244	Hancock	MN	45.497402	-95.79426
56245	Hanley Falls	MN	44.692039	-95.62058
56246	Hawick	MN	45.33763	-94.85611
56248	Herman	MN	45.838396	-96.141993
56249	Holloway	MN	45.29184	-95.624619
56251	Kandiyohi	MN	45.142577	-94.918264
56252	Kerkhoven	MN	45.231737	-95.317927
56253	Lake Lillian	MN	45.011036	-94.900783
56255	Lucan	MN	44.472194	-95.409783
56256	Madison	MN	45.038164	-96.311044
56257	Marietta	MN	45.067384	-96.440184
56258	Marshall	MN	44.460429	-95.785872
56260	Maynard	MN	44.999613	-95.573816
56262	Milan	MN	45.108407	-95.817699
56263	Milroy	MN	44.417632	-95.531336
56264	Minneota	MN	44.52307	-95.954362
56265	Montevideo	MN	45.014054	-95.601718
56266	Morgan	MN	44.404589	-94.976823
56267	Morris	MN	45.595739	-95.923233
56270	Morton	MN	44.586097	-94.97066
56271	Murdock	MN	45.281997	-95.512817
56273	New London	MN	45.167597	-95.049378
56274	Norcross	MN	45.934055	-96.012359
56276	Odessa	MN	45.381402	-96.469532
56277	Olivia	MN	44.760343	-95.032641
56278	Ortonville	MN	45.376691	-96.516214
56279	Pennock	MN	45.219496	-95.141233
56280	Porter	MN	44.674215	-96.11204
56281	Prinsburg	MN	45.029662	-95.000152
56282	Raymond	MN	45.094173	-95.111335
56283	Redwood Falls	MN	44.521759	-95.200255
56284	Renville	MN	44.760675	-95.240065
56285	Sacred Heart	MN	44.779761	-95.370482
56287	Seaforth	MN	44.461267	-95.328167
56288	Spicer	MN	45.164862	-95.020124
56289	Sunburg	MN	45.258882	-95.141945
56291	Taunton	MN	44.580923	-95.883387
56292	Vesta	MN	44.492318	-95.447604
56293	Wabasso	MN	44.420734	-95.248847
56294	Wanda	MN	44.32978	-95.211785
56295	Watson	MN	45.019892	-95.630814
56296	Wheaton	MN	45.703481	-96.633211
56297	Wood Lake	MN	44.667478	-95.576938
56301	Saint Cloud	MN	45.519196	-94.330619
56302	Saint Cloud	MN	45.49343	-94.643922
56303	Saint Cloud	MN	45.627994	-94.223023
56304	Saint Cloud	MN	45.544864	-94.440969
56307	Albany	MN	45.614724	-94.494229
56308	Alexandria	MN	45.902017	-95.420589
56309	Ashby	MN	46.09171	-95.816743
56310	Avon	MN	45.599386	-94.436477
56311	Barrett	MN	45.902775	-95.85025
56312	Belgrade	MN	45.509715	-94.963049
56313	Bock	MN	45.784462	-93.552152
56314	Bowlus	MN	45.868809	-94.422896
56315	Brandon	MN	45.966435	-95.516619
56316	Brooten	MN	45.513441	-95.056661
56317	Buckman	MN	46.061307	-94.208731
56318	Burtrum	MN	45.86575	-94.685781
56319	Carlos	MN	45.991208	-95.371207
56320	Cold Spring	MN	45.470708	-94.661654
56321	Collegeville	MN	45.578278	-94.419941
56323	Cyrus	MN	45.641425	-95.709642
56324	Dalton	MN	46.172907	-95.918542
56325	Elrosa	MN	45.563556	-94.946428
56326	Evansville	MN	45.984659	-95.670545
56327	Farwell	MN	45.728746	-95.623944
56328	Flensburg	MN	45.953341	-94.546845
56329	Foley	MN	45.691931	-93.914797
56330	Foreston	MN	45.735675	-93.647135
56331	Freeport	MN	45.643851	-94.660815
56332	Garfield	MN	45.984672	-95.506622
56333	Gilman	MN	45.691714	-94.05629
56334	Glenwood	MN	45.589131	-95.357347
56335	Greenwald	MN	45.597122	-94.851494
56336	Grey Eagle	MN	45.827791	-94.77757
56338	Hillman	MN	45.990074	-93.888513
56339	Hoffman	MN	45.836077	-95.791353
56340	Holdingford	MN	45.623632	-94.41914
56341	Holmes City	MN	45.830998	-95.541618
56342	Isle	MN	45.973268	-93.536504
56343	Kensington	MN	45.811895	-95.665241
56344	Lastrup	MN	46.061307	-94.208731
56345	Little Falls	MN	45.980055	-94.245867
56347	Long Prairie	MN	45.904136	-94.815114
56349	Lowry	MN	45.715846	-95.540402
56350	Mc Grath	MN	46.195728	-93.377414
56352	Melrose	MN	45.614071	-94.634556
56353	Milaca	MN	45.90321	-93.620355
56354	Miltona	MN	46.06187	-95.295146
56355	Nelson	MN	45.935551	-95.23975
56356	New Munich	MN	45.62974	-94.751937
56357	Oak Park	MN	45.702225	-93.816445
56358	Ogilvie	MN	45.833199	-93.402621
56359	Onamia	MN	45.943108	-93.663152
56360	Osakis	MN	45.871169	-95.237474
56361	Parkers Prairie	MN	46.150334	-95.350438
56362	Paynesville	MN	45.506403	-94.734316
56363	Pease	MN	45.697362	-93.646503
56364	Pierz	MN	45.994539	-94.123384
56367	Rice	MN	45.745866	-94.124878
56368	Richmond	MN	45.460536	-94.536053
56369	Rockville	MN	45.468679	-94.340582
56371	Roscoe	MN	45.426781	-94.633502
56372	Saint Cloud	MN	45.52886	-94.593338
56373	Royalton	MN	45.871308	-94.161377
56374	Saint Joseph	MN	45.614235	-94.350962
56375	Saint Stephen	MN	45.587011	-94.380968
56376	Saint Martin	MN	45.489612	-94.718248
56377	Sartell	MN	45.573808	-94.355049
56378	Sauk Centre	MN	45.638568	-94.974275
56379	Sauk Rapids	MN	45.654829	-94.073533
56381	Starbuck	MN	45.572758	-95.573436
56382	Swanville	MN	45.904382	-94.540074
56384	Upsala	MN	45.804875	-94.565187
56385	Villard	MN	45.719299	-95.225097
56386	Wahkon	MN	45.989733	-93.620235
56387	Waite Park	MN	45.510622	-94.667422
56388	Waite Park	MN	45.55	-94.22
56389	West Union	MN	45.799542	-95.08213
56393	Saint Cloud	MN	45.52886	-94.593338
56395	Saint Cloud	MN	45.52886	-94.593338
56396	Saint Cloud	MN	45.52886	-94.593338
56397	Saint Cloud	MN	45.52886	-94.593338
56398	Saint Cloud	MN	45.52886	-94.593338
56399	Saint Cloud	MN	45.52886	-94.593338
56401	Brainerd	MN	46.350195	-94.099983
56425	Baxter	MN	46.373474	-94.196884
56430	Ah Gwah Ching	MN	46.862332	-94.641872
56431	Aitkin	MN	46.563605	-93.430495
56433	Akeley	MN	46.987609	-94.726405
56434	Aldrich	MN	46.379683	-94.936381
56435	Backus	MN	46.803348	-94.521914
56436	Benedict	MN	47.108153	-94.921064
56437	Bertha	MN	46.268233	-95.06077
56438	Browerville	MN	46.063955	-94.867727
56440	Clarissa	MN	46.128416	-94.950401
56441	Crosby	MN	46.537059	-93.928197
56442	Crosslake	MN	46.67734	-94.112783
56443	Cushing	MN	46.205759	-94.561294
56444	Deerwood	MN	46.429194	-93.878493
56446	Eagle Bend	MN	46.149778	-94.99949
56447	Emily	MN	46.697119	-94.117266
56448	Fifty Lakes	MN	46.727867	-94.040303
56449	Fort Ripley	MN	46.200293	-94.245167
56450	Garrison	MN	46.312962	-93.866016
56452	Hackensack	MN	46.939139	-94.450641
56453	Hewitt	MN	46.330157	-94.945126
56455	Ironton	MN	46.477792	-93.978854
56456	Jenkins	MN	46.480723	-94.08587
56458	Lake George	MN	47.108153	-94.921064
56459	Lake Hubert	MN	46.498749	-94.251926
56461	Laporte	MN	47.108153	-94.921064
56464	Menahga	MN	46.730538	-94.975221
56465	Merrifield	MN	46.539308	-94.134385
56466	Motley	MN	46.288105	-94.563773
56467	Nevis	MN	46.94336	-94.844112
56468	Nisswa	MN	46.401244	-94.237094
56469	Palisade	MN	46.712878	-93.489808
56470	Park Rapids	MN	46.984699	-95.09935
56472	Pequot Lakes	MN	46.616147	-94.235561
56473	Pillager	MN	46.693034	-94.464381
56474	Pine River	MN	46.712158	-94.251126
56475	Randall	MN	46.105236	-94.531738
56477	Sebeka	MN	46.652292	-94.974942
56478	Nimrod	MN	46.605266	-94.900729
56479	Staples	MN	46.250882	-94.934361
56481	Verndale	MN	46.506528	-94.967564
56482	Wadena	MN	46.564002	-95.082796
56484	Walker	MN	47.067057	-94.489824
56501	Detroit Lakes	MN	46.834262	-95.746871
56502	Detroit Lakes	MN	46.933961	-95.678375
56510	Ada	MN	47.325283	-96.597259
56511	Audubon	MN	46.850852	-95.995824
56513	Baker	MN	46.890034	-96.506156
56514	Barnesville	MN	46.649467	-96.391637
56515	Battle Lake	MN	46.294519	-95.707485
56516	Bejou	MN	47.325198	-95.80918
56517	Beltrami	MN	47.801705	-96.43368
56518	Bluffton	MN	46.412413	-95.713452
56519	Borup	MN	47.201971	-96.500374
56520	Breckenridge	MN	46.191367	-96.500224
56521	Callaway	MN	46.979328	-95.912192
56522	Campbell	MN	46.140224	-96.443327
56523	Climax	MN	47.683698	-96.87154
56524	Clitherall	MN	46.275667	-95.630788
56525	Comstock	MN	46.890034	-96.506156
56527	Deer Creek	MN	46.392933	-95.318951
56528	Dent	MN	46.57101	-95.728629
56529	Dilworth	MN	46.877143	-96.709806
56531	Elbow Lake	MN	45.997662	-95.963007
56533	Elizabeth	MN	46.380324	-96.132614
56534	Erhard	MN	46.483858	-96.097914
56535	Erskine	MN	47.665688	-95.99807
56536	Felton	MN	47.077246	-96.503987
56537	Fergus Falls	MN	46.3194	-95.657003
56538	Fergus Falls	MN	46.412413	-95.713452
56540	Fertile	MN	47.534788	-96.285663
56541	Flom	MN	47.325074	-96.469194
56542	Fosston	MN	47.597727	-96.270444
56543	Foxhome	MN	46.326166	-96.528032
56544	Frazee	MN	46.803165	-95.579405
56545	Gary	MN	47.372863	-96.264276
56546	Georgetown	MN	46.890034	-96.506156
56547	Glyndon	MN	46.870695	-96.576425
56548	Halstad	MN	47.350668	-96.82368
56549	Hawley	MN	46.977738	-96.409155
56550	Hendrum	MN	47.263731	-96.811279
56551	Henning	MN	46.293243	-95.483624
56552	Hitterdal	MN	46.972026	-96.25589
56553	Kent	MN	46.326166	-96.528032
56554	Lake Park	MN	46.891231	-96.102425
56556	Mcintosh	MN	47.637117	-95.884768
56557	Mahnomen	MN	47.287889	-95.939586
56560	Moorhead	MN	46.803546	-96.557389
56561	Moorhead	MN	46.890034	-96.506156
56562	Moorhead	MN	46.890034	-96.506156
56563	Moorhead	MN	46.890034	-96.506156
56565	Nashua	MN	46.326166	-96.528032
56566	Naytahwaush	MN	47.325198	-95.80918
56567	New York Mills	MN	46.491294	-95.366068
56568	Nielsville	MN	47.836367	-96.3504
56569	Ogema	MN	47.10914	-95.782254
56570	Osage	MN	46.923974	-95.362298
56571	Ottertail	MN	46.465546	-95.564365
56572	Pelican Rapids	MN	46.611549	-96.059669
56573	Perham	MN	46.597093	-95.822634
56574	Perley	MN	47.177615	-96.804613
56575	Ponsford	MN	47.061724	-95.429915
56576	Richville	MN	46.443365	-95.792367
56577	Richwood	MN	46.933961	-95.678375
56578	Rochert	MN	46.886152	-95.724599
56579	Rothsay	MN	46.529553	-96.349085
56580	Sabin	MN	46.779487	-96.651185
56581	Shelly	MN	47.320042	-96.54441
56583	Tintah	MN	46.007415	-96.359342
56584	Twin Valley	MN	47.271463	-96.182441
56585	Ulen	MN	47.0663	-96.258706
56586	Underwood	MN	46.285081	-95.874117
56587	Vergas	MN	46.457968	-95.919355
56588	Vining	MN	46.412413	-95.713452
56589	Waubun	MN	47.184343	-95.939849
56590	Wendell	MN	45.934055	-96.012359
56591	White Earth	MN	46.933961	-95.678375
56592	Winger	MN	47.631462	-95.889453
56593	Wolf Lake	MN	46.821648	-95.391968
56594	Wolverton	MN	46.326166	-96.528032
56601	Bemidji	MN	47.571964	-94.801272
56619	Bemidji	MN	47.625699	-94.822154
56621	Bagley	MN	47.531644	-95.377949
56623	Baudette	MN	48.750473	-94.84626
56626	Bena	MN	47.370372	-94.251376
56627	Big Falls	MN	48.091615	-93.81606
56628	Bigfork	MN	47.710353	-93.612694
56629	Birchdale	MN	48.642546	-94.06358
56630	Blackduck	MN	47.804493	-94.575871
56631	Bowstring	MN	47.087782	-93.921429
56633	Cass Lake	MN	47.327719	-94.476853
56634	Clearbrook	MN	47.610498	-95.421104
56636	Deer River	MN	47.46843	-93.810949
56637	Talmoon	MN	47.615375	-93.837811
56639	Effie	MN	47.852226	-93.524252
56641	Federal Dam	MN	47.206552	-94.263384
56644	Gonvick	MN	47.715406	-95.470782
56646	Gully	MN	47.836367	-96.3504
56647	Hines	MN	47.974989	-95.008708
56649	International Falls	MN	48.232494	-93.640382
56650	Kelliher	MN	47.940789	-94.45001
56651	Lengby	MN	47.836367	-96.3504
56652	Leonard	MN	47.585873	-95.375974
56653	Littlefork	MN	48.357144	-93.612836
56654	Loman	MN	48.538803	-93.840769
56655	Longville	MN	46.994097	-94.243551
56657	Marcell	MN	47.548518	-93.62356
56658	Margie	MN	48.27888	-93.755536
56659	Max	MN	47.661759	-94.015607
56660	Mizpah	MN	47.933964	-94.23693
56661	Northome	MN	47.919215	-94.097415
56662	Outing	MN	46.879043	-93.918172
56663	Pennington	MN	47.448274	-94.471485
56666	Ponemah	MN	47.974989	-95.008708
56667	Puposky	MN	47.974989	-95.008708
56668	Ranier	MN	48.603143	-93.29771
56669	Ray	MN	48.394721	-93.310667
56670	Redby	MN	47.974989	-95.008708
56671	Redlake	MN	47.974989	-95.008708
56672	Remer	MN	47.095793	-94.021032
56673	Roosevelt	MN	48.769244	-95.747559
56676	Shevlin	MN	47.585873	-95.375974
56678	Solway	MN	47.974989	-95.008708
56679	South International Falls	MN	48.27888	-93.755536
56680	Spring Lake	MN	47.635946	-93.922032
56681	Squaw Lake	MN	47.624415	-94.187735
56682	Swift	MN	48.769244	-95.747559
56683	Tenstrike	MN	47.974989	-95.008708
56684	Trail	MN	47.836367	-96.3504
56685	Waskish	MN	47.974989	-95.008708
56686	Williams	MN	48.820843	-94.933138
56687	Wilton	MN	47.974989	-95.008708
56688	Wirt	MN	47.746177	-93.962961
56701	Thief River Falls	MN	48.076333	-96.149002
56710	Alvarado	MN	48.191085	-96.998433
56711	Angle Inlet	MN	48.875164	-94.885668
56712	Angus	MN	47.836367	-96.3504
56713	Argyle	MN	48.331455	-96.816197
56714	Badger	MN	48.774428	-96.020334
56715	Brooks	MN	47.814152	-96.005263
56716	Crookston	MN	47.705082	-96.412
56720	Donaldson	MN	48.771938	-96.812921
56721	East Grand Forks	MN	47.874048	-96.924085
56722	Euclid	MN	47.836367	-96.3504
56723	Fisher	MN	47.799949	-96.798532
56724	Gatzke	MN	48.358371	-96.378062
56725	Goodridge	MN	48.144158	-95.804345
56726	Greenbush	MN	48.834783	-96.286287
56727	Grygla	MN	48.358371	-96.378062
56728	Hallock	MN	48.774787	-96.942022
56729	Halma	MN	48.771938	-96.812921
56731	Humboldt	MN	48.771938	-96.812921
56732	Karlstad	MN	48.565003	-96.53266
56733	Kennedy	MN	48.643714	-96.914605
56734	Lake Bronson	MN	48.771938	-96.812921
56735	Lancaster	MN	48.771938	-96.812921
56736	Mentor	MN	47.681728	-96.154311
56737	Middle River	MN	48.438104	-96.513779
56738	Newfolden	MN	48.358371	-96.378062
56740	Noyes	MN	48.771938	-96.812921
56741	Oak Island	MN	48.875164	-94.885668
56742	Oklee	MN	47.838345	-95.853261
56744	Oslo	MN	48.19941	-97.130755
56748	Plummer	MN	47.86217	-96.095883
56750	Red Lake Falls	MN	47.885347	-96.270368
56751	Roseau	MN	48.704839	-95.750383
56754	Saint Hilaire	MN	48.014969	-96.213472
56755	Saint Vincent	MN	48.871994	-97.092442
56756	Salol	MN	48.769244	-95.747559
56757	Stephen	MN	48.477158	-96.867548
56758	Strandquist	MN	48.358371	-96.378062
56759	Strathcona	MN	48.749138	-96.062783
56760	Viking	MN	48.358371	-96.378062
56761	Wannaska	MN	48.769244	-95.747559
56762	Warren	MN	48.261376	-96.772583
56763	Warroad	MN	48.900663	-95.250324
\.


--
-- Name: seesaw_dlq_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.seesaw_dlq_id_seq', 1, false);


--
-- Name: seesaw_events_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.seesaw_events_id_seq', 1, false);


--
-- Name: _sqlx_migrations _sqlx_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public._sqlx_migrations
    ADD CONSTRAINT _sqlx_migrations_pkey PRIMARY KEY (version);


--
-- Name: active_languages active_languages_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.active_languages
    ADD CONSTRAINT active_languages_pkey PRIMARY KEY (language_code);


--
-- Name: agent_assistant_configs agent_assistant_configs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.agent_assistant_configs
    ADD CONSTRAINT agent_assistant_configs_pkey PRIMARY KEY (agent_id);


--
-- Name: agents agents_member_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.agents
    ADD CONSTRAINT agents_member_id_key UNIQUE (member_id);


--
-- Name: agents agents_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.agents
    ADD CONSTRAINT agents_pkey PRIMARY KEY (id);


--
-- Name: business_listings business_listings_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.business_listings
    ADD CONSTRAINT business_listings_pkey PRIMARY KEY (listing_id);


--
-- Name: containers chatrooms_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.containers
    ADD CONSTRAINT chatrooms_pkey PRIMARY KEY (id);


--
-- Name: contacts contacts_contactable_type_contactable_id_contact_type_conta_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.contacts
    ADD CONSTRAINT contacts_contactable_type_contactable_id_contact_type_conta_key UNIQUE (contactable_type, contactable_id, contact_type, contact_value);


--
-- Name: contacts contacts_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.contacts
    ADD CONSTRAINT contacts_pkey PRIMARY KEY (id);


--
-- Name: document_references document_references_document_id_reference_kind_reference_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.document_references
    ADD CONSTRAINT document_references_document_id_reference_kind_reference_id_key UNIQUE (document_id, reference_kind, reference_id);


--
-- Name: document_references document_references_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.document_references
    ADD CONSTRAINT document_references_pkey PRIMARY KEY (id);


--
-- Name: website_assessments domain_assessments_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_assessments
    ADD CONSTRAINT domain_assessments_pkey PRIMARY KEY (id);


--
-- Name: website_research_homepage domain_research_homepage_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_research_homepage
    ADD CONSTRAINT domain_research_homepage_pkey PRIMARY KEY (id);


--
-- Name: website_research domain_research_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_research
    ADD CONSTRAINT domain_research_pkey PRIMARY KEY (id);


--
-- Name: website_snapshots domain_snapshots_domain_id_page_url_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_snapshots
    ADD CONSTRAINT domain_snapshots_domain_id_page_url_key UNIQUE (website_id, page_url);


--
-- Name: website_snapshots domain_snapshots_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_snapshots
    ADD CONSTRAINT domain_snapshots_pkey PRIMARY KEY (id);


--
-- Name: extraction_embeddings extraction_embeddings_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.extraction_embeddings
    ADD CONSTRAINT extraction_embeddings_pkey PRIMARY KEY (url);


--
-- Name: extraction_pages extraction_pages_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.extraction_pages
    ADD CONSTRAINT extraction_pages_pkey PRIMARY KEY (url);


--
-- Name: extraction_summaries extraction_summaries_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.extraction_summaries
    ADD CONSTRAINT extraction_summaries_pkey PRIMARY KEY (url);


--
-- Name: identifiers identifiers_phone_hash_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.identifiers
    ADD CONSTRAINT identifiers_phone_hash_key UNIQUE (phone_hash);


--
-- Name: identifiers identifiers_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.identifiers
    ADD CONSTRAINT identifiers_pkey PRIMARY KEY (id);


--
-- Name: jobs jobs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.jobs
    ADD CONSTRAINT jobs_pkey PRIMARY KEY (id);


--
-- Name: jobs jobs_reference_id_job_type_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.jobs
    ADD CONSTRAINT jobs_reference_id_job_type_unique UNIQUE (reference_id, job_type);


--
-- Name: listing_contacts listing_contacts_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.listing_contacts
    ADD CONSTRAINT listing_contacts_pkey PRIMARY KEY (id);


--
-- Name: listing_delivery_modes listing_delivery_modes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.listing_delivery_modes
    ADD CONSTRAINT listing_delivery_modes_pkey PRIMARY KEY (listing_id, delivery_mode);


--
-- Name: post_page_sources listing_page_sources_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.post_page_sources
    ADD CONSTRAINT listing_page_sources_pkey PRIMARY KEY (post_id, page_snapshot_id);


--
-- Name: listing_reports listing_reports_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.listing_reports
    ADD CONSTRAINT listing_reports_pkey PRIMARY KEY (id);


--
-- Name: locations locations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.locations
    ADD CONSTRAINT locations_pkey PRIMARY KEY (id);


--
-- Name: messages messages_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.messages
    ADD CONSTRAINT messages_pkey PRIMARY KEY (id);


--
-- Name: migration_workflows migration_workflows_name_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.migration_workflows
    ADD CONSTRAINT migration_workflows_name_key UNIQUE (name);


--
-- Name: migration_workflows migration_workflows_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.migration_workflows
    ADD CONSTRAINT migration_workflows_pkey PRIMARY KEY (id);


--
-- Name: noteables noteables_note_id_noteable_type_noteable_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.noteables
    ADD CONSTRAINT noteables_note_id_noteable_type_noteable_id_key UNIQUE (note_id, noteable_type, noteable_id);


--
-- Name: noteables noteables_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.noteables
    ADD CONSTRAINT noteables_pkey PRIMARY KEY (id);


--
-- Name: notes notes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.notes
    ADD CONSTRAINT notes_pkey PRIMARY KEY (id);


--
-- Name: opportunity_listings opportunity_listings_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.opportunity_listings
    ADD CONSTRAINT opportunity_listings_pkey PRIMARY KEY (listing_id);


--
-- Name: posts organization_needs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.posts
    ADD CONSTRAINT organization_needs_pkey PRIMARY KEY (id);


--
-- Name: organization_tags organization_tags_organization_id_kind_value_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.organization_tags
    ADD CONSTRAINT organization_tags_organization_id_kind_value_key UNIQUE (organization_id, kind, value);


--
-- Name: organization_tags organization_tags_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.organization_tags
    ADD CONSTRAINT organization_tags_pkey PRIMARY KEY (id);


--
-- Name: organizations organizations_name_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.organizations
    ADD CONSTRAINT organizations_name_unique UNIQUE (name);


--
-- Name: organizations organizations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.organizations
    ADD CONSTRAINT organizations_pkey PRIMARY KEY (id);


--
-- Name: page_extractions page_extractions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.page_extractions
    ADD CONSTRAINT page_extractions_pkey PRIMARY KEY (id);


--
-- Name: page_snapshots page_snapshots_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.page_snapshots
    ADD CONSTRAINT page_snapshots_pkey PRIMARY KEY (id);


--
-- Name: page_snapshots page_snapshots_url_content_hash_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.page_snapshots
    ADD CONSTRAINT page_snapshots_url_content_hash_key UNIQUE (url, content_hash);


--
-- Name: page_summaries page_summaries_content_hash_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.page_summaries
    ADD CONSTRAINT page_summaries_content_hash_key UNIQUE (content_hash);


--
-- Name: page_summaries page_summaries_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.page_summaries
    ADD CONSTRAINT page_summaries_pkey PRIMARY KEY (id);


--
-- Name: post_locations post_locations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.post_locations
    ADD CONSTRAINT post_locations_pkey PRIMARY KEY (id);


--
-- Name: post_locations post_locations_post_id_location_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.post_locations
    ADD CONSTRAINT post_locations_post_id_location_id_key UNIQUE (post_id, location_id);


--
-- Name: post_sources post_sources_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.post_sources
    ADD CONSTRAINT post_sources_pkey PRIMARY KEY (id);


--
-- Name: post_sources post_sources_post_id_source_type_source_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.post_sources
    ADD CONSTRAINT post_sources_post_id_source_type_source_id_key UNIQUE (post_id, source_type, source_id);


--
-- Name: providers providers_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.providers
    ADD CONSTRAINT providers_pkey PRIMARY KEY (id);


--
-- Name: referral_document_translations referral_document_translations_document_id_language_code_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.referral_document_translations
    ADD CONSTRAINT referral_document_translations_document_id_language_code_key UNIQUE (document_id, language_code);


--
-- Name: referral_document_translations referral_document_translations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.referral_document_translations
    ADD CONSTRAINT referral_document_translations_pkey PRIMARY KEY (id);


--
-- Name: referral_documents referral_documents_edit_token_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.referral_documents
    ADD CONSTRAINT referral_documents_edit_token_key UNIQUE (edit_token);


--
-- Name: referral_documents referral_documents_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.referral_documents
    ADD CONSTRAINT referral_documents_pkey PRIMARY KEY (id);


--
-- Name: referral_documents referral_documents_slug_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.referral_documents
    ADD CONSTRAINT referral_documents_slug_key UNIQUE (slug);


--
-- Name: schedules schedules_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.schedules
    ADD CONSTRAINT schedules_pkey PRIMARY KEY (id);


--
-- Name: scrape_jobs scrape_jobs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.scrape_jobs
    ADD CONSTRAINT scrape_jobs_pkey PRIMARY KEY (id);


--
-- Name: search_queries search_queries_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.search_queries
    ADD CONSTRAINT search_queries_pkey PRIMARY KEY (id);


--
-- Name: seesaw_dlq seesaw_dlq_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.seesaw_dlq
    ADD CONSTRAINT seesaw_dlq_pkey PRIMARY KEY (id);


--
-- Name: seesaw_effect_executions seesaw_effect_executions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.seesaw_effect_executions
    ADD CONSTRAINT seesaw_effect_executions_pkey PRIMARY KEY (event_id, effect_id);


--
-- Name: seesaw_events seesaw_events_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.seesaw_events
    ADD CONSTRAINT seesaw_events_pkey PRIMARY KEY (id, created_at);


--
-- Name: seesaw_events_default seesaw_events_default_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.seesaw_events_default
    ADD CONSTRAINT seesaw_events_default_pkey PRIMARY KEY (id, created_at);


--
-- Name: seesaw_join_entries seesaw_join_entries_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.seesaw_join_entries
    ADD CONSTRAINT seesaw_join_entries_pkey PRIMARY KEY (join_effect_id, correlation_id, source_event_id);


--
-- Name: seesaw_join_windows seesaw_join_windows_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.seesaw_join_windows
    ADD CONSTRAINT seesaw_join_windows_pkey PRIMARY KEY (join_effect_id, correlation_id, batch_id);


--
-- Name: seesaw_processed seesaw_processed_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.seesaw_processed
    ADD CONSTRAINT seesaw_processed_pkey PRIMARY KEY (event_id);


--
-- Name: seesaw_state seesaw_state_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.seesaw_state
    ADD CONSTRAINT seesaw_state_pkey PRIMARY KEY (correlation_id);


--
-- Name: service_listings service_listings_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.service_listings
    ADD CONSTRAINT service_listings_pkey PRIMARY KEY (listing_id);


--
-- Name: social_sources social_sources_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.social_sources
    ADD CONSTRAINT social_sources_pkey PRIMARY KEY (id);


--
-- Name: social_sources social_sources_source_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.social_sources
    ADD CONSTRAINT social_sources_source_id_key UNIQUE (source_id);


--
-- Name: social_sources social_sources_source_type_handle_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.social_sources
    ADD CONSTRAINT social_sources_source_type_handle_key UNIQUE (source_type, handle);


--
-- Name: sources sources_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sources
    ADD CONSTRAINT sources_pkey PRIMARY KEY (id);


--
-- Name: sync_batches sync_batches_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sync_batches
    ADD CONSTRAINT sync_batches_pkey PRIMARY KEY (id);


--
-- Name: sync_proposal_merge_sources sync_proposal_merge_sources_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sync_proposal_merge_sources
    ADD CONSTRAINT sync_proposal_merge_sources_pkey PRIMARY KEY (id);


--
-- Name: sync_proposal_merge_sources sync_proposal_merge_sources_proposal_id_source_entity_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sync_proposal_merge_sources
    ADD CONSTRAINT sync_proposal_merge_sources_proposal_id_source_entity_id_key UNIQUE (proposal_id, source_entity_id);


--
-- Name: sync_proposals sync_proposals_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sync_proposals
    ADD CONSTRAINT sync_proposals_pkey PRIMARY KEY (id);


--
-- Name: tag_kinds tag_kinds_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tag_kinds
    ADD CONSTRAINT tag_kinds_pkey PRIMARY KEY (id);


--
-- Name: tag_kinds tag_kinds_slug_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tag_kinds
    ADD CONSTRAINT tag_kinds_slug_key UNIQUE (slug);


--
-- Name: taggables taggables_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.taggables
    ADD CONSTRAINT taggables_pkey PRIMARY KEY (id);


--
-- Name: taggables taggables_tag_id_taggable_type_taggable_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.taggables
    ADD CONSTRAINT taggables_tag_id_taggable_type_taggable_id_key UNIQUE (tag_id, taggable_type, taggable_id);


--
-- Name: tags tags_kind_value_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tags
    ADD CONSTRAINT tags_kind_value_key UNIQUE (kind, value);


--
-- Name: tags tags_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tags
    ADD CONSTRAINT tags_pkey PRIMARY KEY (id);


--
-- Name: tavily_search_queries tavily_search_queries_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tavily_search_queries
    ADD CONSTRAINT tavily_search_queries_pkey PRIMARY KEY (id);


--
-- Name: tavily_search_results tavily_search_results_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tavily_search_results
    ADD CONSTRAINT tavily_search_results_pkey PRIMARY KEY (id);


--
-- Name: taxonomy_crosswalks taxonomy_crosswalks_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.taxonomy_crosswalks
    ADD CONSTRAINT taxonomy_crosswalks_pkey PRIMARY KEY (id);


--
-- Name: taxonomy_crosswalks taxonomy_crosswalks_tag_id_external_system_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.taxonomy_crosswalks
    ADD CONSTRAINT taxonomy_crosswalks_tag_id_external_system_key UNIQUE (tag_id, external_system);


--
-- Name: members volunteers_expo_push_token_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.members
    ADD CONSTRAINT volunteers_expo_push_token_key UNIQUE (expo_push_token);


--
-- Name: members volunteers_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.members
    ADD CONSTRAINT volunteers_pkey PRIMARY KEY (id);


--
-- Name: website_snapshot_listings website_snapshot_listings_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_snapshot_listings
    ADD CONSTRAINT website_snapshot_listings_pkey PRIMARY KEY (id);


--
-- Name: website_snapshot_listings website_snapshot_listings_website_snapshot_id_listing_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_snapshot_listings
    ADD CONSTRAINT website_snapshot_listings_website_snapshot_id_listing_id_key UNIQUE (website_snapshot_id, listing_id);


--
-- Name: website_sources website_sources_domain_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_sources
    ADD CONSTRAINT website_sources_domain_key UNIQUE (domain);


--
-- Name: website_sources website_sources_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_sources
    ADD CONSTRAINT website_sources_pkey PRIMARY KEY (id);


--
-- Name: website_sources website_sources_source_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_sources
    ADD CONSTRAINT website_sources_source_id_key UNIQUE (source_id);


--
-- Name: zip_codes zip_codes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.zip_codes
    ADD CONSTRAINT zip_codes_pkey PRIMARY KEY (zip_code);


--
-- Name: idx_agent_assistant_configs_config_name; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_agent_assistant_configs_config_name ON public.agent_assistant_configs USING btree (config_name);


--
-- Name: idx_business_support_needed; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_business_support_needed ON public.business_listings USING gin (support_needed);


--
-- Name: idx_contacts_entity; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_contacts_entity ON public.contacts USING btree (contactable_type, contactable_id);


--
-- Name: idx_contacts_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_contacts_type ON public.contacts USING btree (contact_type);


--
-- Name: idx_containers_activity; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_containers_activity ON public.containers USING btree (last_activity_at DESC);


--
-- Name: idx_containers_tags; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_containers_tags ON public.containers USING gin (tags);


--
-- Name: idx_crosswalks_external; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_crosswalks_external ON public.taxonomy_crosswalks USING btree (external_system, external_code);


--
-- Name: idx_crosswalks_tag; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_crosswalks_tag ON public.taxonomy_crosswalks USING btree (tag_id);


--
-- Name: idx_document_refs_document; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_document_refs_document ON public.document_references USING btree (document_id);


--
-- Name: idx_document_refs_kind_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_document_refs_kind_id ON public.document_references USING btree (reference_kind, reference_id);


--
-- Name: idx_document_translations_document; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_document_translations_document ON public.referral_document_translations USING btree (document_id);


--
-- Name: idx_document_translations_language; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_document_translations_language ON public.referral_document_translations USING btree (language_code);


--
-- Name: idx_documents_container; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_documents_container ON public.referral_documents USING btree (container_id);


--
-- Name: idx_documents_language; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_documents_language ON public.referral_documents USING btree (source_language);


--
-- Name: idx_documents_slug; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_documents_slug ON public.referral_documents USING btree (slug);


--
-- Name: idx_documents_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_documents_status ON public.referral_documents USING btree (status);


--
-- Name: idx_domain_assessments_generated_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_domain_assessments_generated_at ON public.website_assessments USING btree (generated_at DESC);


--
-- Name: idx_domain_assessments_recommendation; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_domain_assessments_recommendation ON public.website_assessments USING btree (recommendation);


--
-- Name: idx_domain_research_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_domain_research_created_at ON public.website_research USING btree (created_at DESC);


--
-- Name: idx_domain_snapshots_page_snapshot_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_domain_snapshots_page_snapshot_id ON public.website_snapshots USING btree (page_snapshot_id);


--
-- Name: idx_domain_snapshots_pending; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_domain_snapshots_pending ON public.website_snapshots USING btree (website_id, scrape_status) WHERE (scrape_status = 'pending'::text);


--
-- Name: idx_domain_snapshots_scrape_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_domain_snapshots_scrape_status ON public.website_snapshots USING btree (scrape_status);


--
-- Name: idx_extraction_embeddings_site_url; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_extraction_embeddings_site_url ON public.extraction_embeddings USING btree (site_url);


--
-- Name: idx_extraction_pages_content_tsvector; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_extraction_pages_content_tsvector ON public.extraction_pages USING gin (to_tsvector('english'::regconfig, content));


--
-- Name: idx_extraction_pages_site_url; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_extraction_pages_site_url ON public.extraction_pages USING btree (site_url);


--
-- Name: idx_extraction_summaries_prompt_hash; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_extraction_summaries_prompt_hash ON public.extraction_summaries USING btree (prompt_hash);


--
-- Name: idx_extraction_summaries_site_url; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_extraction_summaries_site_url ON public.extraction_summaries USING btree (site_url);


--
-- Name: idx_identifiers_member_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_identifiers_member_id ON public.identifiers USING btree (member_id);


--
-- Name: idx_identifiers_phone_hash; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_identifiers_phone_hash ON public.identifiers USING btree (phone_hash);


--
-- Name: idx_jobs_container_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_jobs_container_id ON public.jobs USING btree (container_id) WHERE (container_id IS NOT NULL);


--
-- Name: idx_jobs_dead_letter; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_jobs_dead_letter ON public.jobs USING btree (status) WHERE (status = 'dead_letter'::text);


--
-- Name: idx_jobs_enabled; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_jobs_enabled ON public.jobs USING btree (enabled) WHERE (enabled = true);


--
-- Name: idx_jobs_idempotency_key; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_jobs_idempotency_key ON public.jobs USING btree (idempotency_key) WHERE ((idempotency_key IS NOT NULL) AND (status = ANY (ARRAY['pending'::text, 'running'::text])));


--
-- Name: idx_jobs_job_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_jobs_job_type ON public.jobs USING btree (job_type);


--
-- Name: idx_jobs_reference_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_jobs_reference_id ON public.jobs USING btree (reference_id) WHERE (reference_id IS NOT NULL);


--
-- Name: idx_jobs_status_next_run; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_jobs_status_next_run ON public.jobs USING btree (status, next_run_at) WHERE (status = 'pending'::text);


--
-- Name: idx_jobs_workflow_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_jobs_workflow_id ON public.jobs USING btree (workflow_id) WHERE (workflow_id IS NOT NULL);


--
-- Name: idx_listing_contacts_listing; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_listing_contacts_listing ON public.listing_contacts USING btree (listing_id);


--
-- Name: idx_listing_page_sources_listing; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_listing_page_sources_listing ON public.post_page_sources USING btree (post_id);


--
-- Name: idx_listing_page_sources_snapshot; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_listing_page_sources_snapshot ON public.post_page_sources USING btree (page_snapshot_id);


--
-- Name: idx_listing_reports_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_listing_reports_created ON public.listing_reports USING btree (created_at DESC);


--
-- Name: idx_listing_reports_listing; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_listing_reports_listing ON public.listing_reports USING btree (listing_id);


--
-- Name: idx_listing_reports_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_listing_reports_status ON public.listing_reports USING btree (status) WHERE (status = 'pending'::text);


--
-- Name: idx_listings_capacity; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_listings_capacity ON public.posts USING btree (capacity_status);


--
-- Name: idx_listings_category; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_listings_category ON public.posts USING btree (category);


--
-- Name: idx_listings_disappeared_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_listings_disappeared_at ON public.posts USING btree (disappeared_at) WHERE (disappeared_at IS NULL);


--
-- Name: idx_listings_language; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_listings_language ON public.posts USING btree (source_language);


--
-- Name: idx_listings_page_snapshot_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_listings_page_snapshot_id ON public.posts USING btree (page_snapshot_id) WHERE (page_snapshot_id IS NOT NULL);


--
-- Name: idx_listings_urgency; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_listings_urgency ON public.posts USING btree (urgency) WHERE (urgency IS NOT NULL);


--
-- Name: idx_listings_verified; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_listings_verified ON public.posts USING btree (verified_at) WHERE (verified_at IS NOT NULL);


--
-- Name: idx_locations_city_state; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_locations_city_state ON public.locations USING btree (city, state);


--
-- Name: idx_locations_postal; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_locations_postal ON public.locations USING btree (postal_code) WHERE (postal_code IS NOT NULL);


--
-- Name: idx_locations_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_locations_type ON public.locations USING btree (location_type);


--
-- Name: idx_members_active; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_members_active ON public.members USING btree (active) WHERE (active = true);


--
-- Name: idx_members_embedding; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_members_embedding ON public.members USING hnsw (embedding public.vector_cosine_ops) WITH (m='16', ef_construction='64');


--
-- Name: INDEX idx_members_embedding; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON INDEX public.idx_members_embedding IS 'HNSW index with m=16 (neighbors per layer), ef_construction=64 (build quality). Optimized for 100K-1M records.';


--
-- Name: idx_members_lat; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_members_lat ON public.members USING btree (latitude) WHERE (latitude IS NOT NULL);


--
-- Name: idx_members_lng; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_members_lng ON public.members USING btree (longitude) WHERE (longitude IS NOT NULL);


--
-- Name: idx_members_token; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_members_token ON public.members USING btree (expo_push_token);


--
-- Name: idx_messages_author; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_messages_author ON public.messages USING btree (author_id);


--
-- Name: idx_messages_container; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_messages_container ON public.messages USING btree (container_id, sequence_number);


--
-- Name: idx_messages_moderation; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_messages_moderation ON public.messages USING btree (moderation_status);


--
-- Name: idx_messages_parent; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_messages_parent ON public.messages USING btree (parent_message_id);


--
-- Name: idx_migration_workflows_name; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_migration_workflows_name ON public.migration_workflows USING btree (name);


--
-- Name: idx_migration_workflows_phase; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_migration_workflows_phase ON public.migration_workflows USING btree (phase) WHERE (phase = ANY (ARRAY['running'::text, 'paused'::text]));


--
-- Name: idx_needs_lat; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_needs_lat ON public.posts USING btree (latitude) WHERE (latitude IS NOT NULL);


--
-- Name: idx_needs_lng; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_needs_lng ON public.posts USING btree (longitude) WHERE (longitude IS NOT NULL);


--
-- Name: idx_needs_submitted_by_member; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_needs_submitted_by_member ON public.posts USING btree (submitted_by_member_id) WHERE (submitted_by_member_id IS NOT NULL);


--
-- Name: idx_noteables_entity; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_noteables_entity ON public.noteables USING btree (noteable_type, noteable_id);


--
-- Name: idx_noteables_note; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_noteables_note ON public.noteables USING btree (note_id);


--
-- Name: idx_notes_expired_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_notes_expired_at ON public.notes USING btree (expired_at) WHERE (expired_at IS NULL);


--
-- Name: idx_notes_is_public; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_notes_is_public ON public.notes USING btree (is_public) WHERE (is_public = true);


--
-- Name: idx_notes_severity; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_notes_severity ON public.notes USING btree (severity);


--
-- Name: idx_notes_source; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_notes_source ON public.notes USING btree (source_type, source_id) WHERE (source_id IS NOT NULL);


--
-- Name: idx_organization_needs_fingerprint; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_organization_needs_fingerprint ON public.posts USING btree (fingerprint) WHERE (fingerprint IS NOT NULL);


--
-- Name: idx_organization_needs_source_url; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_organization_needs_source_url ON public.posts USING btree (source_url);


--
-- Name: idx_organization_needs_submitted_by; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_organization_needs_submitted_by ON public.posts USING btree (submitted_by_member_id) WHERE (submitted_by_member_id IS NOT NULL);


--
-- Name: idx_organization_tags_kind_value; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_organization_tags_kind_value ON public.organization_tags USING btree (kind, value);


--
-- Name: idx_organization_tags_org_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_organization_tags_org_id ON public.organization_tags USING btree (organization_id);


--
-- Name: idx_organization_tags_value; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_organization_tags_value ON public.organization_tags USING btree (value) WHERE (kind = 'service'::text);


--
-- Name: idx_organizations_name; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_organizations_name ON public.organizations USING btree (name);


--
-- Name: idx_organizations_pending; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_organizations_pending ON public.organizations USING btree (status) WHERE (status = 'pending_review'::text);


--
-- Name: idx_organizations_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_organizations_status ON public.organizations USING btree (status);


--
-- Name: idx_page_extractions_current; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_page_extractions_current ON public.page_extractions USING btree (page_snapshot_id, extraction_type) WHERE (is_current = true);


--
-- Name: idx_page_extractions_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_page_extractions_type ON public.page_extractions USING btree (extraction_type, created_at DESC);


--
-- Name: idx_page_extractions_unique_current; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_page_extractions_unique_current ON public.page_extractions USING btree (page_snapshot_id, extraction_type) WHERE (is_current = true);


--
-- Name: idx_page_snapshots_content_hash; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_page_snapshots_content_hash ON public.page_snapshots USING btree (content_hash);


--
-- Name: idx_page_snapshots_crawled_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_page_snapshots_crawled_at ON public.page_snapshots USING btree (crawled_at DESC);


--
-- Name: idx_page_snapshots_extraction_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_page_snapshots_extraction_status ON public.page_snapshots USING btree (extraction_status);


--
-- Name: idx_page_snapshots_url; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_page_snapshots_url ON public.page_snapshots USING btree (url);


--
-- Name: idx_page_summaries_content_hash; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_page_summaries_content_hash ON public.page_summaries USING btree (content_hash);


--
-- Name: idx_page_summaries_snapshot_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_page_summaries_snapshot_id ON public.page_summaries USING btree (page_snapshot_id);


--
-- Name: idx_post_locations_location; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_post_locations_location ON public.post_locations USING btree (location_id);


--
-- Name: idx_post_locations_post; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_post_locations_post ON public.post_locations USING btree (post_id);


--
-- Name: idx_post_sources_active; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_post_sources_active ON public.post_sources USING btree (source_type, source_id) WHERE (disappeared_at IS NULL);


--
-- Name: idx_post_sources_post_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_post_sources_post_id ON public.post_sources USING btree (post_id);


--
-- Name: idx_post_sources_source; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_post_sources_source ON public.post_sources USING btree (source_type, source_id);


--
-- Name: idx_posts_comments_container; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_posts_comments_container ON public.posts USING btree (comments_container_id) WHERE (comments_container_id IS NOT NULL);


--
-- Name: idx_posts_deleted_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_posts_deleted_at ON public.posts USING btree (deleted_at) WHERE (deleted_at IS NULL);


--
-- Name: idx_posts_duplicate_of_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_posts_duplicate_of_id ON public.posts USING btree (duplicate_of_id) WHERE (duplicate_of_id IS NOT NULL);


--
-- Name: idx_posts_embedding; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_posts_embedding ON public.posts USING hnsw (embedding public.vector_cosine_ops) WITH (m='16', ef_construction='64');


--
-- Name: idx_posts_revision_of_post_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_posts_revision_of_post_id ON public.posts USING btree (revision_of_post_id);


--
-- Name: idx_posts_submitted_by_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_posts_submitted_by_id ON public.posts USING btree (submitted_by_id) WHERE (deleted_at IS NULL);


--
-- Name: idx_posts_translation_of_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_posts_translation_of_id ON public.posts USING btree (translation_of_id);


--
-- Name: idx_posts_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_posts_type ON public.posts USING btree (post_type);


--
-- Name: idx_providers_accepting; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_providers_accepting ON public.providers USING btree (accepting_clients) WHERE ((status = 'approved'::text) AND (accepting_clients = true));


--
-- Name: idx_providers_embedding; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_providers_embedding ON public.providers USING hnsw (embedding public.vector_cosine_ops) WHERE ((embedding IS NOT NULL) AND (status = 'approved'::text));


--
-- Name: idx_providers_member; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_providers_member ON public.providers USING btree (member_id) WHERE (member_id IS NOT NULL);


--
-- Name: idx_providers_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_providers_status ON public.providers USING btree (status);


--
-- Name: idx_schedules_day; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_schedules_day ON public.schedules USING btree (day_of_week);


--
-- Name: idx_schedules_schedulable; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_schedules_schedulable ON public.schedules USING btree (schedulable_type, schedulable_id);


--
-- Name: idx_scrape_jobs_source_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_scrape_jobs_source_id ON public.scrape_jobs USING btree (source_id);


--
-- Name: idx_scrape_jobs_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_scrape_jobs_status ON public.scrape_jobs USING btree (status);


--
-- Name: idx_seesaw_effects_poll; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_seesaw_effects_poll ON public.seesaw_effect_executions USING btree (priority, execute_at, event_id, effect_id) WHERE (status = 'pending'::text);


--
-- Name: idx_seesaw_effects_saga; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_seesaw_effects_saga ON public.seesaw_effect_executions USING btree (correlation_id, status);


--
-- Name: idx_seesaw_events_idempotency; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_seesaw_events_idempotency ON ONLY public.seesaw_events USING btree (event_id, created_at);


--
-- Name: idx_seesaw_events_poll; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_seesaw_events_poll ON ONLY public.seesaw_events USING btree (correlation_id, created_at, id) WHERE (processed_at IS NULL);


--
-- Name: idx_service_listings_evening; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_service_listings_evening ON public.service_listings USING btree (evening_hours) WHERE (evening_hours = true);


--
-- Name: idx_service_listings_free; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_service_listings_free ON public.service_listings USING btree (free_service) WHERE (free_service = true);


--
-- Name: idx_service_listings_remote; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_service_listings_remote ON public.service_listings USING btree (remote_available) WHERE (remote_available = true);


--
-- Name: idx_service_listings_wheelchair; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_service_listings_wheelchair ON public.service_listings USING btree (wheelchair_accessible) WHERE (wheelchair_accessible = true);


--
-- Name: idx_social_sources_handle; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_social_sources_handle ON public.social_sources USING btree (source_type, handle);


--
-- Name: idx_social_sources_source_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_social_sources_source_id ON public.social_sources USING btree (source_id);


--
-- Name: idx_sources_active_due; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sources_active_due ON public.sources USING btree (active, status, last_scraped_at) WHERE ((active = true) AND (status = 'approved'::text));


--
-- Name: idx_sources_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sources_created_at ON public.sources USING btree (created_at);


--
-- Name: idx_sources_organization_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sources_organization_id ON public.sources USING btree (organization_id);


--
-- Name: idx_sources_source_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sources_source_type ON public.sources USING btree (source_type);


--
-- Name: idx_sources_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sources_status ON public.sources USING btree (status);


--
-- Name: idx_sync_batches_resource_source; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sync_batches_resource_source ON public.sync_batches USING btree (resource_type, source_id);


--
-- Name: idx_sync_batches_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sync_batches_status ON public.sync_batches USING btree (status);


--
-- Name: idx_sync_proposal_merge_sources_proposal; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sync_proposal_merge_sources_proposal ON public.sync_proposal_merge_sources USING btree (proposal_id);


--
-- Name: idx_sync_proposals_batch_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sync_proposals_batch_id ON public.sync_proposals USING btree (batch_id);


--
-- Name: idx_sync_proposals_entity; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sync_proposals_entity ON public.sync_proposals USING btree (entity_type, target_entity_id);


--
-- Name: idx_sync_proposals_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sync_proposals_status ON public.sync_proposals USING btree (status);


--
-- Name: idx_taggables_entity; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_taggables_entity ON public.taggables USING btree (taggable_type, taggable_id);


--
-- Name: idx_taggables_tag; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_taggables_tag ON public.taggables USING btree (tag_id);


--
-- Name: idx_taggables_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_taggables_type ON public.taggables USING btree (taggable_type);


--
-- Name: idx_tags_kind; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_tags_kind ON public.tags USING btree (kind);


--
-- Name: idx_tags_parent; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_tags_parent ON public.tags USING btree (parent_tag_id) WHERE (parent_tag_id IS NOT NULL);


--
-- Name: idx_tags_value; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_tags_value ON public.tags USING btree (value);


--
-- Name: idx_tavily_search_queries_website_research_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_tavily_search_queries_website_research_id ON public.tavily_search_queries USING btree (website_research_id);


--
-- Name: idx_tavily_search_results_query_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_tavily_search_results_query_id ON public.tavily_search_results USING btree (query_id);


--
-- Name: idx_tavily_search_results_score; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_tavily_search_results_score ON public.tavily_search_results USING btree (score DESC);


--
-- Name: idx_website_assessments_embedding; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_website_assessments_embedding ON public.website_assessments USING ivfflat (embedding public.vector_cosine_ops) WITH (lists='100');


--
-- Name: idx_website_assessments_website_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_website_assessments_website_id ON public.website_assessments USING btree (website_id);


--
-- Name: idx_website_assessments_website_research_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_website_assessments_website_research_id ON public.website_assessments USING btree (website_research_id);


--
-- Name: idx_website_research_homepage_research_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_website_research_homepage_research_id ON public.website_research_homepage USING btree (website_research_id);


--
-- Name: idx_website_research_website_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_website_research_website_id ON public.website_research USING btree (website_id);


--
-- Name: idx_website_snapshots_last_synced_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_website_snapshots_last_synced_at ON public.website_snapshots USING btree (last_synced_at) WHERE (last_synced_at IS NOT NULL);


--
-- Name: idx_website_snapshots_website_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_website_snapshots_website_id ON public.website_snapshots USING btree (website_id);


--
-- Name: idx_website_sources_domain; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_website_sources_domain ON public.website_sources USING btree (domain);


--
-- Name: idx_website_sources_source_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_website_sources_source_id ON public.website_sources USING btree (source_id);


--
-- Name: idx_wsl_listing; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_wsl_listing ON public.website_snapshot_listings USING btree (listing_id);


--
-- Name: idx_wsl_snapshot; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_wsl_snapshot ON public.website_snapshot_listings USING btree (website_snapshot_id);


--
-- Name: idx_zip_codes_city_state; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_zip_codes_city_state ON public.zip_codes USING btree (city, state);


--
-- Name: idx_zip_codes_state; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_zip_codes_state ON public.zip_codes USING btree (state);


--
-- Name: seesaw_events_default_correlation_id_created_at_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX seesaw_events_default_correlation_id_created_at_id_idx ON public.seesaw_events_default USING btree (correlation_id, created_at, id) WHERE (processed_at IS NULL);


--
-- Name: seesaw_events_default_event_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX seesaw_events_default_event_id_created_at_idx ON public.seesaw_events_default USING btree (event_id, created_at);


--
-- Name: seesaw_events_default_correlation_id_created_at_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_seesaw_events_poll ATTACH PARTITION public.seesaw_events_default_correlation_id_created_at_id_idx;


--
-- Name: seesaw_events_default_event_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_seesaw_events_idempotency ATTACH PARTITION public.seesaw_events_default_event_id_created_at_idx;


--
-- Name: seesaw_events_default_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.seesaw_events_pkey ATTACH PARTITION public.seesaw_events_default_pkey;


--
-- Name: website_snapshots_with_listings _RETURN; Type: RULE; Schema: public; Owner: -
--

CREATE OR REPLACE VIEW public.website_snapshots_with_listings AS
 SELECT ws.id,
    ws.website_id,
    ws.page_url,
    ws.page_snapshot_id,
    ws.submitted_by,
    ws.submitted_at,
    ws.last_scraped_at,
    ws.scrape_status,
    ws.scrape_error,
    ws.created_at,
    ws.updated_at,
    (count(wsl.listing_id) > 0) AS has_listings,
    count(wsl.listing_id) AS listings_count
   FROM (public.website_snapshots ws
     LEFT JOIN public.website_snapshot_listings wsl ON ((ws.id = wsl.website_snapshot_id)))
  GROUP BY ws.id;


--
-- Name: seesaw_events seesaw_events_notify; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER seesaw_events_notify AFTER INSERT ON public.seesaw_events FOR EACH ROW EXECUTE FUNCTION public.seesaw_notify_saga();


--
-- Name: identifiers trigger_update_identifiers_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trigger_update_identifiers_updated_at BEFORE UPDATE ON public.identifiers FOR EACH ROW EXECUTE FUNCTION public.update_identifiers_updated_at();


--
-- Name: posts trigger_update_page_snapshot_count; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trigger_update_page_snapshot_count AFTER INSERT OR DELETE OR UPDATE OF page_snapshot_id ON public.posts FOR EACH ROW EXECUTE FUNCTION public.update_page_snapshot_listings_count();


--
-- Name: scrape_jobs trigger_update_scrape_jobs_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trigger_update_scrape_jobs_updated_at BEFORE UPDATE ON public.scrape_jobs FOR EACH ROW EXECUTE FUNCTION public.update_scrape_jobs_updated_at();


--
-- Name: agent_assistant_configs agent_assistant_configs_agent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.agent_assistant_configs
    ADD CONSTRAINT agent_assistant_configs_agent_id_fkey FOREIGN KEY (agent_id) REFERENCES public.agents(id) ON DELETE CASCADE;


--
-- Name: agents agents_member_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.agents
    ADD CONSTRAINT agents_member_id_fkey FOREIGN KEY (member_id) REFERENCES public.members(id) ON DELETE CASCADE;


--
-- Name: business_listings business_listings_listing_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.business_listings
    ADD CONSTRAINT business_listings_listing_id_fkey FOREIGN KEY (listing_id) REFERENCES public.posts(id) ON DELETE CASCADE;


--
-- Name: containers chatrooms_language_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.containers
    ADD CONSTRAINT chatrooms_language_fkey FOREIGN KEY (language) REFERENCES public.active_languages(language_code);


--
-- Name: document_references document_references_document_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.document_references
    ADD CONSTRAINT document_references_document_id_fkey FOREIGN KEY (document_id) REFERENCES public.referral_documents(id) ON DELETE CASCADE;


--
-- Name: website_assessments domain_assessments_generated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_assessments
    ADD CONSTRAINT domain_assessments_generated_by_fkey FOREIGN KEY (generated_by) REFERENCES public.members(id);


--
-- Name: website_research domain_research_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_research
    ADD CONSTRAINT domain_research_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.members(id);


--
-- Name: website_snapshots domain_snapshots_page_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_snapshots
    ADD CONSTRAINT domain_snapshots_page_snapshot_id_fkey FOREIGN KEY (page_snapshot_id) REFERENCES public.page_snapshots(id) ON DELETE SET NULL;


--
-- Name: website_snapshots domain_snapshots_submitted_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_snapshots
    ADD CONSTRAINT domain_snapshots_submitted_by_fkey FOREIGN KEY (submitted_by) REFERENCES public.members(id) ON DELETE SET NULL;


--
-- Name: identifiers fk_member; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.identifiers
    ADD CONSTRAINT fk_member FOREIGN KEY (member_id) REFERENCES public.members(id) ON DELETE CASCADE;


--
-- Name: listing_contacts listing_contacts_listing_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.listing_contacts
    ADD CONSTRAINT listing_contacts_listing_id_fkey FOREIGN KEY (listing_id) REFERENCES public.posts(id) ON DELETE CASCADE;


--
-- Name: listing_delivery_modes listing_delivery_modes_listing_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.listing_delivery_modes
    ADD CONSTRAINT listing_delivery_modes_listing_id_fkey FOREIGN KEY (listing_id) REFERENCES public.posts(id) ON DELETE CASCADE;


--
-- Name: post_page_sources listing_page_sources_listing_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.post_page_sources
    ADD CONSTRAINT listing_page_sources_listing_id_fkey FOREIGN KEY (post_id) REFERENCES public.posts(id) ON DELETE CASCADE;


--
-- Name: post_page_sources listing_page_sources_page_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.post_page_sources
    ADD CONSTRAINT listing_page_sources_page_snapshot_id_fkey FOREIGN KEY (page_snapshot_id) REFERENCES public.page_snapshots(id) ON DELETE CASCADE;


--
-- Name: listing_reports listing_reports_listing_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.listing_reports
    ADD CONSTRAINT listing_reports_listing_id_fkey FOREIGN KEY (listing_id) REFERENCES public.posts(id) ON DELETE CASCADE;


--
-- Name: listing_reports listing_reports_reported_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.listing_reports
    ADD CONSTRAINT listing_reports_reported_by_fkey FOREIGN KEY (reported_by) REFERENCES public.members(id) ON DELETE SET NULL;


--
-- Name: listing_reports listing_reports_resolved_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.listing_reports
    ADD CONSTRAINT listing_reports_resolved_by_fkey FOREIGN KEY (resolved_by) REFERENCES public.members(id) ON DELETE SET NULL;


--
-- Name: posts listings_page_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.posts
    ADD CONSTRAINT listings_page_snapshot_id_fkey FOREIGN KEY (page_snapshot_id) REFERENCES public.page_snapshots(id) ON DELETE SET NULL;


--
-- Name: messages messages_author_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.messages
    ADD CONSTRAINT messages_author_id_fkey FOREIGN KEY (author_id) REFERENCES public.members(id) ON DELETE SET NULL;


--
-- Name: messages messages_container_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.messages
    ADD CONSTRAINT messages_container_id_fkey FOREIGN KEY (container_id) REFERENCES public.containers(id) ON DELETE CASCADE;


--
-- Name: messages messages_parent_message_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.messages
    ADD CONSTRAINT messages_parent_message_id_fkey FOREIGN KEY (parent_message_id) REFERENCES public.messages(id) ON DELETE CASCADE;


--
-- Name: noteables noteables_note_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.noteables
    ADD CONSTRAINT noteables_note_id_fkey FOREIGN KEY (note_id) REFERENCES public.notes(id) ON DELETE CASCADE;


--
-- Name: opportunity_listings opportunity_listings_listing_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.opportunity_listings
    ADD CONSTRAINT opportunity_listings_listing_id_fkey FOREIGN KEY (listing_id) REFERENCES public.posts(id) ON DELETE CASCADE;


--
-- Name: posts organization_needs_submitted_by_volunteer_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.posts
    ADD CONSTRAINT organization_needs_submitted_by_volunteer_id_fkey FOREIGN KEY (submitted_by_member_id) REFERENCES public.members(id) ON DELETE SET NULL;


--
-- Name: page_extractions page_extractions_page_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.page_extractions
    ADD CONSTRAINT page_extractions_page_snapshot_id_fkey FOREIGN KEY (page_snapshot_id) REFERENCES public.page_snapshots(id) ON DELETE CASCADE;


--
-- Name: page_summaries page_summaries_page_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.page_summaries
    ADD CONSTRAINT page_summaries_page_snapshot_id_fkey FOREIGN KEY (page_snapshot_id) REFERENCES public.page_snapshots(id) ON DELETE CASCADE;


--
-- Name: post_locations post_locations_location_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.post_locations
    ADD CONSTRAINT post_locations_location_id_fkey FOREIGN KEY (location_id) REFERENCES public.locations(id) ON DELETE CASCADE;


--
-- Name: post_locations post_locations_post_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.post_locations
    ADD CONSTRAINT post_locations_post_id_fkey FOREIGN KEY (post_id) REFERENCES public.posts(id) ON DELETE CASCADE;


--
-- Name: post_sources post_sources_post_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.post_sources
    ADD CONSTRAINT post_sources_post_id_fkey FOREIGN KEY (post_id) REFERENCES public.posts(id) ON DELETE CASCADE;


--
-- Name: posts posts_comments_container_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.posts
    ADD CONSTRAINT posts_comments_container_id_fkey FOREIGN KEY (comments_container_id) REFERENCES public.containers(id);


--
-- Name: posts posts_duplicate_of_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.posts
    ADD CONSTRAINT posts_duplicate_of_id_fkey FOREIGN KEY (duplicate_of_id) REFERENCES public.posts(id) ON DELETE SET NULL;


--
-- Name: posts posts_revision_of_post_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.posts
    ADD CONSTRAINT posts_revision_of_post_id_fkey FOREIGN KEY (revision_of_post_id) REFERENCES public.posts(id);


--
-- Name: posts posts_submitted_by_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.posts
    ADD CONSTRAINT posts_submitted_by_id_fkey FOREIGN KEY (submitted_by_id) REFERENCES public.members(id);


--
-- Name: posts posts_translation_of_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.posts
    ADD CONSTRAINT posts_translation_of_id_fkey FOREIGN KEY (translation_of_id) REFERENCES public.posts(id);


--
-- Name: providers providers_member_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.providers
    ADD CONSTRAINT providers_member_id_fkey FOREIGN KEY (member_id) REFERENCES public.members(id) ON DELETE SET NULL;


--
-- Name: providers providers_reviewed_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.providers
    ADD CONSTRAINT providers_reviewed_by_fkey FOREIGN KEY (reviewed_by) REFERENCES public.members(id);


--
-- Name: providers providers_source_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.providers
    ADD CONSTRAINT providers_source_id_fkey FOREIGN KEY (source_id) REFERENCES public.sources(id) ON DELETE SET NULL;


--
-- Name: providers providers_submitted_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.providers
    ADD CONSTRAINT providers_submitted_by_fkey FOREIGN KEY (submitted_by) REFERENCES public.members(id);


--
-- Name: referral_document_translations referral_document_translations_document_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.referral_document_translations
    ADD CONSTRAINT referral_document_translations_document_id_fkey FOREIGN KEY (document_id) REFERENCES public.referral_documents(id) ON DELETE CASCADE;


--
-- Name: referral_document_translations referral_document_translations_language_code_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.referral_document_translations
    ADD CONSTRAINT referral_document_translations_language_code_fkey FOREIGN KEY (language_code) REFERENCES public.active_languages(language_code);


--
-- Name: referral_documents referral_documents_container_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.referral_documents
    ADD CONSTRAINT referral_documents_container_id_fkey FOREIGN KEY (container_id) REFERENCES public.containers(id);


--
-- Name: referral_documents referral_documents_source_language_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.referral_documents
    ADD CONSTRAINT referral_documents_source_language_fkey FOREIGN KEY (source_language) REFERENCES public.active_languages(language_code);


--
-- Name: service_listings service_listings_listing_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.service_listings
    ADD CONSTRAINT service_listings_listing_id_fkey FOREIGN KEY (listing_id) REFERENCES public.posts(id) ON DELETE CASCADE;


--
-- Name: social_sources social_sources_source_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.social_sources
    ADD CONSTRAINT social_sources_source_id_fkey FOREIGN KEY (source_id) REFERENCES public.sources(id) ON DELETE CASCADE;


--
-- Name: sources sources_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sources
    ADD CONSTRAINT sources_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE SET NULL;


--
-- Name: sync_proposal_merge_sources sync_proposal_merge_sources_proposal_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sync_proposal_merge_sources
    ADD CONSTRAINT sync_proposal_merge_sources_proposal_id_fkey FOREIGN KEY (proposal_id) REFERENCES public.sync_proposals(id) ON DELETE CASCADE;


--
-- Name: sync_proposals sync_proposals_batch_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sync_proposals
    ADD CONSTRAINT sync_proposals_batch_id_fkey FOREIGN KEY (batch_id) REFERENCES public.sync_batches(id) ON DELETE CASCADE;


--
-- Name: sync_proposals sync_proposals_reviewed_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sync_proposals
    ADD CONSTRAINT sync_proposals_reviewed_by_fkey FOREIGN KEY (reviewed_by) REFERENCES public.members(id);


--
-- Name: taggables taggables_tag_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.taggables
    ADD CONSTRAINT taggables_tag_id_fkey FOREIGN KEY (tag_id) REFERENCES public.tags(id) ON DELETE CASCADE;


--
-- Name: tags tags_parent_tag_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tags
    ADD CONSTRAINT tags_parent_tag_id_fkey FOREIGN KEY (parent_tag_id) REFERENCES public.tags(id) ON DELETE SET NULL;


--
-- Name: tavily_search_queries tavily_search_queries_website_research_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tavily_search_queries
    ADD CONSTRAINT tavily_search_queries_website_research_id_fkey FOREIGN KEY (website_research_id) REFERENCES public.website_research(id) ON DELETE CASCADE;


--
-- Name: tavily_search_results tavily_search_results_query_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tavily_search_results
    ADD CONSTRAINT tavily_search_results_query_id_fkey FOREIGN KEY (query_id) REFERENCES public.tavily_search_queries(id) ON DELETE CASCADE;


--
-- Name: taxonomy_crosswalks taxonomy_crosswalks_tag_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.taxonomy_crosswalks
    ADD CONSTRAINT taxonomy_crosswalks_tag_id_fkey FOREIGN KEY (tag_id) REFERENCES public.tags(id) ON DELETE CASCADE;


--
-- Name: website_assessments website_assessments_source_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_assessments
    ADD CONSTRAINT website_assessments_source_id_fkey FOREIGN KEY (website_id) REFERENCES public.website_sources(source_id) ON DELETE CASCADE;


--
-- Name: website_assessments website_assessments_website_research_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_assessments
    ADD CONSTRAINT website_assessments_website_research_id_fkey FOREIGN KEY (website_research_id) REFERENCES public.website_research(id) ON DELETE SET NULL;


--
-- Name: website_research_homepage website_research_homepage_website_research_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_research_homepage
    ADD CONSTRAINT website_research_homepage_website_research_id_fkey FOREIGN KEY (website_research_id) REFERENCES public.website_research(id) ON DELETE CASCADE;


--
-- Name: website_research website_research_source_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_research
    ADD CONSTRAINT website_research_source_id_fkey FOREIGN KEY (website_id) REFERENCES public.website_sources(source_id) ON DELETE CASCADE;


--
-- Name: website_snapshot_listings website_snapshot_listings_listing_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_snapshot_listings
    ADD CONSTRAINT website_snapshot_listings_listing_id_fkey FOREIGN KEY (listing_id) REFERENCES public.posts(id) ON DELETE CASCADE;


--
-- Name: website_snapshot_listings website_snapshot_listings_website_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_snapshot_listings
    ADD CONSTRAINT website_snapshot_listings_website_snapshot_id_fkey FOREIGN KEY (website_snapshot_id) REFERENCES public.website_snapshots(id) ON DELETE CASCADE;


--
-- Name: website_snapshots website_snapshots_source_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_snapshots
    ADD CONSTRAINT website_snapshots_source_id_fkey FOREIGN KEY (website_id) REFERENCES public.website_sources(source_id) ON DELETE CASCADE;


--
-- Name: website_sources website_sources_source_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.website_sources
    ADD CONSTRAINT website_sources_source_id_fkey FOREIGN KEY (source_id) REFERENCES public.sources(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict ohRKeLUWI5wSJbDPr5vMBj7TP5w2WAiMJa0IV9rtzpJkxneT52B2Fug7fP88AHa

