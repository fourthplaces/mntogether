import * as pulumi from "@pulumi/pulumi";
import * as path from "path";

// Get Pulumi configuration
export const pulumiConfig = new pulumi.Config();
export const awsConfig = new pulumi.Config("aws");

// Stack configuration
export const stack = pulumi.getStack();
export const isDev = stack === "dev";
export const isProd = stack === "prod";

// Domain configuration
export const domain = pulumiConfig.require("domain");

// Get monorepo root
export const rootDir = path.resolve(__dirname, "../../../..");
export const packagesDir = path.join(rootDir, "packages");

export interface Config {
  stack: string;
  domain: string;
  region: string;
  isDev: boolean;
  isProd: boolean;
}

export const config: Config = {
  stack,
  domain,
  region: awsConfig.require("region"),
  isDev,
  isProd,
};

/**
 * Split domain into subdomain and parent domain
 * Example: "admin.mndigitalaid.org" -> { subdomain: "admin", parentDomain: "mndigitalaid.org" }
 */
export function getDomainAndSubdomain(fullDomain: string) {
  const parts = fullDomain.split(".");
  if (parts.length < 2) {
    throw new Error(`Invalid domain: ${fullDomain}`);
  }

  // If 2 parts, no subdomain
  if (parts.length === 2) {
    return {
      subdomain: undefined,
      parentDomain: fullDomain,
    };
  }

  // Otherwise, first part is subdomain, rest is parent
  const subdomain = parts[0];
  const parentDomain = parts.slice(1).join(".");

  return { subdomain, parentDomain };
}

/**
 * Get standard tags for all resources
 */
export function getStandardTags(additionalTags?: Record<string, string>) {
  return {
    Project: "mndigitalaid",
    Environment: stack,
    ManagedBy: "pulumi",
    ...additionalTags,
  };
}
