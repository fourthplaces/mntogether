import { graphql } from "@/gql";

export const PostListFields = graphql(`
  fragment PostListFields on Post {
    id
    title
    description
    summary
    status
    postType
    urgency
    location
    sourceUrl
    createdAt
    publishedAt
    distanceMiles
    organizationId
    organizationName
    submissionType
    weight
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
    bodyAst
    summary
    status
    postType
    category
    urgency
    location
    zipCode
    sourceUrl
    createdAt
    updatedAt
    publishedAt
    organizationId
    organizationName
    distanceMiles
    submissionType
    weight
    priority
    hasUrgentNotes
    bodyHeavy
    bodyMedium
    bodyLight
    latitude
    longitude
    revisionOfPostId
    translationOfId
    duplicateOfId
    sourceLanguage
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
    }
    urgentNotes {
      content
      ctaText
    }
    media {
      imageUrl
      caption
      credit
    }
    items {
      name
      detail
    }
    person {
      name
      role
      bio
      photoUrl
      quote
    }
    link {
      label
      url
      deadline
    }
    sourceAttribution {
      sourceName
      attribution
    }
    meta {
      kicker
      byline
      timestamp
      updated
      deck
    }
    datetime {
      start
      end
      cost
      recurring
    }
    postStatus {
      state
      verified
    }
    schedule {
      day
      opens
      closes
    }
  }
`);

export const OrganizationFields = graphql(`
  fragment OrganizationFields on Organization {
    id
    name
    description
    sourceType
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
    linkedOrgs {
      id
      name
    }
  }
`);
