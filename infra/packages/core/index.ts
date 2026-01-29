import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";
import * as random from "@pulumi/random";
import { config, getStandardTags, getDomainAndSubdomain } from "@infra/common";

const domainParts = getDomainAndSubdomain(config.domain);

// ========== ACM Certificate ==========

// Get Route53 hosted zone
const zoneId = aws.route53
  .getZone({
    name: domainParts.parentDomain,
  })
  .then((zone) => zone.id);

// Request wildcard certificate for all subdomains
const cert = new aws.acm.Certificate("mndigitalaid-cert", {
  domainName: `*.${domainParts.parentDomain}`,
  subjectAlternativeNames: [domainParts.parentDomain],
  validationMethod: "DNS",
  tags: getStandardTags(),
});

// Create DNS validation records
const certValidationRecords: aws.route53.Record[] = [];
cert.domainValidationOptions.apply((options) => {
  const distinctValidationOptions = options.filter(
    (option, index, self) =>
      index === self.findIndex((o) => o.resourceRecordName === option.resourceRecordName)
  );

  distinctValidationOptions.forEach((option, index) => {
    const record = new aws.route53.Record(`cert-validation-${index}`, {
      zoneId,
      name: option.resourceRecordName,
      type: option.resourceRecordType,
      records: [option.resourceRecordValue],
      ttl: 60,
    });
    certValidationRecords.push(record);
  });
});

// Wait for certificate validation
const certValidation = new aws.acm.CertificateValidation("mndigitalaid-cert-validation", {
  certificateArn: cert.arn,
  validationRecordFqdns: pulumi.all(certValidationRecords.map((r) => r.fqdn)),
});

// ========== RDS PostgreSQL Database ==========

// Get default VPC and subnets
const defaultVpc = await aws.ec2.getVpc({ default: true });
const defaultSubnets = await aws.ec2.getSubnets({
  filters: [{ name: "vpc-id", values: [defaultVpc.id] }],
});

// Security group for RDS
const dbSecurityGroup = new aws.ec2.SecurityGroup("mndigitalaid-db-sg", {
  vpcId: defaultVpc.id,
  description: "Allow PostgreSQL access from ECS tasks",
  ingress: [
    {
      protocol: "tcp",
      fromPort: 5432,
      toPort: 5432,
      cidrBlocks: [defaultVpc.cidrBlock],
      description: "PostgreSQL from VPC",
    },
  ],
  egress: [
    {
      protocol: "-1",
      fromPort: 0,
      toPort: 0,
      cidrBlocks: ["0.0.0.0/0"],
    },
  ],
  tags: getStandardTags({ Name: "mndigitalaid-db-sg" }),
});

// DB subnet group
const dbSubnetGroup = new aws.rds.SubnetGroup("mndigitalaid-db-subnet", {
  subnetIds: defaultSubnets.ids,
  tags: getStandardTags(),
});

// Generate random password for RDS
const dbPassword = new random.RandomPassword("db-password", {
  length: 32,
  special: true,
  overrideSpecial: "!#$%&*()-_=+[]{}<>:?",
});

// RDS PostgreSQL instance
const dbInstance = new aws.rds.Instance("mndigitalaid-db", {
  engine: "postgres",
  engineVersion: "16.4",
  instanceClass: config.isDev ? "db.t3.micro" : "db.t3.small",
  allocatedStorage: config.isDev ? 20 : 50,
  storageType: "gp3",
  dbName: "mndigitalaid",
  username: "postgres",
  password: dbPassword.result,
  vpcSecurityGroupIds: [dbSecurityGroup.id],
  dbSubnetGroupName: dbSubnetGroup.name,
  skipFinalSnapshot: config.isDev,
  finalSnapshotIdentifier: config.isDev ? undefined : `mndigitalaid-final-${Date.now()}`,
  backupRetentionPeriod: config.isDev ? 1 : 7,
  publiclyAccessible: false,
  storageEncrypted: true,
  multiAz: config.isProd,
  enabledCloudwatchLogsExports: ["postgresql"],
  tags: getStandardTags({ Name: "mndigitalaid-db" }),
});

// Store DB credentials in Secrets Manager
const dbSecret = new aws.secretsmanager.Secret("mndigitalaid-db-secret", {
  name: `mndigitalaid/${config.stack}/database`,
  description: "PostgreSQL database credentials",
  tags: getStandardTags(),
});

const dbSecretVersion = new aws.secretsmanager.SecretVersion("mndigitalaid-db-secret-version", {
  secretId: dbSecret.id,
  secretString: pulumi.jsonStringify({
    host: dbInstance.address,
    port: dbInstance.port,
    database: dbInstance.dbName,
    username: dbInstance.username,
    password: dbPassword.result,
  }),
});

// ========== Exports ==========

export const certificateArn = certValidation.certificateArn;
export const databaseSecretArn = dbSecret.arn;
export const databaseEndpoint = dbInstance.endpoint;
export const databaseName = dbInstance.dbName;
export const vpcId = defaultVpc.id;
export const subnetIds = defaultSubnets.ids;
export const dbSecurityGroupId = dbSecurityGroup.id;
