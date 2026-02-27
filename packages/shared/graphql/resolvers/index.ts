import { mergeResolvers } from "@graphql-tools/merge";
import { postResolvers } from "./post";
import { tagResolvers } from "./tag";
import { organizationResolvers } from "./organization";
import { jobResolvers } from "./job";
import { noteResolvers } from "./note";
import { editionResolvers } from "./edition";
import { mediaResolvers } from "./media";

export const resolvers = mergeResolvers([
  postResolvers,
  tagResolvers,
  organizationResolvers,
  jobResolvers,
  noteResolvers,
  editionResolvers,
  mediaResolvers,
]);
