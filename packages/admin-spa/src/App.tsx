import { ApolloProvider } from '@apollo/client';
import { BrowserRouter, Routes, Route, Link, Navigate } from 'react-router-dom';
import { apolloClient } from './graphql/client';
import { AuthProvider, useAuth } from './contexts/AuthContext';
import { NeedApprovalQueue } from './pages/NeedApprovalQueue';
import { Login } from './pages/Login';
import { Resources } from './pages/Resources';
import { ResourceDetail } from './pages/ResourceDetail';
import { OrganizationDetail } from './pages/OrganizationDetail';

function ProtectedLayout() {
  const { isAuthenticated, logout } = useAuth();

  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  return (
    <div className="min-h-screen bg-amber-50">
      <nav className="bg-white border-b border-stone-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between h-16">
            <div className="flex">
              <div className="flex-shrink-0 flex items-center">
                <h1 className="text-xl font-bold text-stone-900">
                  Emergency Resource Aggregator
                </h1>
              </div>
              <div className="ml-6 flex space-x-8">
                <Link
                  to="/"
                  className="border-transparent text-stone-600 hover:border-amber-500 hover:text-amber-700 inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium"
                >
                  Approval Queue
                </Link>
                <Link
                  to="/resources"
                  className="border-transparent text-stone-600 hover:border-amber-500 hover:text-amber-700 inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium"
                >
                  Resources
                </Link>
              </div>
            </div>
            <div className="flex items-center">
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
        <Route path="/" element={<NeedApprovalQueue />} />
        <Route path="/resources" element={<Resources />} />
        <Route path="/resources/:sourceId" element={<ResourceDetail />} />
        <Route path="/organizations/:sourceId" element={<OrganizationDetail />} />
      </Routes>
    </div>
  );
}

function AppRoutes() {
  const { isAuthenticated } = useAuth();

  return (
    <Routes>
      <Route
        path="/login"
        element={isAuthenticated ? <Navigate to="/" replace /> : <Login />}
      />
      <Route path="/*" element={<ProtectedLayout />} />
    </Routes>
  );
}

function App() {
  return (
    <ApolloProvider client={apolloClient}>
      <AuthProvider>
        <BrowserRouter basename="/admin">
          <AppRoutes />
        </BrowserRouter>
      </AuthProvider>
    </ApolloProvider>
  );
}

export default App;
