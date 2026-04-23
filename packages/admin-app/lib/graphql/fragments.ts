import { graphql } from "@/gql";

export const PostListFields = graphql(`
  fragment PostListFields on Post {
    id
    title
    bodyRaw
    status
    postType
    isUrgent
    isSeed
    location
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
    bodyRaw
    bodyAst
    status
    postType
    isUrgent
    isSeed
    location
    zipCode
    createdAt
    updatedAt
    publishedAt
    organizationId
    organizationName
    distanceMiles
    submissionType
    weight
    priority
    pencilMark
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
      mediaId
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
      photoMediaId
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
      id
      day
      opens
      closes
    }
    sources {
      id
      sourceUrl
      kind
      organizationId
      organizationName
      individualId
      individualDisplayName
      retrievedAt
      contentHash
      snippet
      confidence
      platformId
      platformPostTypeHint
      isPrimary
      firstSeenAt
      lastSeenAt
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
    isSeed
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
