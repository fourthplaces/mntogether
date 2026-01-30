import { ApolloProvider } from '@apollo/client';
import { BrowserRouter, Routes, Route, Link, Navigate } from 'react-router-dom';
import { apolloClient } from './graphql/client';
import { AuthProvider, useAuth } from './contexts/AuthContext';

// Public pages
import { Home } from './pages/Home';
import { SubmitResource } from './pages/SubmitResource';

// Admin pages
import { Login } from './pages/admin/Login';
import { ListingApprovalQueue } from './pages/admin/ListingApprovalQueue';
import ScrapedListingsReview from './pages/admin/ScrapedListingsReview';
import { Resources } from './pages/admin/Resources';
import { ResourceDetail } from './pages/admin/ResourceDetail';
import { OrganizationDetail } from './pages/admin/OrganizationDetail';
import { OrganizationsList } from './pages/admin/OrganizationsList';

// Admin protected layout
function AdminLayout() {
  const { isAuthenticated, logout } = useAuth();

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
                  to="/admin"
                  className="border-transparent text-stone-600 hover:border-amber-500 hover:text-amber-700 inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium"
                >
                  Approval Queue
                </Link>
                <Link
                  to="/admin/scraped"
                  className="border-transparent text-stone-600 hover:border-amber-500 hover:text-amber-700 inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium"
                >
                  🤖 Scraped Listings
                </Link>
                <Link
                  to="/admin/resources"
                  className="border-transparent text-stone-600 hover:border-amber-500 hover:text-amber-700 inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium"
                >
                  Resources
                </Link>
                <Link
                  to="/admin/organizations"
                  className="border-transparent text-stone-600 hover:border-amber-500 hover:text-amber-700 inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium"
                >
                  Businesses
                </Link>
              </div>
            </div>
            <div className="flex items-center">
              <Link
                to="/"
                className="text-stone-600 hover:text-stone-900 text-sm font-medium mr-4"
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

      <Routes>
        <Route path="/" element={<ListingApprovalQueue />} />
        <Route path="/scraped" element={<ScrapedListingsReview />} />
        <Route path="/resources" element={<Resources />} />
        <Route path="/resources/:sourceId" element={<ResourceDetail />} />
        <Route path="/organizations" element={<OrganizationsList />} />
        <Route path="/organizations/:sourceId" element={<OrganizationDetail />} />
      </Routes>
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
