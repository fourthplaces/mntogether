# mndigitalaid Infrastructure

Pulumi-based infrastructure as code for deploying mndigitalaid to AWS.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        CloudFront                           │
│  ┌──────────────────┐           ┌──────────────────┐       │
│  │   admin-spa      │           │    web-app       │       │
│  │ admin.domain.com │           │  app.domain.com  │       │
│  └────────┬─────────┘           └────────┬─────────┘       │
└───────────┼──────────────────────────────┼─────────────────┘
            │                              │
            │  S3 Static Hosting           │  S3 Static Hosting
            │                              │
            │
┌───────────┴──────────────────────────────────────────────────┐
│                  Application Load Balancers                   │
│  ┌─────────────────────────┐     ┌──────────────────────┐   │
│  │   api.domain.com        │     │  www.domain.com      │   │
│  │   (GraphQL API)         │     │  (Next.js SSR)       │   │
│  └────────┬────────────────┘     └──────────┬───────────┘   │
└───────────┼────────────────────────────────┼───────────────┘
            │                                │
            │                                │
┌───────────┴────────────────────────────────┴──────────────────┐
│                       ECS Fargate                              │
│  ┌──────────────────────────────┐  ┌──────────────────────┐  │
│  │  Server (Rust)               │  │  Next.js (Node.js)   │  │
│  │  - GraphQL API               │  │  - SSR for SEO       │  │
│  │  - WebSocket support         │  │  - GraphQL client    │  │
│  │  - Auto-scaling              │  │  - Auto-scaling      │  │
│  └──────────────┬───────────────┘  └──────────────────────┘  │
└─────────────────┼─────────────────────────────────────────────┘
                  │
                  │
┌─────────────────┴─────────────────────────────────────────────┐
│                    RDS PostgreSQL                              │
│  - pgvector extension                                          │
│  - Automated backups                                           │
│  - Encrypted at rest                                           │
│  - Multi-AZ (prod)                                             │
└────────────────────────────────────────────────────────────────┘
```

## Stacks

### core
Shared infrastructure:
- **ACM Certificate**: Wildcard SSL certificate (*.domain.com)
- **RDS PostgreSQL**: Database with pgvector extension
- **Secrets Manager**: Database credentials
- **VPC & Networking**: Default VPC configuration

### server
API backend deployment:
- **ECS Fargate**: Containerized Rust server
- **Application Load Balancer**: HTTPS termination and routing
- **CloudWatch Logs**: Application logging
- **Auto Scaling**: Based on CPU/memory metrics

### web-app
Unified web application (public + admin):
- **S3**: Static file hosting
- **CloudFront**: CDN distribution
- **Route53**: DNS configuration (app.domain.com)
- **Features**: Public pages + admin dashboard at /admin

### web-next
Next.js application with SSR:
- **ECS Fargate**: Containerized Next.js server
- **Application Load Balancer**: HTTPS termination and routing
- **CloudWatch Logs**: Application logging
- **Auto Scaling**: Based on CPU/memory metrics
- **Route53**: DNS configuration (www.domain.com and domain.com)

## Prerequisites

1. **AWS Account** with appropriate permissions
2. **Pulumi Account** (free tier works)
3. **Domain** registered in Route53
4. **Tools**:
   - Node.js 22+
   - Yarn 4.1+
   - Pulumi CLI
   - AWS CLI

## Setup

### 1. Install Dependencies

```bash
cd infra
yarn install
yarn build
```

### 2. Configure AWS

```bash
aws configure
# Set your AWS credentials and region
```

### 3. Configure Pulumi

```bash
# Login to Pulumi
pulumi login

# Set up Pulumi access token
export PULUMI_ACCESS_TOKEN=<your-token>
```

### 4. Create ECR Repositories

Create ECR repositories for Docker images:

```bash
# Create repository for server
aws ecr create-repository \
  --repository-name mndigitalaid-server \
  --region us-east-1

# Create repository for Next.js app
aws ecr create-repository \
  --repository-name mndigitalaid-web-next \
  --region us-east-1
```

### 5. Configure Stacks

For each environment (dev/prod), configure the domain:

```bash
cd packages/core
pulumi stack select dev --create
pulumi config set domain mndigitalaid.org
pulumi config set aws:region us-east-1
```

Repeat for prod:

```bash
pulumi stack select prod --create
pulumi config set domain mndigitalaid.org
pulumi config set aws:region us-east-1
```

## Deployment

### Manual Deployment

Deploy all stacks:

```bash
./deploy.sh dev all up --yes
```

Deploy specific stack:

```bash
./deploy.sh -e dev -s core
./deploy.sh -e dev -s server
./deploy.sh -e dev -s web-app
./deploy.sh -e dev -s web-next
```

Preview changes:

```bash
ENV=dev PULUMI_ARGS="preview" ./deploy.sh -s server
```

### Automated Deployment (GitHub Actions)

Deployment is triggered automatically:

**Server**:
- Push to `main` → deploys to prod
- Push to `dev` → deploys to dev
- Changes in `packages/server/**`

**Web App (SPA)**:
- After successful server deployment
- Push to `main`/`dev`
- Changes in `packages/web-app/**`

**Next.js App**:
- Push to `main` → deploys to prod
- Push to `dev` → deploys to dev
- Changes in `packages/web-next/**`

### Required GitHub Secrets

```
AWS_ROLE_ARN               # AWS IAM role ARN for GitHub OIDC
PULUMI_ACCESS_TOKEN        # Pulumi access token
PULUMI_CONFIG_PASSPHRASE   # Passphrase for Pulumi secrets
ECR_REPOSITORY             # ECR repository name for Docker images
```

## Configuration

### Stack Configuration Files

Each stack can have environment-specific configuration:

```yaml
# packages/server/Pulumi.dev.yaml
config:
  aws:region: us-east-1
  mndigitalaid-server:apiImageTag: dev-abc123
  mndigitalaid-server:ecrRepoName: mndigitalaid-server
```

### Stack References

Stacks reference each other via `StackReference`:

```typescript
const coreStack = new pulumi.StackReference(`mndigitalaid-core-${config.stack}`);
const certificateArn = coreStack.getOutput("certificateArn");
```

## Outputs

### core
- `certificateArn`: ACM certificate ARN
- `databaseSecretArn`: Secrets Manager ARN for DB credentials
- `databaseEndpoint`: RDS endpoint
- `vpcId`: VPC ID
- `subnetIds`: List of subnet IDs

### server
- `targetUrl`: API URL (https://api.domain.com)
- `albDnsName`: Load balancer DNS name
- `clusterName`: ECS cluster name
- `logGroupName`: CloudWatch log group

### web-app
- `targetUrl`: Web app URL (https://app.domain.com)
- `bucketName`: S3 bucket name
- `cloudFrontDistributionId`: CloudFront distribution ID

### web-next
- `targetUrl`: Next.js app URL (https://www.domain.com)
- `rootUrl`: Root domain URL (https://domain.com)
- `albDnsName`: Load balancer DNS name
- `clusterName`: ECS cluster name
- `logGroupName`: CloudWatch log group

## Monitoring

### CloudWatch Logs

View server logs:

```bash
# API server
aws logs tail /mndigitalaid/dev/api --follow

# Next.js app
aws logs tail /mndigitalaid/dev/web-next --follow
```

### CloudWatch Metrics

Monitor in AWS Console:
- ECS Service CPU/Memory
- ALB Request Count
- RDS Connections
- CloudFront Cache Hit Rate

## Maintenance

### Database Migrations

Run migrations after deploying the server:

```bash
# Connect to ECS task
aws ecs execute-command \
  --cluster mndigitalaid-dev \
  --task <task-id> \
  --container api \
  --interactive \
  --command "/bin/bash"

# Inside container
sqlx migrate run
```

### Update Docker Images

Deploy new server version:

```bash
# Build and push new server image
cd packages/server
docker build -t <ecr-repo>:new-tag .
docker push <ecr-repo>:new-tag

# Update Pulumi config
cd ../../infra/packages/server
pulumi config set apiImageTag new-tag
pulumi up --yes
```

Deploy new Next.js version:

```bash
# Build and push new Next.js image
cd packages/web-next
docker build -t <ecr-repo>:new-tag .
docker push <ecr-repo>:new-tag

# Update Pulumi config
cd ../../infra/packages/web-next
pulumi config set imageTag new-tag
pulumi up --yes
```

### Invalidate CloudFront Cache

After deploying frontend changes:

```bash
# Get distribution ID
cd infra/packages/admin-spa
DIST_ID=$(pulumi stack output cloudFrontDistributionId)

# Invalidate all files
aws cloudfront create-invalidation \
  --distribution-id $DIST_ID \
  --paths "/*"
```

## Troubleshooting

### Stack Fails to Deploy

Check Pulumi logs:

```bash
cd infra/packages/<stack>
pulumi logs --follow
```

### Server Not Starting

1. Check ECS task logs:
```bash
aws logs tail /mndigitalaid/dev/api --follow
```

2. Verify environment variables are set correctly

3. Check database connectivity

### Frontend Not Loading

1. Check S3 bucket has files:
```bash
aws s3 ls s3://admin.mndigitalaid.org/
```

2. Check CloudFront distribution status:
```bash
aws cloudfront get-distribution --id <dist-id>
```

3. Verify DNS records in Route53

## Cost Optimization

### Development Environment

- RDS: `db.t3.micro` (free tier eligible)
- ECS: 1 task, 256 CPU / 512 MB memory
- CloudWatch logs: 7-day retention
- CloudFront: Minimal caching

### Production Environment

- RDS: `db.t3.small`, Multi-AZ enabled
- ECS: 2+ tasks, 512 CPU / 1024 MB memory
- CloudWatch logs: 30-day retention
- CloudFront: Aggressive caching

### Monthly Cost Estimate

**Dev**: ~$30-50/month
- RDS: $15-20
- ECS: $10-15
- Data transfer: $5-10

**Prod**: ~$100-150/month
- RDS: $40-60
- ECS: $40-60
- CloudFront: $10-20
- Data transfer: $10-20

## Security

- All traffic encrypted (HTTPS/TLS)
- Database not publicly accessible
- Secrets stored in AWS Secrets Manager
- IAM roles with least privilege
- Security groups restrict access
- CloudFront protects against DDoS

## Backup & Disaster Recovery

### RDS Backups

- Automated daily backups (7-day retention)
- Manual snapshots before major changes
- Point-in-time recovery enabled

### Restore from Backup

```bash
aws rds restore-db-instance-from-db-snapshot \
  --db-instance-identifier mndigitalaid-restored \
  --db-snapshot-identifier mndigitalaid-snapshot-1
```

### Infrastructure as Code

All infrastructure is version controlled and can be recreated:

```bash
./deploy.sh prod all up --yes
```

## Support

For issues or questions:
- Check Pulumi logs: `pulumi logs`
- Check AWS CloudWatch logs
- Review GitHub Actions workflow runs
- Contact infrastructure team
