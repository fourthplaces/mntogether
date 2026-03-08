import type { CodegenConfig } from "@graphql-codegen/cli";

const config: CodegenConfig = {
  schema: "../shared/graphql/schema.ts",
  documents: [
    "./app/**/*.tsx",
    "./app/**/*.ts",
    "./components/**/*.tsx",
    "./components/**/*.ts",
    "./lib/**/*.ts",
    "!./gql/**/*",
  ],
  ignoreNoDocuments: true,
  generates: {
    "./gql/": {
      preset: "client",
      presetConfig: { fragmentMasking: false },
      config: {
        scalars: { DateTime: "string", UUID: "string" },
        enumsAsTypes: true,
      },
    },
  },
};

export default config;
