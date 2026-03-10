import { graphql } from "@/gql";

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
      rows {
        rowTemplateSlug
        sortOrder
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
