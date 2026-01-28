import { ApolloProvider } from '@apollo/client';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { apolloClient } from './graphql/client';
import { Home } from './pages/Home';
import { SubmitResource } from './pages/SubmitResource';

function App() {
  return (
    <ApolloProvider client={apolloClient}>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/submit" element={<SubmitResource />} />
        </Routes>
      </BrowserRouter>
    </ApolloProvider>
  );
}

export default App;
