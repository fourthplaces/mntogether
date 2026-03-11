import { graphql } from "@/gql";
import "./fragments";

export const OrganizationsListQuery = graphql(`
  query OrganizationsList {
    organizations {
      ...OrganizationFields
    }
  }
`);

export const OrganizationDetailQuery = graphql(`
  query OrganizationDetail($id: ID!) {
    organization(id: $id) {
      ...OrganizationFields
    }
  }
`);

export const OrganizationDetailFullQuery = graphql(`
  query OrganizationDetailFull($id: ID!) {
    organization(id: $id) {
      ...OrganizationFields
      posts {
        posts {
          id
          title
          status
          postType
          createdAt
          organizationId
          organizationName
          tags {
            id
            kind
            value
            displayName
            color
          }
        }
        totalCount
        hasNextPage
        hasPreviousPage
      }
      notes {
        ...NoteFields
      }
      checklist {
        items {
          key
          label
          checked
          checkedBy
          checkedAt
        }
        allChecked
      }
    }
  }
`);

export const OrganizationChecklistQuery = graphql(`
  query OrganizationChecklist($id: ID!) {
    organizationChecklist(id: $id) {
      items {
        key
        label
        checked
        checkedBy
        checkedAt
      }
      allChecked
    }
  }
`);

export const CreateOrganizationMutation = graphql(`
  mutation CreateOrganization($name: String!, $description: String) {
    createOrganization(name: $name, description: $description) {
      id
      name
    }
  }
`);

export const UpdateOrganizationMutation = graphql(`
  mutation UpdateOrganization($id: ID!, $name: String!, $description: String) {
    updateOrganization(id: $id, name: $name, description: $description) {
      id
      name
      description
    }
  }
`);

export const DeleteOrganizationMutation = graphql(`
  mutation DeleteOrganization($id: ID!) {
    deleteOrganization(id: $id)
  }
`);

export const ApproveOrganizationMutation = graphql(`
  mutation ApproveOrganization($id: ID!) {
    approveOrganization(id: $id) {
      id
      status
    }
  }
`);

export const RejectOrganizationMutation = graphql(`
  mutation RejectOrganization($id: ID!, $reason: String!) {
    rejectOrganization(id: $id, reason: $reason) {
      id
      status
    }
  }
`);

export const SuspendOrganizationMutation = graphql(`
  mutation SuspendOrganization($id: ID!, $reason: String!) {
    suspendOrganization(id: $id, reason: $reason) {
      id
      status
    }
  }
`);

export const SetOrganizationStatusMutation = graphql(`
  mutation SetOrganizationStatus($id: ID!, $status: String!, $reason: String) {
    setOrganizationStatus(id: $id, status: $status, reason: $reason) {
      id
      status
    }
  }
`);

export const ToggleChecklistItemMutation = graphql(`
  mutation ToggleChecklistItem($organizationId: ID!, $checklistKey: String!, $checked: Boolean!) {
    toggleChecklistItem(organizationId: $organizationId, checklistKey: $checklistKey, checked: $checked) {
      items {
        key
        label
        checked
        checkedBy
        checkedAt
      }
      allChecked
    }
  }
`);

export const RegenerateOrganizationMutation = graphql(`
  mutation RegenerateOrganization($id: ID!) {
    regenerateOrganization(id: $id) {
      organizationId
      status
    }
  }
`);

