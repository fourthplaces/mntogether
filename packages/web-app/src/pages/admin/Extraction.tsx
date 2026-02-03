import { useState } from 'react';
import { useMutation } from '@apollo/client';
import { SUBMIT_URL, TRIGGER_EXTRACTION, INGEST_SITE } from '../../graphql/mutations';

// Types matching GraphQL schema
interface Source {
  url: string;
  title: string | null;
  fetchedAt: string;
  role: 'PRIMARY' | 'SUPPORTING' | 'CORROBORATING';
}

interface Gap {
  field: string;
  suggestedQuery: string;
  isSearchable: boolean;
}

interface ConflictingClaim {
  statement: string;
  sourceUrl: string;
}

interface Conflict {
  topic: string;
  claims: ConflictingClaim[];
}

interface Extraction {
  content: string;
  status: 'FOUND' | 'PARTIAL' | 'MISSING' | 'CONTRADICTORY';
  grounding: 'VERIFIED' | 'SINGLE_SOURCE' | 'CONFLICTED' | 'INFERRED';
  sources: Source[];
  gaps: Gap[];
  conflicts: Conflict[];
}

interface SubmitUrlResult {
  success: boolean;
  url: string;
  extraction: Extraction | null;
  error: string | null;
}

interface TriggerExtractionResult {
  success: boolean;
  query: string;
  site: string | null;
  extractions: Extraction[];
  error: string | null;
}

interface IngestSiteResult {
  siteUrl: string;
  pagesCrawled: number;
  pagesSummarized: number;
  pagesSkipped: number;
}

// Status badge component
function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, string> = {
    FOUND: 'bg-green-100 text-green-800',
    PARTIAL: 'bg-yellow-100 text-yellow-800',
    MISSING: 'bg-red-100 text-red-800',
    CONTRADICTORY: 'bg-purple-100 text-purple-800',
    VERIFIED: 'bg-green-100 text-green-800',
    SINGLE_SOURCE: 'bg-blue-100 text-blue-800',
    CONFLICTED: 'bg-orange-100 text-orange-800',
    INFERRED: 'bg-gray-100 text-gray-800',
  };

  return (
    <span className={`px-2 py-1 text-xs font-medium rounded-full ${colors[status] || 'bg-gray-100 text-gray-800'}`}>
      {status.replace('_', ' ')}
    </span>
  );
}

// Source role badge
function RoleBadge({ role }: { role: string }) {
  const colors: Record<string, string> = {
    PRIMARY: 'bg-blue-500 text-white',
    SUPPORTING: 'bg-gray-200 text-gray-700',
    CORROBORATING: 'bg-green-200 text-green-700',
  };

  return (
    <span className={`px-2 py-0.5 text-xs font-medium rounded ${colors[role] || 'bg-gray-100'}`}>
      {role}
    </span>
  );
}

// Extraction result display component
function ExtractionResult({ extraction }: { extraction: Extraction }) {
  const [showSources, setShowSources] = useState(false);
  const [showGaps, setShowGaps] = useState(false);
  const [showConflicts, setShowConflicts] = useState(false);

  return (
    <div className="bg-white rounded-lg border border-stone-200 p-4 space-y-4">
      {/* Status indicators */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2">
          <span className="text-sm text-stone-500">Status:</span>
          <StatusBadge status={extraction.status} />
        </div>
        <div className="flex items-center gap-2">
          <span className="text-sm text-stone-500">Grounding:</span>
          <StatusBadge status={extraction.grounding} />
        </div>
      </div>

      {/* Content */}
      <div className="prose prose-sm max-w-none">
        <div className="bg-stone-50 rounded-lg p-4 whitespace-pre-wrap text-stone-800">
          {extraction.content}
        </div>
      </div>

      {/* Sources */}
      {extraction.sources.length > 0 && (
        <div>
          <button
            onClick={() => setShowSources(!showSources)}
            className="flex items-center gap-2 text-sm font-medium text-stone-700 hover:text-stone-900"
          >
            <span>{showSources ? '▼' : '▶'}</span>
            <span>Sources ({extraction.sources.length})</span>
          </button>
          {showSources && (
            <div className="mt-2 space-y-2">
              {extraction.sources.map((source, idx) => (
                <div key={idx} className="flex items-start gap-3 p-2 bg-stone-50 rounded">
                  <RoleBadge role={source.role} />
                  <div className="flex-1 min-w-0">
                    <a
                      href={source.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-sm text-blue-600 hover:underline truncate block"
                    >
                      {source.title || source.url}
                    </a>
                    <p className="text-xs text-stone-500 truncate">{source.url}</p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Gaps */}
      {extraction.gaps.length > 0 && (
        <div>
          <button
            onClick={() => setShowGaps(!showGaps)}
            className="flex items-center gap-2 text-sm font-medium text-amber-700 hover:text-amber-900"
          >
            <span>{showGaps ? '▼' : '▶'}</span>
            <span>Information Gaps ({extraction.gaps.length})</span>
          </button>
          {showGaps && (
            <div className="mt-2 space-y-2">
              {extraction.gaps.map((gap, idx) => (
                <div key={idx} className="p-2 bg-amber-50 rounded border border-amber-200">
                  <p className="text-sm font-medium text-amber-800">{gap.field}</p>
                  <p className="text-xs text-amber-600 mt-1">
                    Suggested search: "{gap.suggestedQuery}"
                    {gap.isSearchable && (
                      <span className="ml-2 text-green-600">(searchable)</span>
                    )}
                  </p>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Conflicts */}
      {extraction.conflicts.length > 0 && (
        <div>
          <button
            onClick={() => setShowConflicts(!showConflicts)}
            className="flex items-center gap-2 text-sm font-medium text-red-700 hover:text-red-900"
          >
            <span>{showConflicts ? '▼' : '▶'}</span>
            <span>Conflicts ({extraction.conflicts.length})</span>
          </button>
          {showConflicts && (
            <div className="mt-2 space-y-3">
              {extraction.conflicts.map((conflict, idx) => (
                <div key={idx} className="p-3 bg-red-50 rounded border border-red-200">
                  <p className="text-sm font-medium text-red-800 mb-2">{conflict.topic}</p>
                  <div className="space-y-1">
                    {conflict.claims.map((claim, claimIdx) => (
                      <div key={claimIdx} className="text-xs text-red-700">
                        <span className="font-medium">Claim:</span> {claim.statement}
                        <a
                          href={claim.sourceUrl}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="ml-2 text-blue-600 hover:underline"
                        >
                          (source)
                        </a>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function Extraction() {
  // URL submission state
  const [url, setUrl] = useState('');
  const [urlQuery, setUrlQuery] = useState('');
  const [urlResult, setUrlResult] = useState<SubmitUrlResult | null>(null);

  // Extraction query state
  const [extractionQuery, setExtractionQuery] = useState('');
  const [extractionSite, setExtractionSite] = useState('');
  const [extractionResult, setExtractionResult] = useState<TriggerExtractionResult | null>(null);

  // Site ingestion state
  const [siteUrl, setSiteUrl] = useState('');
  const [maxPages, setMaxPages] = useState(50);
  const [ingestResult, setIngestResult] = useState<IngestSiteResult | null>(null);

  // Mutations
  const [submitUrl, { loading: submitting }] = useMutation(SUBMIT_URL);
  const [triggerExtraction, { loading: extracting }] = useMutation(TRIGGER_EXTRACTION);
  const [ingestSite, { loading: ingesting }] = useMutation(INGEST_SITE);

  // Handlers
  const handleSubmitUrl = async () => {
    if (!url.trim()) return;
    try {
      const { data } = await submitUrl({
        variables: {
          input: {
            url: url.trim(),
            query: urlQuery.trim() || undefined,
          },
        },
      });
      setUrlResult(data.submitUrl);
    } catch (error) {
      setUrlResult({
        success: false,
        url: url,
        extraction: null,
        error: error instanceof Error ? error.message : 'Unknown error',
      });
    }
  };

  const handleTriggerExtraction = async () => {
    if (!extractionQuery.trim()) return;
    try {
      const { data } = await triggerExtraction({
        variables: {
          input: {
            query: extractionQuery.trim(),
            site: extractionSite.trim() || undefined,
          },
        },
      });
      setExtractionResult(data.triggerExtraction);
    } catch (error) {
      setExtractionResult({
        success: false,
        query: extractionQuery,
        site: extractionSite || null,
        extractions: [],
        error: error instanceof Error ? error.message : 'Unknown error',
      });
    }
  };

  const handleIngestSite = async () => {
    if (!siteUrl.trim()) return;
    try {
      const { data } = await ingestSite({
        variables: {
          siteUrl: siteUrl.trim(),
          maxPages: maxPages,
        },
      });
      setIngestResult(data.ingestSite);
    } catch (error) {
      console.error('Ingest error:', error);
      setIngestResult(null);
    }
  };

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-5xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-stone-900 mb-2">Extraction Console</h1>
          <p className="text-stone-600">
            Submit URLs for extraction, run queries against indexed content, and manage site ingestion.
          </p>
        </div>

        {/* URL Submission Section */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <h2 className="text-xl font-bold text-stone-900 mb-4 flex items-center gap-2">
            <span className="text-2xl">🔗</span>
            Submit URL for Extraction
          </h2>
          <p className="text-sm text-stone-600 mb-4">
            Submit a URL to crawl, index, and extract information from. The page will be analyzed for events, services, programs, or opportunities.
          </p>

          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-stone-700 mb-1">URL</label>
              <input
                type="url"
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder="https://example.org/volunteer-opportunities"
                className="w-full px-3 py-2 border border-stone-300 rounded-lg focus:ring-2 focus:ring-amber-500 focus:border-amber-500"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-stone-700 mb-1">
                Custom Query (optional)
              </label>
              <input
                type="text"
                value={urlQuery}
                onChange={(e) => setUrlQuery(e.target.value)}
                placeholder="Default: events, services, programs, or volunteer opportunities"
                className="w-full px-3 py-2 border border-stone-300 rounded-lg focus:ring-2 focus:ring-amber-500 focus:border-amber-500"
              />
            </div>

            <button
              onClick={handleSubmitUrl}
              disabled={submitting || !url.trim()}
              className="px-4 py-2 bg-amber-600 text-white rounded-lg hover:bg-amber-700 disabled:opacity-50 disabled:cursor-not-allowed font-medium"
            >
              {submitting ? 'Submitting...' : 'Submit URL'}
            </button>
          </div>

          {/* URL Result */}
          {urlResult && (
            <div className="mt-6">
              {urlResult.error ? (
                <div className="p-4 bg-red-50 border border-red-200 rounded-lg">
                  <p className="text-red-800 font-medium">Error</p>
                  <p className="text-red-600 text-sm">{urlResult.error}</p>
                </div>
              ) : urlResult.extraction ? (
                <div>
                  <p className="text-green-700 font-medium mb-3">
                    Successfully extracted from {urlResult.url}
                  </p>
                  <ExtractionResult extraction={urlResult.extraction} />
                </div>
              ) : (
                <p className="text-stone-600">No extraction results available.</p>
              )}
            </div>
          )}
        </div>

        {/* Extraction Query Section */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <h2 className="text-xl font-bold text-stone-900 mb-4 flex items-center gap-2">
            <span className="text-2xl">🔍</span>
            Query Indexed Content
          </h2>
          <p className="text-sm text-stone-600 mb-4">
            Run natural language queries against previously indexed content. Optionally filter to a specific site.
          </p>

          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-stone-700 mb-1">Query</label>
              <input
                type="text"
                value={extractionQuery}
                onChange={(e) => setExtractionQuery(e.target.value)}
                placeholder="What volunteer opportunities are available for seniors?"
                className="w-full px-3 py-2 border border-stone-300 rounded-lg focus:ring-2 focus:ring-amber-500 focus:border-amber-500"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-stone-700 mb-1">
                Site Filter (optional)
              </label>
              <input
                type="text"
                value={extractionSite}
                onChange={(e) => setExtractionSite(e.target.value)}
                placeholder="e.g., redcross.org"
                className="w-full px-3 py-2 border border-stone-300 rounded-lg focus:ring-2 focus:ring-amber-500 focus:border-amber-500"
              />
            </div>

            <button
              onClick={handleTriggerExtraction}
              disabled={extracting || !extractionQuery.trim()}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed font-medium"
            >
              {extracting ? 'Extracting...' : 'Run Extraction'}
            </button>
          </div>

          {/* Extraction Result */}
          {extractionResult && (
            <div className="mt-6">
              {extractionResult.error ? (
                <div className="p-4 bg-red-50 border border-red-200 rounded-lg">
                  <p className="text-red-800 font-medium">Error</p>
                  <p className="text-red-600 text-sm">{extractionResult.error}</p>
                </div>
              ) : extractionResult.extractions.length > 0 ? (
                <div className="space-y-4">
                  <p className="text-green-700 font-medium">
                    Found {extractionResult.extractions.length} result(s) for "{extractionResult.query}"
                    {extractionResult.site && ` on ${extractionResult.site}`}
                  </p>
                  {extractionResult.extractions.map((extraction, idx) => (
                    <ExtractionResult key={idx} extraction={extraction} />
                  ))}
                </div>
              ) : (
                <p className="text-stone-600">No matching content found.</p>
              )}
            </div>
          )}
        </div>

        {/* Site Ingestion Section */}
        <div className="bg-white rounded-lg shadow-md p-6">
          <h2 className="text-xl font-bold text-stone-900 mb-4 flex items-center gap-2">
            <span className="text-2xl">🌐</span>
            Ingest Site (Admin)
          </h2>
          <p className="text-sm text-stone-600 mb-4">
            Crawl and index an entire site for future extraction queries. This will discover pages, summarize content, and store embeddings.
          </p>

          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-stone-700 mb-1">Site URL</label>
              <input
                type="url"
                value={siteUrl}
                onChange={(e) => setSiteUrl(e.target.value)}
                placeholder="https://example.org"
                className="w-full px-3 py-2 border border-stone-300 rounded-lg focus:ring-2 focus:ring-amber-500 focus:border-amber-500"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-stone-700 mb-1">
                Max Pages to Crawl
              </label>
              <input
                type="number"
                value={maxPages}
                onChange={(e) => setMaxPages(parseInt(e.target.value) || 50)}
                min={1}
                max={500}
                className="w-32 px-3 py-2 border border-stone-300 rounded-lg focus:ring-2 focus:ring-amber-500 focus:border-amber-500"
              />
            </div>

            <button
              onClick={handleIngestSite}
              disabled={ingesting || !siteUrl.trim()}
              className="px-4 py-2 bg-purple-600 text-white rounded-lg hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed font-medium"
            >
              {ingesting ? 'Ingesting...' : 'Start Ingestion'}
            </button>
          </div>

          {/* Ingest Result */}
          {ingestResult && (
            <div className="mt-6 p-4 bg-green-50 border border-green-200 rounded-lg">
              <p className="text-green-800 font-medium mb-2">
                Site Ingestion Complete: {ingestResult.siteUrl}
              </p>
              <div className="grid grid-cols-3 gap-4 text-sm">
                <div>
                  <p className="text-stone-500">Pages Crawled</p>
                  <p className="text-2xl font-bold text-green-700">{ingestResult.pagesCrawled}</p>
                </div>
                <div>
                  <p className="text-stone-500">Pages Summarized</p>
                  <p className="text-2xl font-bold text-blue-700">{ingestResult.pagesSummarized}</p>
                </div>
                <div>
                  <p className="text-stone-500">Pages Skipped</p>
                  <p className="text-2xl font-bold text-stone-500">{ingestResult.pagesSkipped}</p>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
