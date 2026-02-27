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
    createdAt
    updatedAt
    publishedAt
    organizationId
    organizationName
    distanceMiles
    relevanceScore
    relevanceBreakdown
    submissionType
    weight
    priority
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
  }
`);

export const OrganizationFields = graphql(`
  fragment OrganizationFields on Organization {
    id
    name
    description
    status
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
