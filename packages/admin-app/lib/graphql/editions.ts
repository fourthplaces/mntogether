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

export const CountyDashboardQuery = graphql(`
  query CountyDashboard {
    countyDashboard {
      county {
        id
        name
        fipsCode
      }
      currentEdition {
        id
        periodStart
        periodEnd
        status
        publishedAt
        rows {
          id
        }
      }
      lastPublishedAt
      isStale
    }
  }
`);

export const EditionsListQuery = graphql(`
  query EditionsList(
    $countyId: ID
    $status: String
    $periodStart: String
    $periodEnd: String
    $limit: Int
    $offset: Int
  ) {
    editions(
      countyId: $countyId
      status: $status
      periodStart: $periodStart
      periodEnd: $periodEnd
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

export const LatestEditionsQuery = graphql(`
  query LatestEditions {
    latestEditions {
      id
      county {
        id
        name
      }
      periodStart
      periodEnd
      status
      publishedAt
      rows {
        id
      }
      createdAt
    }
  }
`);

export const EditionHistoryQuery = graphql(`
  query EditionHistory($countyId: ID, $limit: Int) {
    editions(countyId: $countyId, limit: $limit) {
      editions {
        id
        periodStart
        periodEnd
        status
        publishedAt
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
      sections {
        id
        title
        subtitle
        topicSlug
        sortOrder
      }
      rows {
        id
        sectionId
        rowTemplate {
          id
          slug
          displayName
          description
          layoutVariant
          slots {
            slotIndex
            weight
            count
            accepts
            postTemplateSlug
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

export const RowTemplatesQuery = graphql(`
  query RowTemplates {
    rowTemplates {
      id
      slug
      displayName
      description
      layoutVariant
      slots {
        slotIndex
        weight
        count
        accepts
        postTemplateSlug
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
      weight
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

export const MoveSlotMutation = graphql(`
  mutation MoveSlot($slotId: ID!, $targetRowId: ID!, $slotIndex: Int!) {
    moveSlot(slotId: $slotId, targetRowId: $targetRowId, slotIndex: $slotIndex) {
      id
      slotIndex
      postTemplate
    }
  }
`);

export const AddPostToEditionMutation = graphql(`
  mutation AddPostToEdition($editionRowId: ID!, $postId: ID!, $postTemplate: String!, $slotIndex: Int!) {
    addPostToEdition(editionRowId: $editionRowId, postId: $postId, postTemplate: $postTemplate, slotIndex: $slotIndex) {
      id
      slotIndex
      postTemplate
    }
  }
`);

export const AddEditionRowMutation = graphql(`
  mutation AddEditionRow($editionId: ID!, $rowTemplateSlug: String!, $sortOrder: Int!) {
    addEditionRow(editionId: $editionId, rowTemplateSlug: $rowTemplateSlug, sortOrder: $sortOrder) {
      id
      sortOrder
    }
  }
`);

export const DeleteEditionRowMutation = graphql(`
  mutation DeleteEditionRow($rowId: ID!) {
    deleteEditionRow(rowId: $rowId)
  }
`);

export const ReviewEditionMutation = graphql(`
  mutation ReviewEdition($id: ID!) {
    reviewEdition(id: $id) {
      id
      status
    }
  }
`);

export const ApproveEditionMutation = graphql(`
  mutation ApproveEdition($id: ID!) {
    approveEdition(id: $id) {
      id
      status
    }
  }
`);

export const BatchApproveEditionsMutation = graphql(`
  mutation BatchApproveEditions($ids: [ID!]!) {
    batchApproveEditions(ids: $ids) {
      succeeded
      failed
    }
  }
`);

export const BatchPublishEditionsMutation = graphql(`
  mutation BatchPublishEditions($ids: [ID!]!) {
    batchPublishEditions(ids: $ids) {
      succeeded
      failed
    }
  }
`);

export const EditionKanbanStatsQuery = graphql(`
  query EditionKanbanStats($periodStart: String!, $periodEnd: String!) {
    editionKanbanStats(periodStart: $periodStart, periodEnd: $periodEnd) {
      draft
      inReview
      approved
      published
    }
  }
`);

export const AddWidgetMutation = graphql(`
  mutation AddWidget($editionRowId: ID!, $widgetType: String!, $slotIndex: Int!, $config: String!) {
    addWidget(editionRowId: $editionRowId, widgetType: $widgetType, slotIndex: $slotIndex, config: $config) {
      id
      widgetType
      slotIndex
      config
    }
  }
`);

export const UpdateWidgetMutation = graphql(`
  mutation UpdateWidget($id: ID!, $config: String!) {
    updateWidget(id: $id, config: $config) {
      id
      widgetType
      slotIndex
      config
    }
  }
`);

export const RemoveWidgetMutation = graphql(`
  mutation RemoveWidget($id: ID!) {
    removeWidget(id: $id)
  }
`);

// ─── Section Mutations ────────────────────────────────────────────────────────

export const AddSectionMutation = graphql(`
  mutation AddSection($editionId: ID!, $title: String!, $subtitle: String, $topicSlug: String, $sortOrder: Int!) {
    addSection(editionId: $editionId, title: $title, subtitle: $subtitle, topicSlug: $topicSlug, sortOrder: $sortOrder) {
      id
      title
      subtitle
      topicSlug
      sortOrder
    }
  }
`);

export const UpdateSectionMutation = graphql(`
  mutation UpdateSection($id: ID!, $title: String, $subtitle: String, $topicSlug: String) {
    updateSection(id: $id, title: $title, subtitle: $subtitle, topicSlug: $topicSlug) {
      id
      title
      subtitle
      topicSlug
      sortOrder
    }
  }
`);

export const ReorderSectionsMutation = graphql(`
  mutation ReorderSections($editionId: ID!, $sectionIds: [ID!]!) {
    reorderSections(editionId: $editionId, sectionIds: $sectionIds) {
      id
      sortOrder
    }
  }
`);

export const DeleteSectionMutation = graphql(`
  mutation DeleteSection($id: ID!) {
    deleteSection(id: $id)
  }
`);

export const AssignRowToSectionMutation = graphql(`
  mutation AssignRowToSection($rowId: ID!, $sectionId: ID) {
    assignRowToSection(rowId: $rowId, sectionId: $sectionId)
  }
`);
