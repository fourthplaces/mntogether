import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";
import { config, getStandardTags } from "@infra/common";

// Get outputs from core stack
const coreStack = new pulumi.StackReference(`mndigitalaid-core-${config.stack}`);
const vpcId = coreStack.getOutput("vpcId");
const subnetIds = coreStack.getOutput("subnetIds");

// ========== CloudWatch Logs ==========

const natsLogGroup = new aws.cloudwatch.LogGroup("nats-logs", {
  name: `/mndigitalaid/${config.stack}/nats`,
  retentionInDays: config.isDev ? 7 : 30,
  tags: getStandardTags(),
});

// ========== Security Groups ==========

const natsSecurityGroup = new aws.ec2.SecurityGroup("nats-sg", {
  vpcId,
  description: "NATS messaging server",
  ingress: [
    {
      protocol: "tcp",
      fromPort: 4222,
      toPort: 4222,
      cidrBlocks: ["10.0.0.0/8"], // VPC internal only
      description: "NATS client connections",
    },
    {
      protocol: "tcp",
      fromPort: 8222,
      toPort: 8222,
      cidrBlocks: ["10.0.0.0/8"], // VPC internal only
      description: "NATS monitoring HTTP",
    },
    {
      protocol: "tcp",
      fromPort: 6222,
      toPort: 6222,
      cidrBlocks: ["10.0.0.0/8"], // VPC internal only
      description: "NATS clustering",
    },
  ],
  egress: [
    { protocol: "-1", fromPort: 0, toPort: 0, cidrBlocks: ["0.0.0.0/0"] },
  ],
  tags: getStandardTags({ Name: "nats-sg" }),
});

// ========== EFS for NATS JetStream Persistence ==========

const natsFileSystem = new aws.efs.FileSystem("nats-efs", {
  encrypted: true,
  performanceMode: "generalPurpose",
  throughputMode: "bursting",
  lifecyclePolicies: config.isProd
    ? [{ transitionToIa: "AFTER_30_DAYS" }]
    : undefined,
  tags: getStandardTags({ Name: "nats-jetstream-data" }),
});

// Mount targets in each subnet
const mountTargets = subnetIds.apply((ids: string[]) =>
  ids.map(
    (subnetId, i) =>
      new aws.efs.MountTarget(`nats-efs-mount-${i}`, {
        fileSystemId: natsFileSystem.id,
        subnetId,
        securityGroups: [natsSecurityGroup.id],
      })
  )
);

// Access point for NATS data
const natsAccessPoint = new aws.efs.AccessPoint("nats-ap", {
  fileSystemId: natsFileSystem.id,
  posixUser: {
    gid: 1000,
    uid: 1000,
  },
  rootDirectory: {
    path: "/nats-data",
    creationInfo: {
      ownerGid: 1000,
      ownerUid: 1000,
      permissions: "755",
    },
  },
  tags: getStandardTags(),
});

// ========== ECS Cluster ==========

const cluster = new aws.ecs.Cluster("nats-cluster", {
  name: `mndigitalaid-nats-${config.stack}`,
  settings: [
    {
      name: "containerInsights",
      value: config.isProd ? "enabled" : "disabled",
    },
  ],
  tags: getStandardTags(),
});

// Task execution role
const taskExecutionRole = new aws.iam.Role("nats-task-execution-role", {
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

new aws.iam.RolePolicyAttachment("nats-task-execution-role-policy", {
  role: taskExecutionRole.name,
  policyArn:
    "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy",
});

// Task role
const taskRole = new aws.iam.Role("nats-task-role", {
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

// ========== Task Definition ==========

const taskDefinition = new aws.ecs.TaskDefinition("nats-task", {
  family: `mndigitalaid-nats-${config.stack}`,
  networkMode: "awsvpc",
  requiresCompatibilities: ["FARGATE"],
  cpu: config.isDev ? "256" : "512",
  memory: config.isDev ? "512" : "1024",
  executionRoleArn: taskExecutionRole.arn,
  taskRoleArn: taskRole.arn,
  volumes: [
    {
      name: "nats-data",
      efsVolumeConfiguration: {
        fileSystemId: natsFileSystem.id,
        transitEncryption: "ENABLED",
        authorizationConfig: {
          accessPointId: natsAccessPoint.id,
          iam: "ENABLED",
        },
      },
    },
  ],
  containerDefinitions: JSON.stringify([
    {
      name: "nats",
      image: "nats:2.10-alpine",
      essential: true,
      command: ["-js", "-sd", "/data"],
      portMappings: [
        { containerPort: 4222, protocol: "tcp" },
        { containerPort: 8222, protocol: "tcp" },
        { containerPort: 6222, protocol: "tcp" },
      ],
      mountPoints: [
        {
          sourceVolume: "nats-data",
          containerPath: "/data",
          readOnly: false,
        },
      ],
      logConfiguration: {
        logDriver: "awslogs",
        options: {
          "awslogs-group": natsLogGroup.name,
          "awslogs-region": config.region,
          "awslogs-stream-prefix": "nats",
        },
      },
      healthCheck: {
        command: [
          "CMD-SHELL",
          "wget -q -O - http://127.0.0.1:8222/healthz || exit 1",
        ],
        interval: 30,
        timeout: 5,
        retries: 3,
        startPeriod: 10,
      },
    },
  ]),
  tags: getStandardTags(),
});

// ========== Service Discovery ==========

const privateNamespace = new aws.servicediscovery.PrivateDnsNamespace(
  "nats-namespace",
  {
    name: `nats.${config.stack}.mndigitalaid.internal`,
    vpc: vpcId,
    description: "Private DNS namespace for NATS service discovery",
    tags: getStandardTags(),
  }
);

const natsServiceDiscovery = new aws.servicediscovery.Service("nats-sd", {
  name: "nats",
  namespaceId: privateNamespace.id,
  dnsConfig: {
    namespaceId: privateNamespace.id,
    dnsRecords: [
      {
        ttl: 10,
        type: "A",
      },
    ],
  },
  healthCheckCustomConfig: {
    failureThreshold: 1,
  },
  tags: getStandardTags(),
});

// ========== ECS Service ==========

const service = new aws.ecs.Service("nats-service", {
  cluster: cluster.arn,
  taskDefinition: taskDefinition.arn,
  desiredCount: 1, // Single node for now, can scale for HA
  launchType: "FARGATE",
  networkConfiguration: {
    subnets: subnetIds,
    securityGroups: [natsSecurityGroup.id],
    assignPublicIp: false, // Internal only
  },
  serviceRegistries: {
    registryArn: natsServiceDiscovery.arn,
  },
  tags: getStandardTags(),
});

// ========== Exports ==========

export const natsClusterName = cluster.name;
export const natsServiceName = service.name;
export const natsSecurityGroupId = natsSecurityGroup.id;
export const natsConnectionUrl = pulumi.interpolate`nats://nats.${privateNamespace.name}:4222`;
export const logGroupName = natsLogGroup.name;
