import { graphql } from "@/gql";

export const PublicPostsQuery = graphql(`
  query PublicPosts(
    $postType: String
    $category: String
    $limit: Int
    $offset: Int
    $zipCode: String
    $radiusMiles: Float
  ) {
    publicPosts(
      postType: $postType
      category: $category
      limit: $limit
      offset: $offset
      zipCode: $zipCode
      radiusMiles: $radiusMiles
    ) {
      posts {
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
      totalCount
    }
  }
`);

export const PublicFiltersQuery = graphql(`
  query PublicFilters {
    publicFilters {
      categories {
        value
        displayName
        count
      }
      postTypes {
        value
        displayName
        description
        color
        emoji
      }
    }
  }
`);

export const PublicOrganizationsQuery = graphql(`
  query PublicOrganizations {
    publicOrganizations {
      id
      name
      description
      status
    }
  }
`);

export const PublicOrganizationQuery = graphql(`
  query PublicOrganization($id: ID!) {
    publicOrganization(id: $id) {
      id
      name
      description
      status
      posts {
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
    }
  }
`);

export const PostDetailPublicQuery = graphql(`
  query PostDetailPublic($id: ID!) {
    post(id: $id) {
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
  }
`);

export const TrackPostViewMutation = graphql(`
  mutation TrackPostView($postId: ID!) {
    trackPostView(postId: $postId)
  }
`);

export const TrackPostClickMutation = graphql(`
  mutation TrackPostClick($postId: ID!) {
    trackPostClick(postId: $postId)
  }
`);
