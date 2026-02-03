import { ApolloClient, InMemoryCache, HttpLink, ApolloLink } from '@apollo/client';
import { setContext } from '@apollo/client/link/context';
import { onError } from '@apollo/client/link/error';

const TOKEN_KEY = 'admin_jwt_token';

// Check if error message indicates an auth failure
function isAuthError(message: string): boolean {
  const authErrorPatterns = [
    'Unauthenticated',
    'Unauthorized',
    'Admin access required',
    'Valid JWT required',
    'authentication required',
  ];
  return authErrorPatterns.some(pattern =>
    message.toLowerCase().includes(pattern.toLowerCase())
  );
}

// Error link to handle auth failures globally
const errorLink = onError(({ graphQLErrors, networkError }) => {
  if (graphQLErrors) {
    for (const err of graphQLErrors) {
      if (isAuthError(err.message)) {
        // Clear the invalid token
        localStorage.removeItem(TOKEN_KEY);

        // Only redirect if we're on an admin page
        if (window.location.pathname.startsWith('/admin') &&
            !window.location.pathname.includes('/admin/login')) {
          window.location.href = '/admin/login';
        }
        return;
      }
    }
  }

  // Handle network errors that might indicate auth issues (401)
  if (networkError && 'statusCode' in networkError && networkError.statusCode === 401) {
    localStorage.removeItem(TOKEN_KEY);
    if (window.location.pathname.startsWith('/admin') &&
        !window.location.pathname.includes('/admin/login')) {
      window.location.href = '/admin/login';
    }
  }
});

// Auth link to add JWT token to headers (for admin requests)
const authLink = setContext((_, { headers }) => {
  const token = localStorage.getItem(TOKEN_KEY);

  return {
    headers: {
      ...headers,
      ...(token ? { authorization: `Bearer ${token}` } : {}),
    },
  };
});

const httpLink = new HttpLink({
  uri: '/graphql',
});

export const apolloClient = new ApolloClient({
  link: ApolloLink.from([errorLink, authLink, httpLink]),
  cache: new InMemoryCache(),
});
