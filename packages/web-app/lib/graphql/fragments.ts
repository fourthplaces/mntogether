import { graphql } from "@/gql";

export const PublicPostFields = graphql(`
  fragment PublicPostFields on PublicPost {
    id
    title
    bodyRaw
    bodyLight
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
    bodyRaw
    bodyAst
    status
    postType
    category
    urgency
    location
    sourceUrl
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
    relatedPosts {
      id
      title
      postType
      bodyLight
      tags {
        kind
        value
        displayName
        color
      }
    }
  }
`);
