import React from 'react';
import { ApolloProvider } from '@apollo/client';
import { NavigationContainer } from '@react-navigation/native';
import { createStackNavigator } from '@react-navigation/stack';
import { apolloClient } from './src/graphql/client';
import { NeedListScreen } from './src/screens/NeedListScreen';
import { NeedDetailScreen } from './src/screens/NeedDetailScreen';

const Stack = createStackNavigator();

export default function App() {
  return (
    <ApolloProvider client={apolloClient}>
      <NavigationContainer>
        <Stack.Navigator
          initialRouteName="NeedList"
          screenOptions={{
            headerStyle: {
              backgroundColor: '#2563eb',
            },
            headerTintColor: '#fff',
            headerTitleStyle: {
              fontWeight: '600',
            },
          }}
        >
          <Stack.Screen
            name="NeedList"
            component={NeedListScreen}
            options={{ title: 'Community Needs' }}
          />
          <Stack.Screen
            name="NeedDetail"
            component={NeedDetailScreen}
            options={{ title: 'Need Details' }}
          />
        </Stack.Navigator>
      </NavigationContainer>
    </ApolloProvider>
  );
}
