import * as awsx from "@pulumi/awsx";
import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";
import { config, getDomainAndSubdomain, getStandardTags } from "@infra/common";

const domainParts = getDomainAndSubdomain(config.domain);
const subdomain = "api";
const fullDomain = `${subdomain}.${domainParts.parentDomain}`;

// Get outputs from core stack
const coreStack = new pulumi.StackReference(`mndigitalaid-core-${config.stack}`);
const certificateArn = coreStack.getOutput("certificateArn");
const databaseSecretArn = coreStack.getOutput("databaseSecretArn");
const vpcId = coreStack.getOutput("vpcId");
const subnetIds = coreStack.getOutput("subnetIds");
const dbSecurityGroupId = coreStack.getOutput("dbSecurityGroupId");

// Get Route53 zone
const zoneId = aws.route53
  .getZone({
    name: domainParts.parentDomain,
  })
  .then((zone) => zone.id);

// Get Pulumi config for API image
const pulumiConfig = new pulumi.Config();
const apiImageTag = pulumiConfig.require("apiImageTag");
const ecrRepoName = pulumiConfig.require("ecrRepoName");

// Get ECR repository
const ecrRepo = aws.ecr.getRepository({ name: ecrRepoName });

// ========== CloudWatch Logs ==========

const apiLogGroup = new aws.cloudwatch.LogGroup("api-logs", {
  name: `/mndigitalaid/${config.stack}/api`,
  retentionInDays: config.isDev ? 7 : 30,
  tags: getStandardTags(),
});

// ========== Security Groups ==========

const albSecurityGroup = new aws.ec2.SecurityGroup("api-alb-sg", {
  vpcId,
  ingress: [
    {
      protocol: "tcp",
      fromPort: 80,
      toPort: 80,
      cidrBlocks: ["0.0.0.0/0"],
      description: "HTTP (redirects to HTTPS)",
    },
    {
      protocol: "tcp",
      fromPort: 443,
      toPort: 443,
      cidrBlocks: ["0.0.0.0/0"],
      description: "HTTPS",
    },
  ],
  egress: [
    { protocol: "-1", fromPort: 0, toPort: 0, cidrBlocks: ["0.0.0.0/0"] },
  ],
  tags: getStandardTags({ Name: "api-alb-sg" }),
});

const serviceSecurityGroup = new aws.ec2.SecurityGroup("api-service-sg", {
  vpcId,
  ingress: [
    {
      protocol: "tcp",
      fromPort: 8080,
      toPort: 8080,
      securityGroups: [albSecurityGroup.id],
      description: "HTTP from ALB",
    },
  ],
  egress: [
    { protocol: "-1", fromPort: 0, toPort: 0, cidrBlocks: ["0.0.0.0/0"] },
  ],
  tags: getStandardTags({ Name: "api-service-sg" }),
});

// Allow ECS to access database
new aws.ec2.SecurityGroupRule("ecs-to-db", {
  type: "ingress",
  fromPort: 5432,
  toPort: 5432,
  protocol: "tcp",
  securityGroupId: dbSecurityGroupId,
  sourceSecurityGroupId: serviceSecurityGroup.id,
  description: "PostgreSQL from ECS",
});

// ========== Application Load Balancer ==========

const alb = new aws.lb.LoadBalancer("api-alb", {
  internal: false,
  loadBalancerType: "application",
  securityGroups: [albSecurityGroup.id],
  subnets: subnetIds,
  enableDeletionProtection: config.isProd,
  tags: getStandardTags({ Name: "api-alb" }),
});

// Target group for ECS service
const targetGroup = new aws.lb.TargetGroup("api-tg", {
  port: 8080,
  protocol: "HTTP",
  vpcId,
  targetType: "ip",
  healthCheck: {
    enabled: true,
    path: "/health",
    protocol: "HTTP",
    matcher: "200",
    interval: 30,
    timeout: 5,
    healthyThreshold: 2,
    unhealthyThreshold: 3,
  },
  deregistrationDelay: 30,
  tags: getStandardTags(),
});

// HTTPS listener
const httpsListener = new aws.lb.Listener("api-https", {
  loadBalancerArn: alb.arn,
  port: 443,
  protocol: "HTTPS",
  certificateArn,
  sslPolicy: "ELBSecurityPolicy-TLS13-1-2-2021-06",
  defaultActions: [
    {
      type: "forward",
      targetGroupArn: targetGroup.arn,
    },
  ],
  tags: getStandardTags(),
});

// HTTP listener (redirect to HTTPS)
new aws.lb.Listener("api-http", {
  loadBalancerArn: alb.arn,
  port: 80,
  protocol: "HTTP",
  defaultActions: [
    {
      type: "redirect",
      redirect: {
        port: "443",
        protocol: "HTTPS",
        statusCode: "HTTP_301",
      },
    },
  ],
  tags: getStandardTags(),
});

// ========== ECS Cluster and Service ==========

const cluster = new aws.ecs.Cluster("api-cluster", {
  name: `mndigitalaid-${config.stack}`,
  settings: [
    {
      name: "containerInsights",
      value: config.isProd ? "enabled" : "disabled",
    },
  ],
  tags: getStandardTags(),
});

// Task execution role (for pulling images and writing logs)
const taskExecutionRole = new aws.iam.Role("task-execution-role", {
  assumeRolePolicy: JSON.stringify({
    Version: "2012-10-17",
    Statement: [
      {
        Action: "sts:AssumeRole",
        Effect: "Allow",
        Principal: {
          Service: "ecs-tasks.amazonaws.com",
        },
      },
    ],
  }),
  tags: getStandardTags(),
});

new aws.iam.RolePolicyAttachment("task-execution-role-policy", {
  role: taskExecutionRole.name,
  policyArn: "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy",
});

// Policy to read secrets
const secretsPolicy = new aws.iam.Policy("secrets-policy", {
  policy: pulumi.jsonStringify({
    Version: "2012-10-17",
    Statement: [
      {
        Effect: "Allow",
        Action: ["secretsmanager:GetSecretValue"],
        Resource: [databaseSecretArn],
      },
    ],
  }),
});

new aws.iam.RolePolicyAttachment("task-execution-secrets", {
  role: taskExecutionRole.name,
  policyArn: secretsPolicy.arn,
});

// Task role (for application permissions)
const taskRole = new aws.iam.Role("task-role", {
  assumeRolePolicy: JSON.stringify({
    Version: "2012-10-17",
    Statement: [
      {
        Action: "sts:AssumeRole",
        Effect: "Allow",
        Principal: {
          Service: "ecs-tasks.amazonaws.com",
        },
      },
    ],
  }),
  tags: getStandardTags(),
});

// Task definition
const taskDefinition = new aws.ecs.TaskDefinition("api-task", {
  family: `mndigitalaid-api-${config.stack}`,
  networkMode: "awsvpc",
  requiresCompatibilities: ["FARGATE"],
  cpu: config.isDev ? "256" : "512",
  memory: config.isDev ? "512" : "1024",
  executionRoleArn: taskExecutionRole.arn,
  taskRoleArn: taskRole.arn,
  containerDefinitions: pulumi.all([ecrRepo, apiImageTag, databaseSecretArn]).apply(
    ([repo, tag, secretArn]) =>
      JSON.stringify([
        {
          name: "api",
          image: `${repo.repositoryUrl}:${tag}`,
          essential: true,
          portMappings: [
            {
              containerPort: 8080,
              protocol: "tcp",
            },
          ],
          environment: [
            { name: "PORT", value: "8080" },
            { name: "RUST_LOG", value: config.isDev ? "debug" : "info" },
          ],
          secrets: [
            {
              name: "DATABASE_URL",
              valueFrom: `${secretArn}:host::`,
            },
          ],
          logConfiguration: {
            logDriver: "awslogs",
            options: {
              "awslogs-group": apiLogGroup.name,
              "awslogs-region": config.region,
              "awslogs-stream-prefix": "api",
            },
          },
        },
      ])
  ),
  tags: getStandardTags(),
});

// ECS Service
const service = new aws.ecs.Service("api-service", {
  cluster: cluster.arn,
  taskDefinition: taskDefinition.arn,
  desiredCount: config.isDev ? 1 : 2,
  launchType: "FARGATE",
  networkConfiguration: {
    subnets: subnetIds,
    securityGroups: [serviceSecurityGroup.id],
    assignPublicIp: true,
  },
  loadBalancers: [
    {
      targetGroupArn: targetGroup.arn,
      containerName: "api",
      containerPort: 8080,
    },
  ],
  healthCheckGracePeriodSeconds: 60,
  tags: getStandardTags(),
});

// ========== Route53 DNS Record ==========

new aws.route53.Record(`${fullDomain}-alias`, {
  name: fullDomain,
  zoneId,
  type: "A",
  aliases: [
    {
      name: alb.dnsName,
      zoneId: alb.zoneId,
      evaluateTargetHealth: true,
    },
  ],
});

// ========== Exports ==========

export const albDnsName = alb.dnsName;
export const targetUrl = `https://${fullDomain}`;
export const clusterName = cluster.name;
export const serviceName = service.name;
export const logGroupName = apiLogGroup.name;
