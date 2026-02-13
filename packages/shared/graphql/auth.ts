import { GraphQLError } from "graphql";
import type { GraphQLContext } from "./context";
import type { AuthUser } from "./restate-client";

export function requireAuth(ctx: GraphQLContext): AuthUser {
  if (!ctx.user) {
    throw new GraphQLError("Authentication required", {
      extensions: { code: "UNAUTHENTICATED" },
    });
  }
  return ctx.user;
}

export function requireAdmin(ctx: GraphQLContext): AuthUser {
  const user = requireAuth(ctx);
  if (!user.isAdmin) {
    throw new GraphQLError("Admin access required", {
      extensions: { code: "FORBIDDEN" },
    });
  }
  return user;
}
