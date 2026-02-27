# Root Editorial Infrastructure

Pulumi-based infrastructure as code for deploying to AWS.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        CloudFront                           │
│  ┌──────────────────┐           ┌──────────────────┐       │
│  │   admin-app      │           │    web-app       │       │
│  │ admin.mntogether │           │  app.mntogether  │       │
│  │      .org        │           │      .org        │       │
│  └────────┬─────────┘           └────────┬─────────┘       │
└───────────┼──────────────────────────────┼─────────────────┘
            │                              │
            │  S3 Static Hosting           │  S3 Static Hosting
            │                              │
┌───────────┴──────────────────────────────────────────────────┐
│                  Application Load Balancer                    │
│  ┌─────────────────────────┐                                 │
│  │   api.mntogether.org    │                                 │
│  │   (Rust API Server)     │                                 │
│  └────────┬────────────────┘                                 │
└───────────┼──────────────────────────────────────────────────┘
            │
┌───────────┴──────────────────────────────────────────────────┐
│                       ECS Fargate                             │
│  ┌──────────────────────────────┐                            │
│  │  Server (Rust)               │                            │
│  │  - Restate endpoint (h2c)    │                            │
│  │  - Auto-scaling              │                            │
│  └──────────────┬───────────────┘                            │
└─────────────────┼────────────────────────────────────────────┘
                  │
┌─────────────────┴────────────────────────────────────────────┐
│                    RDS PostgreSQL                              │
│  - pgvector extension                                         │
│  - Automated backups                                          │
│  - Encrypted at rest                                          │
│  - Multi-AZ (prod)                                            │
└───────────────────────────────────────────────────────────────┘
```

## Stacks

### core (`infra/packages/core/`)
Shared infrastructure:
- **ACM Certificate**: Wildcard SSL certificate (*.mntogether.org)
- **RDS PostgreSQL**: Database with pgvector extension
- **Secrets Manager**: Database credentials
- **VPC & Networking**: Default VPC configuration
- **Route53**: DNS hosted zone

### server (`infra/packages/server/`)
API backend deployment:
- **ECS Fargate**: Containerized Rust server
- **Application Load Balancer**: HTTPS termination and routing
- **CloudWatch Logs**: Application logging
- **Auto Scaling**: Based on CPU/memory metrics
- **ECR**: Container image repository (`rooteditorial-server`)

### web-app (`infra/packages/web-app/`)
Static frontend hosting (both admin and public apps):
- **S3**: Static file hosting
- **CloudFront**: CDN distribution with HTTPS
- **Route53**: DNS configuration (admin.mntogether.org, app.mntogether.org)

## Prerequisites

1. **AWS Account** with appropriate permissions
2. **Pulumi Account** (free tier works)
3. **Domain** registered in Route53 (mntogether.org)
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
# Region: us-east-1
```

### 3. Configure Pulumi

```bash
pulumi login
export PULUMI_ACCESS_TOKEN=<your-token>
```

### 4. Create ECR Repository

```bash
aws ecr create-repository \
  --repository-name rooteditorial-server \
  --region us-east-1
```

### 5. Configure Stacks

```bash
cd packages/core
pulumi stack select dev --create
pulumi config set domain mntogether.org
pulumi config set aws:region us-east-1
```

Repeat for prod:

```bash
pulumi stack select prod --create
pulumi config set domain mntogether.org
pulumi config set aws:region us-east-1
```

## Deployment

### Manual Deployment

Deploy all stacks:

```bash
./deploy.sh -e dev -s all
```

Deploy specific stack:

```bash
./deploy.sh -e dev -s core
./deploy.sh -e dev -s server
./deploy.sh -e dev -s web-app
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
- Triggered by changes in `packages/server/**`

**Web App (Static)**:
- After successful server deployment
- Push to `main`/`dev`
- Triggered by changes in `packages/admin-app/**` or `packages/web-app/**`

### Required GitHub Secrets

```
AWS_ROLE_ARN               # AWS IAM role ARN for GitHub OIDC
PULUMI_ACCESS_TOKEN        # Pulumi access token
PULUMI_CONFIG_PASSPHRASE   # Passphrase for Pulumi secrets
ECR_REPOSITORY             # ECR repository name (rooteditorial-server)
```

## Stack Outputs

### core
- `certificateArn`: ACM certificate ARN
- `databaseSecretArn`: Secrets Manager ARN for DB credentials
- `databaseEndpoint`: RDS endpoint
- `vpcId`: VPC ID
- `subnetIds`: List of subnet IDs

### server
- `targetUrl`: API URL (https://api.mntogether.org)
- `albDnsName`: Load balancer DNS name
- `clusterName`: ECS cluster name
- `logGroupName`: CloudWatch log group

### web-app
- `targetUrl`: Web app URL (https://app.mntogether.org)
- `bucketName`: S3 bucket name
- `cloudFrontDistributionId`: CloudFront distribution ID

## Stack References

Stacks reference each other via `StackReference`:

```typescript
const coreStack = new pulumi.StackReference(`mntogether/rooteditorial-core/${stack}`);
const certificateArn = coreStack.getOutput("certificateArn");
```

## Monitoring

### CloudWatch Logs

```bash
aws logs tail /rooteditorial/dev/api --follow
```

### CloudWatch Metrics

Monitor in AWS Console:
- ECS Service CPU/Memory
- ALB Request Count
- RDS Connections
- CloudFront Cache Hit Rate

## Maintenance

### Database Migrations

```bash
# Connect to ECS task
aws ecs execute-command \
  --cluster rooteditorial-dev \
  --task <task-id> \
  --container api \
  --interactive \
  --command "/bin/bash"

# Inside container
sqlx migrate run
```

### Update Server Image

```bash
cd packages/server
docker build -t <ecr-repo>:new-tag .
docker push <ecr-repo>:new-tag

cd ../../infra/packages/server
pulumi config set apiImageTag new-tag
pulumi up --yes
```

### Invalidate CloudFront Cache

```bash
cd infra/packages/web-app
DIST_ID=$(pulumi stack output cloudFrontDistributionId)

aws cloudfront create-invalidation \
  --distribution-id $DIST_ID \
  --paths "/*"
```

## Cost Estimates

### Development (~$30-50/month)

- RDS `db.t3.micro`: $15-20 (free tier eligible)
- ECS 1 task (256 CPU / 512 MB): $10-15
- Data transfer: $5-10
- CloudWatch logs: 7-day retention

### Production (~$100-150/month)

- RDS `db.t3.small` Multi-AZ: $40-60
- ECS 2+ tasks (512 CPU / 1024 MB): $40-60
- CloudFront: $10-20
- Data transfer: $10-20
- CloudWatch logs: 30-day retention

## Security

- All traffic encrypted (HTTPS/TLS)
- Database not publicly accessible
- Secrets stored in AWS Secrets Manager
- IAM roles with least privilege
- Security groups restrict access
- CloudFront DDoS protection
- GitHub Actions OIDC — no long-lived AWS credentials

## Backup & Disaster Recovery

### RDS Backups

- Automated daily backups (7-day retention)
- Point-in-time recovery enabled
- Manual snapshots before major changes

### Restore from Backup

```bash
aws rds restore-db-instance-from-db-snapshot \
  --db-instance-identifier rooteditorial-restored \
  --db-snapshot-identifier rooteditorial-snapshot-1
```

### Infrastructure as Code

All infrastructure is version controlled and can be recreated:

```bash
./deploy.sh -e prod -s all
```
