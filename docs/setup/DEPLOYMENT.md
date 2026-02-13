# mndigitalaid Deployment Guide

Complete guide for deploying mndigitalaid to AWS using Pulumi and GitHub Actions.

## Overview

The deployment infrastructure consists of:

- **Infrastructure as Code**: Pulumi (TypeScript) for AWS resources
- **CI/CD**: GitHub Actions for automated deployments
- **Environments**: Development (`dev`) and Production (`prod`)
- **Hosting**:
  - Admin SPA → CloudFront + S3 (admin.mndigitalaid.org)
  - Web App → CloudFront + S3 (app.mndigitalaid.org)
  - Server API → ECS Fargate + ALB (api.mndigitalaid.org)
  - Database → RDS PostgreSQL with pgvector

## Initial Setup

### 1. AWS Setup

#### Create IAM Role for GitHub Actions (OIDC)

```bash
# Create trust policy for GitHub
cat > github-trust-policy.json <<EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "Federated": "arn:aws:iam::<AWS_ACCOUNT_ID>:oidc-provider/token.actions.githubusercontent.com"
      },
      "Action": "sts:AssumeRoleWithWebIdentity",
      "Condition": {
        "StringEquals": {
          "token.actions.githubusercontent.com:aud": "sts.amazonaws.com"
        },
        "StringLike": {
          "token.actions.githubusercontent.com:sub": "repo:fourthplaces/mndigitalaid:*"
        }
      }
    }
  ]
}
EOF

# Create IAM role
aws iam create-role \
  --role-name github-actions-mndigitalaid \
  --assume-role-policy-document file://github-trust-policy.json

# Attach policies (adjust as needed)
aws iam attach-role-policy \
  --role-name github-actions-mndigitalaid \
  --policy-arn arn:aws:iam::aws:policy/AdministratorAccess
```

#### Create ECR Repository

```bash
aws ecr create-repository \
  --repository-name mndigitalaid-server \
  --region us-east-1
```

### 2. Route53 Domain

Ensure you have a hosted zone for your domain:

```bash
aws route53 list-hosted-zones
# Should show mndigitalaid.org
```

If not, create one:

```bash
aws route53 create-hosted-zone \
  --name mndigitalaid.org \
  --caller-reference $(date +%s)
```

### 3. Pulumi Setup

#### Sign up for Pulumi

1. Go to https://app.pulumi.com
2. Sign up (free tier is sufficient)
3. Create a new organization (e.g., "mndigitalaid")

#### Get Pulumi Access Token

1. Go to Settings → Access Tokens
2. Create new token
3. Copy token for later use

#### Set up Pulumi passphrase

Generate a secure passphrase:

```bash
openssl rand -base64 32
# Save this passphrase securely
```

### 4. GitHub Secrets

Add these secrets to your GitHub repository (Settings → Secrets and variables → Actions):

```
AWS_ROLE_ARN
  Value: arn:aws:iam::<AWS_ACCOUNT_ID>:role/github-actions-mndigitalaid

PULUMI_ACCESS_TOKEN
  Value: <your-pulumi-access-token>

PULUMI_CONFIG_PASSPHRASE
  Value: <your-generated-passphrase>

ECR_REPOSITORY
  Value: mndigitalaid-server
```

### 5. Local Development Setup

```bash
# Install Pulumi CLI
curl -fsSL https://get.pulumi.com | sh

# Login to Pulumi
pulumi login

# Set access token
export PULUMI_ACCESS_TOKEN=<your-token>

# Install dependencies
cd infra
yarn install
yarn build
```

## Deployment Workflows

### Manual Deployment

#### Deploy to Development

```bash
# Deploy all stacks
./deploy.sh dev all up --yes

# Or deploy individually
./deploy.sh dev core up --yes      # Core infrastructure first
./deploy.sh dev server up --yes    # Then server
./deploy.sh dev admin-spa up --yes # Frontend apps
./deploy.sh dev web-app up --yes
```

#### Deploy to Production

```bash
./deploy.sh prod all up --yes
```

#### Preview Changes

```bash
./deploy.sh dev server preview
```

### Automated Deployment (GitHub Actions)

#### Deployment Triggers

**Server Deployment**:
- Automatic on push to `main` (→ prod) or `dev` (→ dev)
- Automatic when files change in:
  - `packages/server/**`
  - `infra/packages/server/**`
  - `infra/packages/core/**`
- Manual via workflow dispatch

**Frontend Deployment**:
- Automatic after successful server deployment
- Automatic on push to `main` or `dev`
- Automatic when files change in:
  - `packages/admin-spa/**`
  - `packages/web-app/**`
  - `infra/packages/*-spa/**`
- Manual via workflow dispatch

#### Manual Workflow Trigger

1. Go to GitHub → Actions
2. Select "Deploy Server" or "Deploy Frontend"
3. Click "Run workflow"
4. Choose environment (dev/prod) and ref (branch/tag)
5. Run workflow

### Deployment Order

**First Time Deployment**:

```bash
1. Core stack (RDS, certificates, VPC)
2. Server stack (ECS, ALB)
3. Admin SPA stack (CloudFront, S3)
4. Web App stack (CloudFront, S3)
```

**Subsequent Deployments**:

- Core changes: Rarely needed
- Server: Any backend code changes
- Frontend: Any admin-spa or web-app changes

## Post-Deployment Tasks

### 1. Database Migrations

After deploying the server for the first time:

```bash
# Get ECS task ID
aws ecs list-tasks \
  --cluster mndigitalaid-dev \
  --service-name api-service

# Connect to task
aws ecs execute-command \
  --cluster mndigitalaid-dev \
  --task <task-id> \
  --container api \
  --interactive \
  --command "/bin/bash"

# Run migrations
cd /app
sqlx migrate run
```

### 2. Verify Deployment

```bash
# Check API health
curl https://api.mndigitalaid.org/health

# Check admin SPA
curl -I https://admin.mndigitalaid.org

# Check web app
curl -I https://app.mndigitalaid.org
```

### 3. Generate Embeddings

After database is set up:

```bash
# Connect to ECS task
aws ecs execute-command \
  --cluster mndigitalaid-prod \
  --task <task-id> \
  --container api \
  --interactive \
  --command "/bin/bash"

# Inside container
/app/generate_embeddings
```

## Monitoring

### CloudWatch Logs

```bash
# Server logs
aws logs tail /mndigitalaid/prod/api --follow

# Filter for errors
aws logs tail /mndigitalaid/prod/api --follow --filter-pattern "ERROR"
```

### ECS Service Status

```bash
aws ecs describe-services \
  --cluster mndigitalaid-prod \
  --services api-service
```

### RDS Status

```bash
aws rds describe-db-instances \
  --db-instance-identifier mndigitalaid-db-prod
```

## Troubleshooting

### Deployment Fails

1. **Check GitHub Actions logs**:
   - Go to Actions tab in GitHub
   - Click on failed workflow
   - Review logs

2. **Check Pulumi logs**:
```bash
cd infra/packages/<stack>
pulumi logs --follow
```

3. **Common issues**:
   - Stack reference not found → Deploy core stack first
   - ECR image not found → Build and push Docker image
   - Certificate validation timeout → Check DNS records

### Server Not Starting

1. **Check ECS logs**:
```bash
aws logs tail /mndigitalaid/prod/api --follow
```

2. **Check service events**:
```bash
aws ecs describe-services \
  --cluster mndigitalaid-prod \
  --services api-service \
  --query 'services[0].events[:10]'
```

3. **Common issues**:
   - Database connection → Check security groups
   - Missing environment variables → Check task definition
   - Health check failing → Verify /health endpoint

### Frontend Not Loading

1. **Check S3 bucket**:
```bash
aws s3 ls s3://admin.mndigitalaid.org/
```

2. **Check CloudFront distribution**:
```bash
# Get distribution ID from Pulumi
cd infra/packages/admin-spa
pulumi stack output cloudFrontDistributionId

# Check status
aws cloudfront get-distribution --id <dist-id>
```

3. **Invalidate cache** (if files exist but not showing):
```bash
aws cloudfront create-invalidation \
  --distribution-id <dist-id> \
  --paths "/*"
```

## Rollback

### Rollback Server Deployment

```bash
# Revert to previous image tag
cd infra/packages/server
pulumi config set apiImageTag <previous-tag>
pulumi up --yes

# Or use GitHub workflow to deploy specific ref
```

### Rollback Infrastructure Changes

```bash
# Preview stack at previous state
cd infra/packages/<stack>
git checkout <previous-commit>
pulumi preview

# Apply if looks correct
pulumi up --yes
```

### Restore Database from Backup

```bash
# List snapshots
aws rds describe-db-snapshots \
  --db-instance-identifier mndigitalaid-db-prod

# Restore from snapshot
aws rds restore-db-instance-from-db-snapshot \
  --db-instance-identifier mndigitalaid-db-restored \
  --db-snapshot-identifier <snapshot-id>
```

## Cost Management

### View Current Costs

Use AWS Cost Explorer:
1. AWS Console → Cost Explorer
2. Filter by tag: Project=mndigitalaid
3. View by service

### Cost Optimization Tips

- **Dev environment**: Use smaller instances, single-AZ RDS
- **Prod environment**: Right-size after monitoring usage
- **CloudFront**: Aggressive caching reduces origin requests
- **ECS**: Auto-scaling based on load
- **RDS**: Consider Aurora Serverless for variable workloads

## Security

### Secrets Management

- Database credentials → AWS Secrets Manager
- API keys → GitHub Secrets (for CI/CD)
- Application secrets → Pulumi Config (encrypted)

### Network Security

- RDS not publicly accessible
- ECS tasks in private subnets (with public IPs for internet access)
- ALB security groups restrict to HTTPS only
- CloudFront provides DDoS protection

### Regular Updates

```bash
# Update dependencies
cd infra
yarn upgrade-interactive

# Update Pulumi
pulumi upgrade

# Update Docker base images
docker pull rust:1.84-slim
docker pull debian:bookworm-slim
```

## Support

For deployment issues:
1. Check this guide
2. Review GitHub Actions logs
3. Check CloudWatch logs
4. Review Pulumi state: `pulumi stack --show-urns`
5. Contact infrastructure team

## Additional Resources

- [Pulumi Documentation](https://www.pulumi.com/docs/)
- [AWS ECS Best Practices](https://docs.aws.amazon.com/AmazonECS/latest/bestpracticesguide/)
- [CloudFront Documentation](https://docs.aws.amazon.com/cloudfront/)
- [Infrastructure README](./infra/README.md)
