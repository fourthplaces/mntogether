import { ApolloProvider } from '@apollo/client';
import { BrowserRouter, Routes, Route, Link } from 'react-router-dom';
import { apolloClient } from './graphql/client';
import { NeedApprovalQueue } from './pages/NeedApprovalQueue';

function App() {
  return (
    <ApolloProvider client={apolloClient}>
      <BrowserRouter>
        <div className="min-h-screen bg-gray-50">
          <nav className="bg-white border-b border-gray-200">
            <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
              <div className="flex justify-between h-16">
                <div className="flex">
                  <div className="flex-shrink-0 flex items-center">
                    <h1 className="text-xl font-bold text-gray-900">
                      Emergency Resource Aggregator
                    </h1>
                  </div>
                  <div className="ml-6 flex space-x-8">
                    <Link
                      to="/"
                      className="border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700 inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium"
                    >
                      Approval Queue
                    </Link>
                  </div>
                </div>
              </div>
            </div>
          </nav>

          <Routes>
            <Route path="/" element={<NeedApprovalQueue />} />
          </Routes>
        </div>
      </BrowserRouter>
    </ApolloProvider>
  );
}

export default App;
