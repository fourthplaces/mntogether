import { graphql } from "@/gql";

export const PostListFields = graphql(`
  fragment PostListFields on Post {
    id
    title
    description
    summary
    status
    postType
    category
    capacityStatus
    urgency
    location
    sourceUrl
    submissionType
    createdAt
    publishedAt
    distanceMiles
    relevanceScore
    relevanceBreakdown
    organizationId
    organizationName
    tags {
      id
      kind
      value
      displayName
      color
    }
  }
`);

export const PostDetailFields = graphql(`
  fragment PostDetailFields on Post {
    id
    title
    description
    descriptionMarkdown
    summary
    status
    postType
    category
    capacityStatus
    urgency
    location
    sourceUrl
    submissionType
    createdAt
    updatedAt
    publishedAt
    organizationId
    organizationName
    distanceMiles
    relevanceScore
    relevanceBreakdown
    hasUrgentNotes
    tags {
      id
      kind
      value
      displayName
      color
      description
      emoji
    }
    schedules {
      id
      dayOfWeek
      opensAt
      closesAt
      timezone
      notes
      rrule
      dtstart
      dtend
      isAllDay
      durationMinutes
    }
    contacts {
      id
      contactType
      contactValue
      contactLabel
    }
    submittedBy {
      submitterType
      agentId
      agentName
    }
    urgentNotes {
      content
      ctaText
    }
    comments {
      id
      containerId
      role
      content
      parentMessageId
      createdAt
    }
  }
`);

export const OrganizationFields = graphql(`
  fragment OrganizationFields on Organization {
    id
    name
    description
    status
    websiteCount
    socialProfileCount
    snapshotCount
    createdAt
    updatedAt
  }
`);

export const SourceFields = graphql(`
  fragment SourceFields on Source {
    id
    sourceType
    identifier
    url
    status
    active
    organizationId
    organizationName
    scrapeFrequencyHours
    lastScrapedAt
    postCount
    snapshotCount
    createdAt
    updatedAt
  }
`);

export const NoteFields = graphql(`
  fragment NoteFields on Note {
    id
    content
    ctaText
    severity
    sourceUrl
    sourceId
    sourceType
    isPublic
    createdBy
    expiredAt
    createdAt
    updatedAt
    linkedPosts {
      id
      title
    }
  }
`);
