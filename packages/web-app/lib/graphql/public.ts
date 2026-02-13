import { graphql } from "@/gql";
import "./fragments";

export const PublicPostsQuery = graphql(`
  query PublicPosts(
    $postType: String
    $category: String
    $limit: Int
    $offset: Int
    $zipCode: String
    $radiusMiles: Float
  ) {
    publicPosts(
      postType: $postType
      category: $category
      limit: $limit
      offset: $offset
      zipCode: $zipCode
      radiusMiles: $radiusMiles
    ) {
      posts {
        ...PublicPostFields
      }
      totalCount
    }
  }
`);

export const PublicFiltersQuery = graphql(`
  query PublicFilters {
    publicFilters {
      categories {
        value
        displayName
        count
      }
      postTypes {
        value
        displayName
        description
        color
        emoji
      }
    }
  }
`);

export const PublicOrganizationsQuery = graphql(`
  query PublicOrganizations {
    publicOrganizations {
      id
      name
      description
      status
    }
  }
`);

export const PublicOrganizationQuery = graphql(`
  query PublicOrganization($id: ID!) {
    publicOrganization(id: $id) {
      id
      name
      description
      status
      posts {
        ...PublicPostFields
      }
    }
  }
`);

export const PostDetailPublicQuery = graphql(`
  query PostDetailPublic($id: ID!) {
    post(id: $id) {
      ...PostDetailPublicFields
    }
  }
`);

export const TrackPostViewMutation = graphql(`
  mutation TrackPostView($postId: ID!) {
    trackPostView(postId: $postId)
  }
`);

export const TrackPostClickMutation = graphql(`
  mutation TrackPostClick($postId: ID!) {
    trackPostClick(postId: $postId)
  }
`);
