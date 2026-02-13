import { graphql } from "@/gql";

export const OrganizationsListQuery = graphql(`
  query OrganizationsList {
    organizations {
      id
      name
      description
      status
      websiteCount
      socialProfileCount
      createdAt
    }
  }
`);

export const OrganizationDetailQuery = graphql(`
  query OrganizationDetail($id: ID!) {
    organization(id: $id) {
      id
      name
      description
      status
      websiteCount
      socialProfileCount
      snapshotCount
      createdAt
      updatedAt
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

export const ExtractOrgPostsMutation = graphql(`
  mutation ExtractOrgPosts($id: ID!) {
    extractOrgPosts(id: $id)
  }
`);

export const CleanUpOrgPostsMutation = graphql(`
  mutation CleanUpOrgPosts($id: ID!) {
    cleanUpOrgPosts(id: $id)
  }
`);

export const RunCuratorMutation = graphql(`
  mutation RunCurator($id: ID!) {
    runCurator(id: $id)
  }
`);

export const RemoveAllOrgPostsMutation = graphql(`
  mutation RemoveAllOrgPosts($id: ID!) {
    removeAllOrgPosts(id: $id)
  }
`);

export const RemoveAllOrgNotesMutation = graphql(`
  mutation RemoveAllOrgNotes($id: ID!) {
    removeAllOrgNotes(id: $id)
  }
`);

export const RewriteNarrativesMutation = graphql(`
  mutation RewriteNarratives($organizationId: ID!) {
    rewriteNarratives(organizationId: $organizationId) {
      rewritten
      failed
      total
    }
  }
`);
