import { useState } from 'react';
import { useQuery, useMutation, gql } from '@apollo/client';

const GET_ALL_AGENTS = gql`
  query GetAllAgents {
    agents {
      id
      name
      queryTemplate
      description
      enabled
      searchFrequencyHours
      lastSearchedAt
      locationContext
      searchDepth
      maxResults
      daysRange
      minRelevanceScore
      extractionInstructions
      systemPrompt
      autoApproveDomains
      autoScrape
      autoCreateListings
      totalSearchesRun
      totalDomainsDiscovered
      totalDomainsApproved
      createdAt
    }
  }
`;

const CREATE_AGENT = gql`
  mutation CreateAgent($input: CreateAgentInput!) {
    createAgent(input: $input) {
      id
      name
    }
  }
`;

const TRIGGER_AGENT_SEARCH = gql`
  mutation TriggerAgentSearch($agentId: ID!) {
    triggerAgentSearch(agentId: $agentId) {
      jobId
      status
    }
  }
`;

interface Agent {
  id: string;
  name: string;
  queryTemplate: string;
  description: string | null;
  enabled: boolean;
  searchFrequencyHours: number;
  lastSearchedAt: string | null;
  locationContext: string;
  searchDepth: string;
  maxResults: number;
  daysRange: number;
  minRelevanceScore: number;
  extractionInstructions: string | null;
  systemPrompt: string | null;
  autoApproveDomains: boolean;
  autoScrape: boolean;
  autoCreateListings: boolean;
  totalSearchesRun: number;
  totalDomainsDiscovered: number;
  totalDomainsApproved: number;
  createdAt: string;
}

export function Agents() {
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [newAgent, setNewAgent] = useState({
    name: '',
    queryTemplate: '',
    description: '',
    locationContext: 'Minnesota',
    extractionInstructions: '',
  });
  const [error, setError] = useState<string | null>(null);

  const { data, loading, refetch } = useQuery<{ agents: Agent[] }>(GET_ALL_AGENTS);

  const [createAgent] = useMutation(CREATE_AGENT, {
    onCompleted: () => {
      setShowCreateForm(false);
      setNewAgent({
        name: '',
        queryTemplate: '',
        description: '',
        locationContext: 'Minnesota',
        extractionInstructions: '',
      });
      setError(null);
      refetch();
    },
    onError: (err) => {
      setError(err.message);
    },
  });

  const [triggerSearch] = useMutation(TRIGGER_AGENT_SEARCH, {
    onCompleted: () => {
      refetch();
    },
    onError: (err) => {
      setError(err.message);
    },
  });

  const handleCreateAgent = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (!newAgent.name.trim() || !newAgent.queryTemplate.trim()) {
      setError('Name and query template are required');
      return;
    }

    await createAgent({
      variables: {
        input: {
          name: newAgent.name,
          queryTemplate: newAgent.queryTemplate,
          description: newAgent.description || null,
          extractionInstructions: newAgent.extractionInstructions || null,
          systemPrompt: null,
          locationContext: newAgent.locationContext,
        },
      },
    });
  };

  const handleTriggerSearch = async (agentId: string) => {
    setError(null);
    await triggerSearch({ variables: { agentId } });
  };

  const formatDate = (dateString: string | null) => {
    if (!dateString) return 'Never';
    return new Date(dateString).toLocaleString();
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-stone-600">Loading agents...</div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex justify-between items-center mb-6">
          <div>
            <h1 className="text-3xl font-bold text-stone-900 mb-2">Autonomous Agents</h1>
            <p className="text-stone-600">
              Agents automatically search for domains, scrape content, and extract listings
            </p>
          </div>
          <button
            onClick={() => setShowCreateForm(!showCreateForm)}
            className="bg-purple-600 text-white px-6 py-3 rounded-lg hover:bg-purple-700 focus:outline-none focus:ring-2 focus:ring-purple-500 font-medium"
          >
            {showCreateForm ? 'Cancel' : '+ Create Agent'}
          </button>
        </div>

        {error && (
          <div className="mb-4 p-4 bg-red-50 border border-red-200 text-red-800 rounded-lg">
            {error}
          </div>
        )}

        {/* Create Agent Form */}
        {showCreateForm && (
          <div className="bg-white rounded-lg shadow-md p-6 mb-6">
            <h2 className="text-xl font-semibold text-stone-900 mb-4">Create New Agent</h2>
            <form onSubmit={handleCreateAgent} className="space-y-4">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-stone-700 mb-2">
                    Agent Name *
                  </label>
                  <input
                    type="text"
                    value={newAgent.name}
                    onChange={(e) => setNewAgent({ ...newAgent, name: e.target.value })}
                    placeholder="e.g., Legal Aid Finder"
                    className="w-full px-3 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                    required
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-stone-700 mb-2">
                    Location Context
                  </label>
                  <input
                    type="text"
                    value={newAgent.locationContext}
                    onChange={(e) => setNewAgent({ ...newAgent, locationContext: e.target.value })}
                    placeholder="Minnesota"
                    className="w-full px-3 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                  />
                </div>
              </div>

              <div>
                <label className="block text-sm font-medium text-stone-700 mb-2">
                  Search Query Template *
                </label>
                <input
                  type="text"
                  value={newAgent.queryTemplate}
                  onChange={(e) => setNewAgent({ ...newAgent, queryTemplate: e.target.value })}
                  placeholder='legal aid {location} OR "immigration help" {location}'
                  className="w-full px-3 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                  required
                />
                <p className="mt-1 text-sm text-stone-600">
                  Use {`{location}`} placeholder for location context
                </p>
              </div>

              <div>
                <label className="block text-sm font-medium text-stone-700 mb-2">
                  Description
                </label>
                <textarea
                  value={newAgent.description}
                  onChange={(e) => setNewAgent({ ...newAgent, description: e.target.value })}
                  placeholder="What this agent searches for..."
                  rows={2}
                  className="w-full px-3 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-stone-700 mb-2">
                  Extraction Instructions
                </label>
                <textarea
                  value={newAgent.extractionInstructions}
                  onChange={(e) =>
                    setNewAgent({ ...newAgent, extractionInstructions: e.target.value })
                  }
                  placeholder="Extract legal aid services including eligibility requirements, contact information..."
                  rows={3}
                  className="w-full px-3 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                />
                <p className="mt-1 text-sm text-stone-600">
                  Tell the AI what to look for when extracting listings from scraped pages
                </p>
              </div>

              <button
                type="submit"
                className="bg-purple-600 text-white px-6 py-2 rounded-md hover:bg-purple-700 focus:outline-none focus:ring-2 focus:ring-purple-500 font-medium"
              >
                Create Agent
              </button>
            </form>
          </div>
        )}

        {/* Agents Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {data?.agents.map((agent) => (
            <div key={agent.id} className="bg-white rounded-lg shadow-md p-6">
              {/* Agent Header */}
              <div className="flex justify-between items-start mb-4">
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <h3 className="text-xl font-semibold text-stone-900">{agent.name}</h3>
                    {agent.enabled ? (
                      <span className="px-2 py-1 text-xs rounded-full bg-green-100 text-green-800">
                        Active
                      </span>
                    ) : (
                      <span className="px-2 py-1 text-xs rounded-full bg-gray-100 text-gray-800">
                        Disabled
                      </span>
                    )}
                  </div>
                  {agent.description && (
                    <p className="text-sm text-stone-600">{agent.description}</p>
                  )}
                </div>
              </div>

              {/* Query Template */}
              <div className="mb-4 p-3 bg-stone-50 rounded border border-stone-200">
                <p className="text-xs font-medium text-stone-700 mb-1">SEARCH QUERY</p>
                <p className="text-sm font-mono text-stone-900">{agent.queryTemplate}</p>
              </div>

              {/* Stats */}
              <div className="grid grid-cols-3 gap-4 mb-4">
                <div>
                  <p className="text-xs text-stone-600 mb-1">Searches Run</p>
                  <p className="text-lg font-bold text-stone-900">{agent.totalSearchesRun}</p>
                </div>
                <div>
                  <p className="text-xs text-stone-600 mb-1">Domains Found</p>
                  <p className="text-lg font-bold text-purple-600">
                    {agent.totalDomainsDiscovered}
                  </p>
                </div>
                <div>
                  <p className="text-xs text-stone-600 mb-1">Auto-Approved</p>
                  <p className="text-lg font-bold text-green-600">{agent.totalDomainsApproved}</p>
                </div>
              </div>

              {/* Config */}
              <div className="mb-4 space-y-2 text-sm">
                <div className="flex items-center gap-2">
                  <span className="text-stone-600">Location:</span>
                  <span className="font-medium text-stone-900">{agent.locationContext}</span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-stone-600">Last Searched:</span>
                  <span className="font-medium text-stone-900">
                    {formatDate(agent.lastSearchedAt)}
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-stone-600">Search Frequency:</span>
                  <span className="font-medium text-stone-900">
                    Every {agent.searchFrequencyHours}h
                  </span>
                </div>
              </div>

              {/* Automation Flags */}
              <div className="flex flex-wrap gap-2 mb-4">
                {agent.autoScrape && (
                  <span className="px-2 py-1 text-xs rounded-full bg-blue-100 text-blue-800">
                    🤖 Auto-Scrape
                  </span>
                )}
                {agent.autoApproveDomains && (
                  <span className="px-2 py-1 text-xs rounded-full bg-green-100 text-green-800">
                    ✅ Auto-Approve
                  </span>
                )}
                {agent.autoCreateListings && (
                  <span className="px-2 py-1 text-xs rounded-full bg-purple-100 text-purple-800">
                    📝 Auto-Extract
                  </span>
                )}
              </div>

              {/* Actions */}
              <div className="flex gap-2">
                <button
                  onClick={() => handleTriggerSearch(agent.id)}
                  className="flex-1 bg-purple-600 text-white px-4 py-2 rounded hover:bg-purple-700 text-sm font-medium"
                >
                  Run Search Now
                </button>
                <button className="px-4 py-2 border border-stone-300 rounded hover:bg-stone-50 text-sm font-medium">
                  Edit
                </button>
              </div>
            </div>
          ))}
        </div>

        {data?.agents?.length === 0 && (
          <div className="text-center py-12 bg-white rounded-lg shadow">
            <div className="text-4xl mb-3">🤖</div>
            <p className="text-stone-600 mb-4">
              No agents configured yet. Create your first agent to start discovering resources!
            </p>
            <button
              onClick={() => setShowCreateForm(true)}
              className="bg-purple-600 text-white px-6 py-2 rounded-lg hover:bg-purple-700 font-medium"
            >
              Create First Agent
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
