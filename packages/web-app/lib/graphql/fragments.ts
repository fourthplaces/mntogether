import { graphql } from "@/gql";

export const PublicPostFields = graphql(`
  fragment PublicPostFields on PublicPost {
    id
    title
    summary
    description
    location
    sourceUrl
    postType
    category
    createdAt
    publishedAt
    distanceMiles
    organizationId
    organizationName
    tags {
      kind
      value
      displayName
      color
    }
    urgentNotes {
      content
      ctaText
    }
  }
`);

export const PostDetailPublicFields = graphql(`
  fragment PostDetailPublicFields on Post {
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
