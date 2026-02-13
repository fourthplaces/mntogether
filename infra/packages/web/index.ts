import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";
import { config, getDomainAndSubdomain, getStandardTags } from "@infra/common";

const domainParts = getDomainAndSubdomain(config.domain);
const subdomain = "www";
const fullDomain = `${subdomain}.${domainParts.parentDomain}`;

// Get outputs from core stack
const coreStack = new pulumi.StackReference(`mndigitalaid-core-${config.stack}`);
const certificateArn = coreStack.getOutput("certificateArn");
const vpcId = coreStack.getOutput("vpcId");
const subnetIds = coreStack.getOutput("subnetIds");

// Get Route53 zone
const zoneId = aws.route53
  .getZone({
    name: domainParts.parentDomain,
  })
  .then((zone) => zone.id);

// Get Pulumi config for Next.js image
const pulumiConfig = new pulumi.Config();
const imageTag = pulumiConfig.require("imageTag");
const ecrRepoName = pulumiConfig.require("ecrRepoName");
const apiUrl = pulumiConfig.require("apiUrl");

// Get ECR repository
const ecrRepo = aws.ecr.getRepository({ name: ecrRepoName });

// ========== CloudWatch Logs ==========

const logGroup = new aws.cloudwatch.LogGroup("web-logs", {
  name: `/mndigitalaid/${config.stack}/web`,
  retentionInDays: config.isDev ? 7 : 30,
  tags: getStandardTags(),
});

// ========== Security Groups ==========

const albSecurityGroup = new aws.ec2.SecurityGroup("web-alb-sg", {
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
  tags: getStandardTags({ Name: "web-alb-sg" }),
});

const serviceSecurityGroup = new aws.ec2.SecurityGroup("web-service-sg", {
  vpcId,
  ingress: [
    {
      protocol: "tcp",
      fromPort: 3000,
      toPort: 3000,
      securityGroups: [albSecurityGroup.id],
      description: "HTTP from ALB",
    },
  ],
  egress: [
    { protocol: "-1", fromPort: 0, toPort: 0, cidrBlocks: ["0.0.0.0/0"] },
  ],
  tags: getStandardTags({ Name: "web-service-sg" }),
});

// ========== Application Load Balancer ==========

const alb = new aws.lb.LoadBalancer("web-alb", {
  internal: false,
  loadBalancerType: "application",
  securityGroups: [albSecurityGroup.id],
  subnets: subnetIds,
  enableDeletionProtection: config.isProd,
  tags: getStandardTags({ Name: "web-alb" }),
});

// Target group for ECS service
const targetGroup = new aws.lb.TargetGroup("web-tg", {
  port: 3000,
  protocol: "HTTP",
  vpcId,
  targetType: "ip",
  healthCheck: {
    enabled: true,
    path: "/",
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
new aws.lb.Listener("web-https", {
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
new aws.lb.Listener("web-http", {
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

const cluster = new aws.ecs.Cluster("web-cluster", {
  name: `mndigitalaid-web-${config.stack}`,
  settings: [
    {
      name: "containerInsights",
      value: config.isProd ? "enabled" : "disabled",
    },
  ],
  tags: getStandardTags(),
});

// Task execution role
const taskExecutionRole = new aws.iam.Role("web-task-execution-role", {
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

new aws.iam.RolePolicyAttachment("web-task-execution-role-policy", {
  role: taskExecutionRole.name,
  policyArn: "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy",
});

// Task role
const taskRole = new aws.iam.Role("web-task-role", {
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
const taskDefinition = new aws.ecs.TaskDefinition("web-task", {
  family: `mndigitalaid-web-${config.stack}`,
  networkMode: "awsvpc",
  requiresCompatibilities: ["FARGATE"],
  cpu: config.isDev ? "256" : "512",
  memory: config.isDev ? "512" : "1024",
  executionRoleArn: taskExecutionRole.arn,
  taskRoleArn: taskRole.arn,
  containerDefinitions: pulumi.all([ecrRepo, imageTag]).apply(([repo, tag]) =>
    JSON.stringify([
      {
        name: "web",
        image: `${repo.repositoryUrl}:${tag}`,
        essential: true,
        portMappings: [
          {
            containerPort: 3000,
            protocol: "tcp",
          },
        ],
        environment: [
          { name: "PORT", value: "3000" },
          { name: "NODE_ENV", value: "production" },
          { name: "NEXT_PUBLIC_API_URL", value: apiUrl },
        ],
        logConfiguration: {
          logDriver: "awslogs",
          options: {
            "awslogs-group": logGroup.name,
            "awslogs-region": config.region,
            "awslogs-stream-prefix": "web",
          },
        },
      },
    ])
  ),
  tags: getStandardTags(),
});

// ECS Service
const service = new aws.ecs.Service("web-service", {
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
      containerName: "web",
      containerPort: 3000,
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

// Also create record for root domain
new aws.route53.Record(`${domainParts.parentDomain}-alias`, {
  name: domainParts.parentDomain,
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
export const rootUrl = `https://${domainParts.parentDomain}`;
export const clusterName = cluster.name;
export const serviceName = service.name;
export const logGroupName = logGroup.name;
