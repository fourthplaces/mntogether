import { graphql } from "@/gql";

export const MediaLibraryQuery = graphql(`
  query MediaLibrary($limit: Int, $offset: Int, $contentType: String, $search: String, $unusedOnly: Boolean) {
    mediaLibrary(limit: $limit, offset: $offset, contentType: $contentType, search: $search, unusedOnly: $unusedOnly) {
      media {
        id
        filename
        contentType
        sizeBytes
        url
        storageKey
        altText
        width
        height
        createdAt
        updatedAt
        usageCount
      }
      totalCount
      hasNextPage
    }
  }
`);

export const MediaUsageQuery = graphql(`
  query MediaUsage($mediaId: ID!) {
    mediaUsage(mediaId: $mediaId) {
      referenceableType
      referenceableId
      fieldKey
      title
    }
  }
`);

export const PresignedUploadQuery = graphql(`
  query PresignedUpload($filename: String!, $contentType: String!, $sizeBytes: Int!) {
    presignedUpload(filename: $filename, contentType: $contentType, sizeBytes: $sizeBytes) {
      uploadUrl
      storageKey
      publicUrl
    }
  }
`);

export const ConfirmUploadMutation = graphql(`
  mutation ConfirmUpload(
    $storageKey: String!
    $publicUrl: String!
    $filename: String!
    $contentType: String!
    $sizeBytes: Int!
    $altText: String
    $width: Int
    $height: Int
  ) {
    confirmUpload(
      storageKey: $storageKey
      publicUrl: $publicUrl
      filename: $filename
      contentType: $contentType
      sizeBytes: $sizeBytes
      altText: $altText
      width: $width
      height: $height
    ) {
      id
      filename
      contentType
      sizeBytes
      url
      altText
      width
      height
      createdAt
    }
  }
`);

export const UpdateMediaMetadataMutation = graphql(`
  mutation UpdateMediaMetadata($id: ID!, $altText: String, $filename: String) {
    updateMediaMetadata(id: $id, altText: $altText, filename: $filename) {
      id
      filename
      altText
      updatedAt
    }
  }
`);

export const DeleteMediaMutation = graphql(`
  mutation DeleteMedia($id: ID!) {
    deleteMedia(id: $id)
  }
`);
