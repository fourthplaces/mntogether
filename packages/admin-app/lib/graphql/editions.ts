import { graphql } from "@/gql";
import "./fragments";

// ─── Queries ─────────────────────────────────────────────────────────────────

export const CountiesQuery = graphql(`
  query Counties {
    counties {
      id
      fipsCode
      name
      state
    }
  }
`);

export const EditionsListQuery = graphql(`
  query EditionsList(
    $countyId: ID
    $status: String
    $limit: Int
    $offset: Int
  ) {
    editions(
      countyId: $countyId
      status: $status
      limit: $limit
      offset: $offset
    ) {
      editions {
        id
        county {
          id
          name
        }
        title
        periodStart
        periodEnd
        status
        publishedAt
        createdAt
        rows {
          id
        }
      }
      totalCount
    }
  }
`);

export const EditionDetailQuery = graphql(`
  query EditionDetail($id: ID!) {
    edition(id: $id) {
      id
      county {
        id
        fipsCode
        name
        state
      }
      title
      periodStart
      periodEnd
      status
      publishedAt
      createdAt
      rows {
        id
        rowTemplate {
          id
          slug
          displayName
          description
          slots {
            slotIndex
            weight
            count
            accepts
          }
        }
        sortOrder
        slots {
          id
          post {
            id
            title
            postType
            weight
            status
          }
          postTemplate
          slotIndex
        }
      }
    }
  }
`);

export const RowTemplatesQuery = graphql(`
  query RowTemplates {
    rowTemplates {
      id
      slug
      displayName
      description
      slots {
        slotIndex
        weight
        count
        accepts
      }
    }
  }
`);

export const PostTemplatesQuery = graphql(`
  query PostTemplates {
    postTemplates {
      id
      slug
      displayName
      description
      compatibleTypes
      bodyTarget
      bodyMax
      titleMax
    }
  }
`);

// ─── Mutations ───────────────────────────────────────────────────────────────

export const CreateEditionMutation = graphql(`
  mutation CreateEdition(
    $countyId: ID!
    $periodStart: String!
    $periodEnd: String!
    $title: String
  ) {
    createEdition(
      countyId: $countyId
      periodStart: $periodStart
      periodEnd: $periodEnd
      title: $title
    ) {
      id
      status
    }
  }
`);

export const GenerateEditionMutation = graphql(`
  mutation GenerateEdition($id: ID!) {
    generateEdition(id: $id) {
      id
      status
    }
  }
`);

export const PublishEditionMutation = graphql(`
  mutation PublishEdition($id: ID!) {
    publishEdition(id: $id) {
      id
      status
      publishedAt
    }
  }
`);

export const ArchiveEditionMutation = graphql(`
  mutation ArchiveEdition($id: ID!) {
    archiveEdition(id: $id) {
      id
      status
    }
  }
`);

export const BatchGenerateEditionsMutation = graphql(`
  mutation BatchGenerateEditions($periodStart: String!, $periodEnd: String!) {
    batchGenerateEditions(periodStart: $periodStart, periodEnd: $periodEnd) {
      created
      failed
      totalCounties
    }
  }
`);

export const ReorderEditionRowsMutation = graphql(`
  mutation ReorderEditionRows($editionId: ID!, $rowIds: [ID!]!) {
    reorderEditionRows(editionId: $editionId, rowIds: $rowIds) {
      id
      sortOrder
    }
  }
`);

export const RemovePostFromEditionMutation = graphql(`
  mutation RemovePostFromEdition($slotId: ID!) {
    removePostFromEdition(slotId: $slotId)
  }
`);

export const ChangeSlotTemplateMutation = graphql(`
  mutation ChangeSlotTemplate($slotId: ID!, $postTemplate: String!) {
    changeSlotTemplate(slotId: $slotId, postTemplate: $postTemplate) {
      id
      postTemplate
    }
  }
`);
