import { graphql } from "@/gql";

export const SyncBatchesQuery = graphql(`
  query SyncBatches($status: String, $limit: Int) {
    syncBatches(status: $status, limit: $limit) {
      batches {
        id
        resourceType
        sourceId
        sourceName
        status
        summary
        proposalCount
        approvedCount
        rejectedCount
        createdAt
        reviewedAt
      }
    }
  }
`);

export const SyncProposalsQuery = graphql(`
  query SyncProposals($batchId: ID!) {
    syncProposals(batchId: $batchId) {
      proposals {
        id
        batchId
        operation
        status
        entityType
        draftEntityId
        targetEntityId
        reason
        reviewedBy
        reviewedAt
        createdAt
        draftTitle
        targetTitle
        mergeSourceIds
        mergeSourceTitles
        relevanceScore
        curatorReasoning
        confidence
        sourceUrls
        revisionCount
      }
    }
  }
`);

export const ApproveProposalMutation = graphql(`
  mutation ApproveProposal($id: ID!) {
    approveProposal(id: $id)
  }
`);

export const RejectProposalMutation = graphql(`
  mutation RejectProposal($id: ID!) {
    rejectProposal(id: $id)
  }
`);

export const ApproveBatchMutation = graphql(`
  mutation ApproveBatch($id: ID!) {
    approveBatch(id: $id)
  }
`);

export const RejectBatchMutation = graphql(`
  mutation RejectBatch($id: ID!) {
    rejectBatch(id: $id)
  }
`);

export const RefineProposalMutation = graphql(`
  mutation RefineProposal($proposalId: ID!, $comment: String!) {
    refineProposal(proposalId: $proposalId, comment: $comment)
  }
`);
