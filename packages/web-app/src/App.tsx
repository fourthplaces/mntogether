import { useState } from 'react';
import { ApolloProvider } from '@apollo/client';
import { BrowserRouter, Routes, Route, Link, Navigate } from 'react-router-dom';
import { apolloClient } from './graphql/client';
import { AuthProvider, useAuth } from './contexts/AuthContext';

// Public pages
import { Home } from './pages/Home';
import { SubmitResource } from './pages/SubmitResource';

// Admin pages
import { Login } from './pages/admin/Login';
import { Dashboard } from './pages/admin/Dashboard';
import { AgentsEnhanced as Agents } from './pages/admin/AgentsEnhanced';
import { Websites } from './pages/admin/Websites';
import { ListingApprovalQueue } from './pages/admin/ListingApprovalQueue';
import ScrapedListingsReview from './pages/admin/ScrapedListingsReview';
import { Resources } from './pages/admin/Resources';
import { ResourceDetail } from './pages/admin/ResourceDetail';
import { OrganizationDetail } from './pages/admin/OrganizationDetail';
import { OrganizationsList } from './pages/admin/OrganizationsList';
import { WebsiteDetail } from './pages/admin/WebsiteDetail';
import { ListingDetail } from './pages/admin/ListingDetail';

// Components
import { Chatroom } from './components/Chatroom';

// Admin protected layout
function AdminLayout() {
  const { isAuthenticated, logout } = useAuth();
  const [isChatOpen, setIsChatOpen] = useState(false);

  if (!isAuthenticated) {
    return <Navigate to="/admin/login" replace />;
  }

  return (
    <div className="min-h-screen bg-amber-50">
      <nav className="bg-white border-b border-stone-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between h-16">
            <div className="flex">
              <div className="flex-shrink-0 flex items-center">
                <h1 className="text-xl font-bold text-stone-900">
                  MN Together - Admin
                </h1>
              </div>
              <div className="ml-6 flex space-x-8">
                <Link
                  to="/admin/dashboard"
                  className="border-transparent text-stone-600 hover:border-amber-500 hover:text-amber-700 inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium"
                >
                  📊 Dashboard
                </Link>
                <Link
                  to="/admin/agents"
                  className="border-transparent text-stone-600 hover:border-amber-500 hover:text-amber-700 inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium"
                >
                  🤖 Agents
                </Link>
                <Link
                  to="/admin/websites"
                  className="border-transparent text-stone-600 hover:border-amber-500 hover:text-amber-700 inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium"
                >
                  🌐 Websites
                </Link>
                <Link
                  to="/admin/listings"
                  className="border-transparent text-stone-600 hover:border-amber-500 hover:text-amber-700 inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium"
                >
                  ✅ Listings
                </Link>
                <Link
                  to="/admin/scraped"
                  className="border-transparent text-stone-600 hover:border-amber-500 hover:text-amber-700 inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium"
                >
                  🔍 Scraped
                </Link>
              </div>
            </div>
            <div className="flex items-center gap-4">
              {/* Chat toggle button */}
              <button
                onClick={() => setIsChatOpen(!isChatOpen)}
                className={`relative p-2 rounded-lg transition-colors ${
                  isChatOpen
                    ? 'bg-amber-500 text-white'
                    : 'text-stone-600 hover:text-stone-900 hover:bg-stone-100'
                }`}
                title="Admin Assistant"
              >
                <svg
                  className="w-5 h-5"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"
                  />
                </svg>
                {/* Notification dot for new messages - can be wired to real state later */}
                <span className="absolute -top-1 -right-1 w-3 h-3 bg-green-500 border-2 border-white rounded-full"></span>
              </button>
              <Link
                to="/"
                className="text-stone-600 hover:text-stone-900 text-sm font-medium"
              >
                Public Site
              </Link>
              <button
                onClick={logout}
                className="text-stone-600 hover:text-stone-900 text-sm font-medium"
              >
                Logout
              </button>
            </div>
          </div>
        </div>
      </nav>

      {/* Main content with adjusted width when chat is open */}
      <div className={`transition-all duration-300 ${isChatOpen ? 'mr-96' : ''}`}>
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/dashboard" element={<Dashboard />} />
          <Route path="/agents" element={<Agents />} />
          <Route path="/websites" element={<Websites />} />
          <Route path="/websites/:websiteId" element={<WebsiteDetail />} />
          <Route path="/listings" element={<ListingApprovalQueue />} />
          <Route path="/listings/:listingId" element={<ListingDetail />} />
          <Route path="/scraped" element={<ScrapedListingsReview />} />
          <Route path="/resources" element={<Resources />} />
          <Route path="/resources/:sourceId" element={<ResourceDetail />} />
          <Route path="/organizations" element={<OrganizationsList />} />
          <Route path="/organizations/:sourceId" element={<OrganizationDetail />} />
        </Routes>
      </div>

      {/* Chatroom slide-out panel */}
      <Chatroom isOpen={isChatOpen} onClose={() => setIsChatOpen(false)} />
    </div>
  );
}

// Admin routes with auth
function AdminRoutes() {
  const { isAuthenticated } = useAuth();

  return (
    <Routes>
      <Route
        path="/login"
        element={isAuthenticated ? <Navigate to="/admin" replace /> : <Login />}
      />
      <Route path="/*" element={<AdminLayout />} />
    </Routes>
  );
}

function App() {
  return (
    <ApolloProvider client={apolloClient}>
      <AuthProvider>
        <BrowserRouter>
          <Routes>
            {/* Public routes */}
            <Route path="/" element={<Home />} />
            <Route path="/submit" element={<SubmitResource />} />

            {/* Admin routes */}
            <Route path="/admin/*" element={<AdminRoutes />} />
          </Routes>
        </BrowserRouter>
      </AuthProvider>
    </ApolloProvider>
  );
}

export default App;
