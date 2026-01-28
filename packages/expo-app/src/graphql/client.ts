import { ApolloClient, InMemoryCache, HttpLink } from '@apollo/client';
import Constants from 'expo-constants';

// Get API URL from environment or use localhost for development
const API_URL = Constants.expoConfig?.extra?.apiUrl || 'http://localhost:8080/graphql';

const httpLink = new HttpLink({
  uri: API_URL,
});

export const apolloClient = new ApolloClient({
  link: httpLink,
  cache: new InMemoryCache(),
});
