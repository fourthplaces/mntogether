import { graphql } from "@/gql";

/**
 * Post template configs — body length limits, title limits, weight, type
 * compatibility. Fetched once at page-level and threaded into preparePost
 * so the renderer doesn't hardcode a duplicate of post_template_configs.
 */
export const PostTemplateConfigsQuery = graphql(`
  query PostTemplateConfigs {
    postTemplates {
      id
      slug
      bodyTarget
      bodyMax
      titleMax
      weight
      compatibleTypes
    }
  }
`);

export const EditionPreviewQuery = graphql(`
  query EditionPreview($editionId: ID!) {
    editionPreview(editionId: $editionId) {
      id
      title
      periodStart
      periodEnd
      status
      publishedAt
      county {
        id
        fipsCode
        name
        state
      }
      sections {
        id
        title
        subtitle
        topicSlug
        sortOrder
      }
      rows {
        rowTemplateSlug
        layoutVariant
        sortOrder
        sectionId
        slots {
          kind
          postTemplate
          widgetTemplate
          slotIndex
          post {
            id
            title
            bodyRaw
            postType
            weight
            urgency
            isUrgent
            pencilMark
            location
            sourceUrl
            organizationName
            publishedAt
            tags {
              kind
              value
              displayName
              color
            }
            contacts {
              contactType
              contactValue
              contactLabel
            }
            urgentNotes {
              content
              ctaText
            }
            bodyHeavy
            bodyMedium
            bodyLight
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
          widget {
            id
            widgetType
            authoringMode
            data
          }
        }
      }
    }
  }
`);

export const PublicBroadsheetQuery = graphql(`
  query PublicBroadsheet($countyId: ID!) {
    publicBroadsheet(countyId: $countyId) {
      id
      title
      periodStart
      periodEnd
      status
      publishedAt
      county {
        id
        fipsCode
        name
        state
      }
      sections {
        id
        title
        subtitle
        topicSlug
        sortOrder
      }
      rows {
        rowTemplateSlug
        layoutVariant
        sortOrder
        sectionId
        slots {
          kind
          postTemplate
          widgetTemplate
          slotIndex
          post {
            id
            title
            bodyRaw
            postType
            weight
            urgency
            isUrgent
            pencilMark
            location
            sourceUrl
            organizationName
            publishedAt
            tags {
              kind
              value
              displayName
              color
            }
            contacts {
              contactType
              contactValue
              contactLabel
            }
            urgentNotes {
              content
              ctaText
            }
            bodyHeavy
            bodyMedium
            bodyLight
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
          widget {
            id
            widgetType
            authoringMode
            data
          }
        }
      }
    }
  }
`);
