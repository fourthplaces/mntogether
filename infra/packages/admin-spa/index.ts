import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";
import * as path from "path";
import { globSync } from "glob";
import mime from "mime";
import * as fs from "fs";
import { config, packagesDir, getDomainAndSubdomain, getStandardTags } from "@infra/common";

const adminSpaDir = path.join(packagesDir, "admin-spa", "dist");
const TEN_MINUTES = 60 * 10;

// Get certificate ARN from core stack
const coreStack = new pulumi.StackReference(`mndigitalaid-core-${config.stack}`);
const certificateArn = coreStack.getOutput("certificateArn");

const domainParts = getDomainAndSubdomain(config.domain);
const subdomain = "admin";
const fullDomain = `${subdomain}.${domainParts.parentDomain}`;

// Get Route53 zone
const zoneId = aws.route53
  .getZone({
    name: domainParts.parentDomain,
  })
  .then((zone) => zone.id);

// Create S3 bucket for static site
const siteBucket = new aws.s3.Bucket(
  `${subdomain}-${domainParts.parentDomain}`,
  {
    website: {
      indexDocument: "index.html",
      errorDocument: "index.html",
    },
    serverSideEncryptionConfiguration: {
      rule: {
        applyServerSideEncryptionByDefault: {
          sseAlgorithm: "AES256",
        },
      },
    },
    tags: getStandardTags({ Name: "admin-spa-bucket" }),
  }
);

// Upload static assets
function uploadStaticAssets(sitePath: string, bucket: aws.s3.Bucket) {
  if (!fs.existsSync(sitePath)) {
    console.warn(`Warning: ${sitePath} does not exist. Skipping asset upload.`);
    return;
  }

  const allAssets = globSync(sitePath + "/**/*");

  for (const assetPath of allAssets) {
    const relativePath = assetPath.replace(sitePath + "/", "");
    if (fs.lstatSync(assetPath).isDirectory()) {
      continue;
    }

    console.log("Uploading ", assetPath);
    new aws.s3.BucketObject(
      `admin-${relativePath}`,
      {
        bucket: bucket.bucket,
        source: new pulumi.asset.FileAsset(assetPath),
        contentType: mime.getType(assetPath) ?? undefined,
      }
    );
  }
}

uploadStaticAssets(adminSpaDir, siteBucket);

// Create CloudFront Origin Access Identity
const originAccessIdentity = new aws.cloudfront.OriginAccessIdentity(
  "admin-oai",
  {
    comment: `OAI for ${fullDomain} S3 bucket`,
  }
);

// Create CloudFront distribution
const distribution = new aws.cloudfront.Distribution("admin-cdn", {
  enabled: true,
  aliases: [fullDomain],

  origins: [
    {
      originId: siteBucket.arn,
      domainName: siteBucket.bucketRegionalDomainName,
      s3OriginConfig: {
        originAccessIdentity: originAccessIdentity.cloudfrontAccessIdentityPath,
      },
    },
  ],

  defaultRootObject: "index.html",

  defaultCacheBehavior: {
    targetOriginId: siteBucket.arn,
    viewerProtocolPolicy: "redirect-to-https",
    allowedMethods: ["GET", "HEAD", "OPTIONS"],
    cachedMethods: ["GET", "HEAD", "OPTIONS"],

    forwardedValues: {
      cookies: { forward: "none" },
      queryString: false,
    },

    minTtl: 0,
    defaultTtl: TEN_MINUTES,
    maxTtl: TEN_MINUTES,
  },

  priceClass: "PriceClass_100",

  // SPA routing - return index.html for 404/403
  customErrorResponses: [
    {
      errorCode: 404,
      responseCode: 200,
      responsePagePath: "/index.html",
      errorCachingMinTtl: 60,
    },
    {
      errorCode: 403,
      responseCode: 200,
      responsePagePath: "/index.html",
      errorCachingMinTtl: 60,
    },
  ],

  restrictions: {
    geoRestriction: {
      restrictionType: "none",
    },
  },

  viewerCertificate: {
    acmCertificateArn: certificateArn,
    sslSupportMethod: "sni-only",
  },

  tags: getStandardTags({ Name: "admin-cdn" }),
});

// Grant CloudFront access to S3 bucket
new aws.s3.BucketPolicy("admin-bucket-policy", {
  bucket: siteBucket.id,
  policy: pulumi.jsonStringify({
    Version: "2012-10-17",
    Statement: [
      {
        Effect: "Allow",
        Principal: {
          AWS: originAccessIdentity.iamArn,
        },
        Action: ["s3:GetObject"],
        Resource: [pulumi.interpolate`${siteBucket.arn}/*`],
      },
    ],
  }),
});

// Create Route53 alias record
new aws.route53.Record(`${fullDomain}-alias`, {
  name: fullDomain,
  zoneId,
  type: "A",
  aliases: [
    {
      name: distribution.domainName,
      zoneId: distribution.hostedZoneId,
      evaluateTargetHealth: true,
    },
  ],
});

// Exports
export const bucketName = siteBucket.bucket;
export const cloudFrontDomain = distribution.domainName;
export const cloudFrontDistributionId = distribution.id;
export const targetUrl = `https://${fullDomain}/`;
