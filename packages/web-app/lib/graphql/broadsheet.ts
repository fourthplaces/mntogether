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
          postTemplate
          slotIndex
          post {
            id
            title
            description
            postType
            weight
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
          }
        }
        widgets {
          id
          widgetType
          slotIndex
          config
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
          postTemplate
          slotIndex
          post {
            id
            title
            description
            postType
            weight
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
          }
        }
        widgets {
          id
          widgetType
          slotIndex
          config
        }
      }
    }
  }
`);
