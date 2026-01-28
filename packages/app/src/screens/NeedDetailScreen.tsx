import React from 'react';
import { View, Text, ScrollView, StyleSheet, ActivityIndicator, TouchableOpacity, Linking } from 'react-native';
import { useQuery } from '@apollo/client';
import { GET_NEED_DETAIL } from '../graphql/queries';

interface NeedDetailScreenProps {
  route: any;
  navigation: any;
}

export function NeedDetailScreen({ route, navigation }: NeedDetailScreenProps) {
  const { needId } = route.params;
  const { data, loading, error } = useQuery(GET_NEED_DETAIL, {
    variables: { id: needId },
  });

  if (loading) {
    return (
      <View style={styles.center}>
        <ActivityIndicator size="large" color="#2563eb" />
      </View>
    );
  }

  if (error || !data?.need) {
    return (
      <View style={styles.center}>
        <Text style={styles.errorText}>Error loading need details</Text>
        <TouchableOpacity
          style={styles.backButton}
          onPress={() => navigation.goBack()}
        >
          <Text style={styles.backButtonText}>Go Back</Text>
        </TouchableOpacity>
      </View>
    );
  }

  const need = data.need;

  const handleContact = (type: 'email' | 'phone' | 'website', value: string) => {
    let url = value;
    if (type === 'email') {
      url = `mailto:${value}`;
    } else if (type === 'phone') {
      url = `tel:${value}`;
    } else if (!value.startsWith('http')) {
      url = `https://${value}`;
    }
    Linking.openURL(url);
  };

  return (
    <ScrollView style={styles.container}>
      <View style={styles.content}>
        {/* Organization Header */}
        <View style={styles.header}>
          <Text style={styles.organizationName}>{need.organizationName}</Text>
          {need.urgency && (
            <View style={[
              styles.urgencyBadge,
              need.urgency === 'urgent' && styles.urgencyUrgent,
            ]}>
              <Text style={styles.urgencyText}>{need.urgency}</Text>
            </View>
          )}
        </View>

        {/* Title */}
        <Text style={styles.title}>{need.title}</Text>

        {/* Location */}
        {need.location && (
          <Text style={styles.location}>📍 {need.location}</Text>
        )}

        {/* TLDR */}
        {need.tldr && (
          <View style={styles.section}>
            <Text style={styles.sectionTitle}>Summary</Text>
            <Text style={styles.tldr}>{need.tldr}</Text>
          </View>
        )}

        {/* Full Description */}
        <View style={styles.section}>
          <Text style={styles.sectionTitle}>Details</Text>
          <Text style={styles.description}>{need.description}</Text>
        </View>

        {/* Contact Information */}
        {need.contactInfo && (
          <View style={styles.section}>
            <Text style={styles.sectionTitle}>Contact</Text>
            {need.contactInfo.email && (
              <TouchableOpacity
                style={styles.contactButton}
                onPress={() => handleContact('email', need.contactInfo.email)}
              >
                <Text style={styles.contactButtonText}>
                  ✉️ {need.contactInfo.email}
                </Text>
              </TouchableOpacity>
            )}
            {need.contactInfo.phone && (
              <TouchableOpacity
                style={styles.contactButton}
                onPress={() => handleContact('phone', need.contactInfo.phone)}
              >
                <Text style={styles.contactButtonText}>
                  📞 {need.contactInfo.phone}
                </Text>
              </TouchableOpacity>
            )}
            {need.contactInfo.website && (
              <TouchableOpacity
                style={styles.contactButton}
                onPress={() => handleContact('website', need.contactInfo.website)}
              >
                <Text style={styles.contactButtonText}>
                  🌐 {need.contactInfo.website}
                </Text>
              </TouchableOpacity>
            )}
          </View>
        )}

        {/* I'm Interested Button */}
        <TouchableOpacity style={styles.interestedButton}>
          <Text style={styles.interestedButtonText}>
            I'm Interested
          </Text>
        </TouchableOpacity>
      </View>
    </ScrollView>
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
  content: {
    padding: 16,
  },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
  },
  organizationName: {
    fontSize: 16,
    color: '#6b7280',
    fontWeight: '500',
  },
  urgencyBadge: {
    paddingHorizontal: 12,
    paddingVertical: 6,
    borderRadius: 16,
    backgroundColor: '#fef3c7',
  },
  urgencyUrgent: {
    backgroundColor: '#fee2e2',
  },
  urgencyText: {
    fontSize: 14,
    fontWeight: '600',
    color: '#92400e',
  },
  title: {
    fontSize: 28,
    fontWeight: '700',
    color: '#111827',
    marginBottom: 12,
  },
  location: {
    fontSize: 16,
    color: '#6b7280',
    marginBottom: 20,
  },
  section: {
    marginBottom: 24,
  },
  sectionTitle: {
    fontSize: 18,
    fontWeight: '600',
    color: '#111827',
    marginBottom: 12,
  },
  tldr: {
    fontSize: 16,
    color: '#374151',
    lineHeight: 24,
    backgroundColor: '#f9fafb',
    padding: 12,
    borderRadius: 8,
  },
  description: {
    fontSize: 16,
    color: '#374151',
    lineHeight: 24,
  },
  contactButton: {
    backgroundColor: 'white',
    padding: 16,
    borderRadius: 8,
    marginBottom: 8,
    borderWidth: 1,
    borderColor: '#e5e7eb',
  },
  contactButtonText: {
    fontSize: 16,
    color: '#2563eb',
    fontWeight: '500',
  },
  interestedButton: {
    backgroundColor: '#2563eb',
    padding: 16,
    borderRadius: 12,
    alignItems: 'center',
    marginTop: 12,
    marginBottom: 32,
  },
  interestedButtonText: {
    color: 'white',
    fontSize: 18,
    fontWeight: '600',
  },
  errorText: {
    fontSize: 16,
    color: '#ef4444',
    marginBottom: 16,
  },
  backButton: {
    paddingHorizontal: 20,
    paddingVertical: 10,
    backgroundColor: '#6b7280',
    borderRadius: 8,
  },
  backButtonText: {
    color: 'white',
    fontSize: 16,
    fontWeight: '600',
  },
});
