import { graphql } from "@/gql";

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
            description
            postType
            weight
            urgency
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
            description
            postType
            weight
            urgency
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
