"use client";

import { useMemo, type ReactNode } from "react";
import {
  UrqlProvider,
  ssrExchange,
  cacheExchange,
  fetchExchange,
  createClient,
} from "@urql/next";

export default function GraphQLProvider({
  children,
}: {
  children: ReactNode;
}) {
  const [client, ssr] = useMemo(() => {
    const ssr = ssrExchange({
      isClient: typeof window !== "undefined",
    });
    const isServer = typeof window === "undefined";
    const url = isServer
      ? `http://localhost:${process.env.PORT || 3001}/api/graphql`
      : "/api/graphql";

    const client = createClient({
      url,
      exchanges: [cacheExchange, ssr, fetchExchange],
      suspense: false,
      fetchOptions: { credentials: "same-origin" },
    });
    return [client, ssr] as const;
  }, []);

  return (
    <UrqlProvider client={client} ssr={ssr}>
      {children}
    </UrqlProvider>
  );
}
