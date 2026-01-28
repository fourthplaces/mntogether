import React from 'react';
import { View, Text, FlatList, TouchableOpacity, StyleSheet, ActivityIndicator } from 'react-native';
import { useQuery } from '@apollo/client';
import { GET_ACTIVE_NEEDS } from '../graphql/queries';

interface Need {
  id: string;
  organizationName: string;
  title: string;
  tldr: string;
  location?: string;
  urgency?: string;
  createdAt: string;
}

interface NeedListScreenProps {
  navigation: any;
}

export function NeedListScreen({ navigation }: NeedListScreenProps) {
  const { data, loading, error, refetch } = useQuery(GET_ACTIVE_NEEDS, {
    variables: { limit: 50, offset: 0 },
  });

  if (loading) {
    return (
      <View style={styles.center}>
        <ActivityIndicator size="large" color="#2563eb" />
      </View>
    );
  }

  if (error) {
    return (
      <View style={styles.center}>
        <Text style={styles.errorText}>Error loading needs</Text>
        <TouchableOpacity style={styles.retryButton} onPress={() => refetch()}>
          <Text style={styles.retryText}>Retry</Text>
        </TouchableOpacity>
      </View>
    );
  }

  const needs = data?.needs?.nodes || [];

  const renderNeed = ({ item }: { item: Need }) => (
    <TouchableOpacity
      style={styles.card}
      onPress={() => navigation.navigate('NeedDetail', { needId: item.id })}
    >
      <View style={styles.cardHeader}>
        <Text style={styles.organizationName}>{item.organizationName}</Text>
        {item.urgency && (
          <View style={[
            styles.urgencyBadge,
            item.urgency === 'urgent' && styles.urgencyUrgent,
          ]}>
            <Text style={styles.urgencyText}>{item.urgency}</Text>
          </View>
        )}
      </View>
      <Text style={styles.title}>{item.title}</Text>
      {item.location && (
        <Text style={styles.location}>📍 {item.location}</Text>
      )}
      <Text style={styles.tldr} numberOfLines={2}>
        {item.tldr}
      </Text>
    </TouchableOpacity>
  );

  return (
    <View style={styles.container}>
      <FlatList
        data={needs}
        renderItem={renderNeed}
        keyExtractor={(item) => item.id}
        contentContainerStyle={styles.list}
        refreshing={loading}
        onRefresh={refetch}
        ListEmptyComponent={
          <View style={styles.center}>
            <Text style={styles.emptyText}>No needs available</Text>
          </View>
        }
      />
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#f3f4f6',
  },
  center: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    padding: 20,
  },
  list: {
    padding: 16,
  },
  card: {
    backgroundColor: 'white',
    borderRadius: 12,
    padding: 16,
    marginBottom: 16,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 4,
    elevation: 3,
  },
  cardHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 8,
  },
  organizationName: {
    fontSize: 14,
    color: '#6b7280',
    fontWeight: '500',
  },
  urgencyBadge: {
    paddingHorizontal: 8,
    paddingVertical: 4,
    borderRadius: 12,
    backgroundColor: '#fef3c7',
  },
  urgencyUrgent: {
    backgroundColor: '#fee2e2',
  },
  urgencyText: {
    fontSize: 12,
    fontWeight: '600',
    color: '#92400e',
  },
  title: {
    fontSize: 18,
    fontWeight: '600',
    color: '#111827',
    marginBottom: 8,
  },
  location: {
    fontSize: 14,
    color: '#6b7280',
    marginBottom: 8,
  },
  tldr: {
    fontSize: 14,
    color: '#4b5563',
    lineHeight: 20,
  },
  errorText: {
    fontSize: 16,
    color: '#ef4444',
    marginBottom: 16,
  },
  retryButton: {
    paddingHorizontal: 20,
    paddingVertical: 10,
    backgroundColor: '#2563eb',
    borderRadius: 8,
  },
  retryText: {
    color: 'white',
    fontSize: 16,
    fontWeight: '600',
  },
  emptyText: {
    fontSize: 16,
    color: '#9ca3af',
  },
});
