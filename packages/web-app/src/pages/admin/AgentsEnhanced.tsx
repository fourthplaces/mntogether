import { useState } from 'react';
import { useQuery, useMutation, gql } from '@apollo/client';
import { useNavigate } from 'react-router-dom';

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
      autoApproveWebsites
      autoScrape
      autoCreateListings
      totalSearchesRun
      totalWebsitesDiscovered
      totalWebsitesApproved
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

const GENERATE_AGENT_CONFIG = gql`
  mutation GenerateAgentConfig($description: String!, $locationContext: String!) {
    generateAgentConfig(description: $description, locationContext: $locationContext) {
      name
      queryTemplate
      extractionInstructions
      systemPrompt
    }
  }
`;

const TRIGGER_AGENT_SEARCH = gql`
  mutation TriggerAgentSearch($agentId: String!) {
    triggerAgentSearch(agentId: $agentId) {
      jobId
      status
    }
  }
`;

const UPDATE_AGENT = gql`
  mutation UpdateAgent($agentId: String!, $input: UpdateAgentInput!) {
    updateAgent(agentId: $agentId, input: $input) {
      id
      name
      queryTemplate
      description
      enabled
      locationContext
      extractionInstructions
      systemPrompt
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
  autoApproveWebsites: boolean;
  autoScrape: boolean;
  autoCreateListings: boolean;
  totalSearchesRun: number;
  totalWebsitesDiscovered: number;
  totalWebsitesApproved: number;
  createdAt: string;
}

export function AgentsEnhanced() {
  const navigate = useNavigate();
  const [showCreateWizard, setShowCreateWizard] = useState(false);
  const [editingAgent, setEditingAgent] = useState<Agent | null>(null);
  const [wizardStep, setWizardStep] = useState<'intent' | 'review'>('intent');
  const [userIntent, setUserIntent] = useState('');
  const [locationContext, setLocationContext] = useState('Minnesota');
  const [generatedConfig, setGeneratedConfig] = useState<any>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchingAgentId, setSearchingAgentId] = useState<string | null>(null);
  const [searchSuccess, setSearchSuccess] = useState<string | null>(null);
  const [isUpdating, setIsUpdating] = useState(false);

  const { data, loading, refetch } = useQuery<{ agents: Agent[] }>(GET_ALL_AGENTS);

  const [createAgent] = useMutation(CREATE_AGENT, {
    onCompleted: () => {
      setShowCreateWizard(false);
      setWizardStep('intent');
      setUserIntent('');
      setGeneratedConfig(null);
      setError(null);
      refetch();
    },
    onError: (err) => {
      setError(err.message);
    },
  });

  const [generateConfig] = useMutation(GENERATE_AGENT_CONFIG, {
    onCompleted: (data) => {
      setGeneratedConfig(data.generateAgentConfig);
      setWizardStep('review');
      setIsGenerating(false);
    },
    onError: (err) => {
      setError(err.message);
      setIsGenerating(false);
    },
  });

  const [triggerSearch] = useMutation(TRIGGER_AGENT_SEARCH, {
    onCompleted: (data) => {
      setSearchingAgentId(null);
      setSearchSuccess(
        `Search queued successfully! The agent will search Tavily and discover websites.`
      );
      setTimeout(() => setSearchSuccess(null), 5000);
      refetch();
    },
    onError: (err) => {
      setSearchingAgentId(null);
      setError(err.message);
    },
  });

  const [updateAgent] = useMutation(UPDATE_AGENT, {
    onCompleted: () => {
      console.log('Agent updated successfully');
      setIsUpdating(false);
      setEditingAgent(null);
      setError(null);
      refetch();
    },
    onError: (err) => {
      console.error('Error updating agent:', err);
      setIsUpdating(false);
      setError(err.message);
    },
  });

  const handleGenerateConfig = async () => {
    if (!userIntent.trim()) {
      setError('Please describe what the agent should search for');
      return;
    }

    setError(null);
    setIsGenerating(true);
    await generateConfig({
      variables: {
        description: userIntent,
        locationContext: locationContext,
      },
    });
  };

  const handleCreateAgent = async () => {
    if (!generatedConfig) return;

    setError(null);
    await createAgent({
      variables: {
        input: {
          name: generatedConfig.name,
          queryTemplate: generatedConfig.queryTemplate,
          description: userIntent,
          extractionInstructions: generatedConfig.extractionInstructions,
          systemPrompt: generatedConfig.systemPrompt,
          locationContext: locationContext,
        },
      },
    });
  };

  const handleTriggerSearch = async (agentId: string) => {
    setError(null);
    setSearchSuccess(null);
    setSearchingAgentId(agentId);
    await triggerSearch({ variables: { agentId } });
  };

  const handleUpdateAgent = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setIsUpdating(true);

    if (!editingAgent) {
      setIsUpdating(false);
      return;
    }

    console.log('Updating agent:', editingAgent.id);

    await updateAgent({
      variables: {
        agentId: editingAgent.id,
        input: {
          name: editingAgent.name,
          queryTemplate: editingAgent.queryTemplate,
          description: editingAgent.description || null,
          extractionInstructions: editingAgent.extractionInstructions || null,
          systemPrompt: editingAgent.systemPrompt || null,
          locationContext: editingAgent.locationContext,
          enabled: editingAgent.enabled,
        },
      },
    });
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
              AI-powered agents that automatically discover, scrape, and extract community resources
            </p>
          </div>
          <button
            onClick={() => setShowCreateWizard(!showCreateWizard)}
            className="bg-purple-600 text-white px-6 py-3 rounded-lg hover:bg-purple-700 focus:outline-none focus:ring-2 focus:ring-purple-500 font-medium flex items-center gap-2"
          >
            {showCreateWizard ? (
              <>Cancel</>
            ) : (
              <>
                <span>✨</span> Create Agent with AI
              </>
            )}
          </button>
        </div>

        {error && (
          <div className="mb-4 p-4 bg-red-50 border border-red-200 text-red-800 rounded-lg">
            {error}
          </div>
        )}

        {searchSuccess && (
          <div className="mb-4 p-4 bg-green-50 border border-green-200 text-green-800 rounded-lg flex items-center gap-2">
            <span className="text-xl">✅</span>
            {searchSuccess}
          </div>
        )}

        {/* AI-Powered Creation Wizard */}
        {showCreateWizard && (
          <div className="bg-white rounded-lg shadow-lg p-8 mb-6 border-2 border-purple-200">
            <div className="mb-6">
              <div className="flex items-center gap-3 mb-4">
                <div className="text-4xl">✨</div>
                <div>
                  <h2 className="text-2xl font-semibold text-stone-900">
                    {wizardStep === 'intent' ? 'Describe Your Agent' : 'Review & Create'}
                  </h2>
                  <p className="text-stone-600">
                    {wizardStep === 'intent'
                      ? 'Tell us what you want the agent to find, and AI will handle the rest'
                      : 'Review the generated configuration and make any adjustments'}
                  </p>
                </div>
              </div>

              {/* Progress Indicator */}
              <div className="flex gap-2 mb-6">
                <div
                  className={`flex-1 h-2 rounded ${
                    wizardStep === 'intent' ? 'bg-purple-600' : 'bg-purple-300'
                  }`}
                />
                <div
                  className={`flex-1 h-2 rounded ${
                    wizardStep === 'review' ? 'bg-purple-600' : 'bg-stone-200'
                  }`}
                />
              </div>
            </div>

            {wizardStep === 'intent' && (
              <div className="space-y-6">
                <div>
                  <label className="block text-sm font-medium text-stone-700 mb-2">
                    What should this agent search for? *
                  </label>
                  <textarea
                    value={userIntent}
                    onChange={(e) => setUserIntent(e.target.value)}
                    placeholder="Example: Find legal aid services for immigrants and refugees&#10;Example: Discover volunteer opportunities helping seniors&#10;Example: Locate food banks and emergency food assistance for families"
                    rows={4}
                    className="w-full px-4 py-3 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 text-base"
                  />
                  <p className="mt-2 text-sm text-stone-600">
                    Describe the type of resources, services, or opportunities this agent should discover.
                    Be specific about the target audience and focus area.
                  </p>
                </div>

                <div>
                  <label className="block text-sm font-medium text-stone-700 mb-2">
                    Location Context
                  </label>
                  <input
                    type="text"
                    value={locationContext}
                    onChange={(e) => setLocationContext(e.target.value)}
                    placeholder="Minnesota"
                    className="w-full px-4 py-3 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                  />
                  <p className="mt-2 text-sm text-stone-600">
                    The geographic area where this agent should search
                  </p>
                </div>

                <div className="bg-purple-50 border border-purple-200 rounded-lg p-4">
                  <div className="flex items-start gap-3">
                    <div className="text-2xl">💡</div>
                    <div>
                      <h4 className="font-medium text-purple-900 mb-1">How it works</h4>
                      <p className="text-sm text-purple-800">
                        Our AI will analyze your description and automatically generate:
                      </p>
                      <ul className="text-sm text-purple-800 mt-2 space-y-1 ml-4 list-disc">
                        <li>Search query templates optimized for discovery</li>
                        <li>Extraction instructions for AI-powered scraping</li>
                        <li>System prompts for intelligent data extraction</li>
                      </ul>
                    </div>
                  </div>
                </div>

                <button
                  onClick={handleGenerateConfig}
                  disabled={isGenerating || !userIntent.trim()}
                  className="w-full bg-purple-600 text-white px-6 py-4 rounded-lg hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed font-medium text-lg flex items-center justify-center gap-2"
                >
                  {isGenerating ? (
                    <>
                      <div className="animate-spin h-5 w-5 border-2 border-white border-t-transparent rounded-full" />
                      Generating with AI...
                    </>
                  ) : (
                    <>
                      <span>✨</span> Generate Agent Configuration
                    </>
                  )}
                </button>
              </div>
            )}

            {wizardStep === 'review' && generatedConfig && (
              <div className="space-y-6">
                <div className="bg-green-50 border border-green-200 rounded-lg p-4 mb-6">
                  <div className="flex items-center gap-2">
                    <span className="text-2xl">✅</span>
                    <p className="text-green-800 font-medium">
                      Configuration generated successfully! Review and edit if needed.
                    </p>
                  </div>
                </div>

                <div>
                  <label className="block text-sm font-medium text-stone-700 mb-2">
                    Agent Name
                  </label>
                  <input
                    type="text"
                    value={generatedConfig.name}
                    onChange={(e) =>
                      setGeneratedConfig({ ...generatedConfig, name: e.target.value })
                    }
                    className="w-full px-4 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-stone-700 mb-2">
                    Search Query Template
                  </label>
                  <input
                    type="text"
                    value={generatedConfig.queryTemplate}
                    onChange={(e) =>
                      setGeneratedConfig({ ...generatedConfig, queryTemplate: e.target.value })
                    }
                    className="w-full px-4 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono text-sm"
                  />
                  <p className="mt-1 text-xs text-stone-600">
                    Use {`{location}`} as a placeholder for location context
                  </p>
                </div>

                <div>
                  <label className="block text-sm font-medium text-stone-700 mb-2">
                    Extraction Instructions
                  </label>
                  <textarea
                    value={generatedConfig.extractionInstructions}
                    onChange={(e) =>
                      setGeneratedConfig({
                        ...generatedConfig,
                        extractionInstructions: e.target.value,
                      })
                    }
                    rows={4}
                    className="w-full px-4 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 text-sm"
                  />
                  <p className="mt-1 text-xs text-stone-600">
                    Instructions for what information to extract from discovered pages
                  </p>
                </div>

                <div>
                  <label className="block text-sm font-medium text-stone-700 mb-2">
                    System Prompt
                  </label>
                  <textarea
                    value={generatedConfig.systemPrompt}
                    onChange={(e) =>
                      setGeneratedConfig({ ...generatedConfig, systemPrompt: e.target.value })
                    }
                    rows={3}
                    className="w-full px-4 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 text-sm"
                  />
                  <p className="mt-1 text-xs text-stone-600">
                    System prompt for the AI extraction engine
                  </p>
                </div>

                <div className="flex gap-3">
                  <button
                    onClick={() => {
                      setWizardStep('intent');
                      setGeneratedConfig(null);
                    }}
                    className="flex-1 bg-stone-200 text-stone-700 px-6 py-3 rounded-lg hover:bg-stone-300 font-medium"
                  >
                    ← Back
                  </button>
                  <button
                    onClick={handleCreateAgent}
                    className="flex-1 bg-purple-600 text-white px-6 py-3 rounded-lg hover:bg-purple-700 font-medium"
                  >
                    Create Agent →
                  </button>
                </div>
              </div>
            )}
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
                <div
                  onClick={() => navigate(`/admin/websites?agentId=${agent.id}`)}
                  className="cursor-pointer hover:bg-purple-50 rounded p-1 -m-1 transition-colors"
                  title="Click to view websites"
                >
                  <p className="text-xs text-stone-600 mb-1">Websites Found</p>
                  <p className="text-lg font-bold text-purple-600 hover:underline">
                    {agent.totalWebsitesDiscovered}
                  </p>
                </div>
                <div>
                  <p className="text-xs text-stone-600 mb-1">Auto-Approved</p>
                  <p className="text-lg font-bold text-green-600">{agent.totalWebsitesApproved}</p>
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
                {agent.autoApproveWebsites && (
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
                  disabled={searchingAgentId === agent.id}
                  className="flex-1 bg-purple-600 text-white px-4 py-2 rounded hover:bg-purple-700 text-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                >
                  {searchingAgentId === agent.id ? (
                    <>
                      <div className="animate-spin h-4 w-4 border-2 border-white border-t-transparent rounded-full" />
                      Searching...
                    </>
                  ) : (
                    <>
                      🔍 Run Search Now
                    </>
                  )}
                </button>
                <button
                  onClick={() => setEditingAgent(agent)}
                  className="px-4 py-2 border border-stone-300 rounded hover:bg-stone-50 text-sm font-medium"
                >
                  Edit
                </button>
              </div>
            </div>
          ))}
        </div>

        {data?.agents?.length === 0 && (
          <div className="text-center py-16 bg-white rounded-lg shadow">
            <div className="text-6xl mb-4">✨</div>
            <h3 className="text-2xl font-bold text-stone-900 mb-2">No agents configured yet</h3>
            <p className="text-stone-600 mb-6 max-w-md mx-auto">
              Create your first autonomous agent with AI assistance. Just describe what you want to
              find, and we'll handle the technical details!
            </p>
            <button
              onClick={() => setShowCreateWizard(true)}
              className="bg-purple-600 text-white px-8 py-3 rounded-lg hover:bg-purple-700 font-medium inline-flex items-center gap-2"
            >
              <span>✨</span> Create First Agent with AI
            </button>
          </div>
        )}

        {/* Edit Agent Modal */}
        {editingAgent && (
          <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-6">
            <div className="bg-white rounded-lg shadow-xl max-w-2xl w-full max-h-[90vh] overflow-y-auto">
              <div className="p-6">
                <div className="flex justify-between items-center mb-4">
                  <h2 className="text-xl font-semibold text-stone-900">Edit Agent</h2>
                  <button
                    onClick={() => setEditingAgent(null)}
                    className="text-stone-400 hover:text-stone-600 text-2xl"
                  >
                    ×
                  </button>
                </div>

                <form onSubmit={handleUpdateAgent} className="space-y-4">
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                      <label className="block text-sm font-medium text-stone-700 mb-2">
                        Agent Name *
                      </label>
                      <input
                        type="text"
                        value={editingAgent.name}
                        onChange={(e) =>
                          setEditingAgent({ ...editingAgent, name: e.target.value })
                        }
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
                        value={editingAgent.locationContext}
                        onChange={(e) =>
                          setEditingAgent({ ...editingAgent, locationContext: e.target.value })
                        }
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
                      value={editingAgent.queryTemplate}
                      onChange={(e) =>
                        setEditingAgent({ ...editingAgent, queryTemplate: e.target.value })
                      }
                      className="w-full px-3 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                      required
                    />
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-stone-700 mb-2">
                      Description
                    </label>
                    <textarea
                      value={editingAgent.description || ''}
                      onChange={(e) =>
                        setEditingAgent({ ...editingAgent, description: e.target.value })
                      }
                      rows={2}
                      className="w-full px-3 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                    />
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-stone-700 mb-2">
                      Extraction Instructions
                    </label>
                    <textarea
                      value={editingAgent.extractionInstructions || ''}
                      onChange={(e) =>
                        setEditingAgent({
                          ...editingAgent,
                          extractionInstructions: e.target.value,
                        })
                      }
                      rows={3}
                      className="w-full px-3 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                    />
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-stone-700 mb-2">
                      System Prompt
                    </label>
                    <textarea
                      value={editingAgent.systemPrompt || ''}
                      onChange={(e) =>
                        setEditingAgent({ ...editingAgent, systemPrompt: e.target.value })
                      }
                      rows={2}
                      className="w-full px-3 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                    />
                  </div>

                  <div className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      id="enabled"
                      checked={editingAgent.enabled}
                      onChange={(e) =>
                        setEditingAgent({ ...editingAgent, enabled: e.target.checked })
                      }
                      className="w-4 h-4 text-purple-600 border-stone-300 rounded focus:ring-purple-500"
                    />
                    <label htmlFor="enabled" className="text-sm font-medium text-stone-700">
                      Agent Enabled
                    </label>
                  </div>

                  <div className="flex gap-3 pt-4">
                    <button
                      type="submit"
                      disabled={isUpdating}
                      className="flex-1 bg-purple-600 text-white px-6 py-2 rounded-md hover:bg-purple-700 focus:outline-none focus:ring-2 focus:ring-purple-500 font-medium disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                    >
                      {isUpdating ? (
                        <>
                          <div className="animate-spin h-4 w-4 border-2 border-white border-t-transparent rounded-full" />
                          Saving...
                        </>
                      ) : (
                        'Save Changes'
                      )}
                    </button>
                    <button
                      type="button"
                      onClick={() => setEditingAgent(null)}
                      disabled={isUpdating}
                      className="px-6 py-2 border border-stone-300 rounded-md hover:bg-stone-50 font-medium disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      Cancel
                    </button>
                  </div>
                </form>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
