# Deployment Examples for with-env

This document provides concrete deployment examples for various platforms and secret management solutions.

## Table of Contents

1. [GitHub Actions Deployment](#github-actions-deployment)
2. [AWS ECS/Fargate](#aws-ecsfargate)
3. [Kubernetes](#kubernetes)
4. [Docker Compose](#docker-compose)
5. [Local Development](#local-development)

---

## GitHub Actions Deployment

### Basic CI/CD Pipeline

Located at `.github/workflows/ci.yml` - already exists in your repo.

### Secret Management Workflow

See `examples/github-actions-secrets-management.yml` for a comprehensive example.

**Quick Start:**

```bash
# 1. Build and push to GitHub
git push origin main

# 2. Trigger secret sync workflow
gh workflow run manage-github-secrets.yml \
  -f environment=production \
  -f action=sync-secrets
```

---

## AWS ECS/Fargate

### Option 1: Using AWS Secrets Manager

**Step 1: Store GitHub token**
```bash
aws secretsmanager create-secret \
  --name with-env/github-token \
  --description "GitHub token for with-env" \
  --secret-string "ghp_xxxxxxxxxxxxx" \
  --region us-east-1
```

**Step 2: Build and push Docker image**
```bash
# Login to ECR
aws ecr get-login-password --region us-east-1 | \
  docker login --username AWS --password-stdin YOUR_ACCOUNT.dkr.ecr.us-east-1.amazonaws.com

# Build and push
docker build -t with-env:latest .
docker tag with-env:latest YOUR_ACCOUNT.dkr.ecr.us-east-1.amazonaws.com/with-env:latest
docker push YOUR_ACCOUNT.dkr.ecr.us-east-1.amazonaws.com/with-env:latest
```

**Step 3: Deploy using CloudFormation**
```bash
aws cloudformation create-stack \
  --stack-name with-env-stack \
  --template-body file://examples/cloudformation/with-env-ecs.yaml \
  --parameters \
    ParameterKey=GitHubOrganization,ParameterValue=your-org \
    ParameterKey=GitHubTokenSecretArn,ParameterValue=arn:aws:secretsmanager:us-east-1:YOUR_ACCOUNT:secret:with-env/github-token-xxxxx \
    ParameterKey=VpcId,ParameterValue=vpc-xxxxx \
    ParameterKey=SubnetIds,ParameterValue="subnet-xxxxx\\,subnet-yyyyy" \
  --capabilities CAPABILITY_IAM
```

**Step 4: Run ECS task**
```bash
aws ecs run-task \
  --cluster with-env-cluster \
  --task-definition with-env-task \
  --launch-type FARGATE \
  --network-configuration "awsvpcConfiguration={subnets=[subnet-xxxxx],securityGroups=[sg-xxxxx],assignPublicIp=DISABLED}"
```

### Option 2: Using AWS Parameter Store (Cost-Effective)

**Step 1: Store secrets in Parameter Store**
```bash
# Store GitHub token
aws ssm put-parameter \
  --name "/with-env/github-token" \
  --value "ghp_xxxxxxxxxxxxx" \
  --type "SecureString" \
  --tier "Standard"

# Store additional secrets
aws ssm put-parameter \
  --name "/with-env/production/api-key" \
  --value "your-api-key" \
  --type "SecureString"
```

**Step 2: Create modified task definition**
```json
{
  "family": "with-env-task",
  "containerDefinitions": [{
    "name": "with-env",
    "image": "YOUR_ACCOUNT.dkr.ecr.us-east-1.amazonaws.com/with-env:latest",
    "secrets": [
      {
        "name": "GITHUB_TOKEN",
        "valueFrom": "arn:aws:ssm:us-east-1:YOUR_ACCOUNT:parameter/with-env/github-token"
      }
    ]
  }],
  "executionRoleArn": "arn:aws:iam::YOUR_ACCOUNT:role/ecsTaskExecutionRole",
  "taskRoleArn": "arn:aws:iam::YOUR_ACCOUNT:role/with-env-task-role"
}
```

**Step 3: Update IAM role for Parameter Store access**
```bash
cat > parameter-store-policy.json <<EOF
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Action": [
      "ssm:GetParameters",
      "ssm:GetParameter",
      "ssm:GetParametersByPath"
    ],
    "Resource": "arn:aws:ssm:us-east-1:YOUR_ACCOUNT:parameter/with-env/*"
  },
  {
    "Effect": "Allow",
    "Action": [
      "kms:Decrypt"
    ],
    "Resource": "arn:aws:kms:us-east-1:YOUR_ACCOUNT:key/your-kms-key-id"
  }]
}
EOF

aws iam put-role-policy \
  --role-name ecsTaskExecutionRole \
  --policy-name ParameterStoreAccess \
  --policy-document file://parameter-store-policy.json
```

### Option 3: Using Terraform

```bash
cd examples/terraform

# Initialize Terraform
terraform init

# Plan deployment
terraform plan \
  -var="github_org=your-org" \
  -var="github_token_secret_arn=arn:aws:secretsmanager:us-east-1:YOUR_ACCOUNT:secret:github-token-xxxxx"

# Apply
terraform apply
```

---

## Kubernetes

### Option 1: Basic Kubernetes Secrets

**Step 1: Create secret**
```bash
kubectl create secret generic with-env-secrets \
  --from-literal=github-token=ghp_xxxxxxxxxxxxx \
  --from-literal=organization=your-org \
  --namespace=default
```

**Step 2: Create deployment**
```yaml
# with-env-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: with-env
  namespace: default
spec:
  replicas: 1
  selector:
    matchLabels:
      app: with-env
  template:
    metadata:
      labels:
        app: with-env
    spec:
      containers:
      - name: with-env
        image: with-env:latest
        env:
        - name: GITHUB_TOKEN
          valueFrom:
            secretKeyRef:
              name: with-env-secrets
              key: github-token
        - name: ORGANIZATION
          valueFrom:
            secretKeyRef:
              name: with-env-secrets
              key: organization
        command: ["with-env", "list-envs"]
```

**Step 3: Deploy**
```bash
kubectl apply -f with-env-deployment.yaml
```

### Option 2: External Secrets Operator (AWS Secrets Manager)

**Step 1: Install External Secrets Operator**
```bash
helm repo add external-secrets https://charts.external-secrets.io
helm install external-secrets \
  external-secrets/external-secrets \
  -n external-secrets-system \
  --create-namespace
```

**Step 2: Create SecretStore**
```yaml
# secret-store.yaml
apiVersion: external-secrets.io/v1beta1
kind: SecretStore
metadata:
  name: aws-secrets-manager
  namespace: default
spec:
  provider:
    aws:
      service: SecretsManager
      region: us-east-1
      auth:
        jwt:
          serviceAccountRef:
            name: external-secrets-sa
```

**Step 3: Create ExternalSecret**
```yaml
# external-secret.yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: with-env-secrets
  namespace: default
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: aws-secrets-manager
    kind: SecretStore
  target:
    name: with-env-secrets
    creationPolicy: Owner
  data:
  - secretKey: github-token
    remoteRef:
      key: with-env/github-token
```

**Step 4: Deploy**
```bash
kubectl apply -f secret-store.yaml
kubectl apply -f external-secret.yaml
kubectl apply -f with-env-deployment.yaml
```

### Option 3: HashiCorp Vault

**Step 1: Install Vault**
```bash
helm repo add hashicorp https://helm.releases.hashicorp.com
helm install vault hashicorp/vault \
  --set "server.dev.enabled=true" \
  --namespace vault \
  --create-namespace
```

**Step 2: Store secrets in Vault**
```bash
# Port-forward to Vault
kubectl port-forward -n vault vault-0 8200:8200 &

# Store secret
export VAULT_ADDR='http://127.0.0.1:8200'
export VAULT_TOKEN='root'

vault kv put secret/with-env \
  github-token="ghp_xxxxxxxxxxxxx" \
  organization="your-org"
```

**Step 3: Configure Vault Agent Injector**
```yaml
# with-env-vault-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: with-env
spec:
  template:
    metadata:
      annotations:
        vault.hashicorp.com/agent-inject: "true"
        vault.hashicorp.com/role: "with-env"
        vault.hashicorp.com/agent-inject-secret-config: "secret/data/with-env"
        vault.hashicorp.com/agent-inject-template-config: |
          {{ with secret "secret/data/with-env" -}}
          export GITHUB_TOKEN="{{ .Data.data.github-token }}"
          export ORGANIZATION="{{ .Data.data.organization }}"
          {{- end }}
    spec:
      serviceAccountName: with-env
      containers:
      - name: with-env
        image: with-env:latest
        command: ["/bin/sh", "-c"]
        args:
        - source /vault/secrets/config && with-env list-envs
```

---

## Docker Compose

### Option 1: Environment Variables

```bash
# Create .env file (DO NOT commit to Git)
cat > .env <<EOF
GITHUB_TOKEN=ghp_xxxxxxxxxxxxx
ORGANIZATION=your-org
EOF

# Run with docker-compose
docker-compose up
```

### Option 2: Docker Secrets

```bash
# Create secrets
echo "ghp_xxxxxxxxxxxxx" | docker secret create github_token -
echo "your-org" | docker secret create organization -

# Use in docker-compose.yml
version: '3.8'
services:
  with-env:
    image: with-env:latest
    secrets:
      - github_token
      - organization
    environment:
      - GITHUB_TOKEN=/run/secrets/github_token
      - ORGANIZATION=/run/secrets/organization

secrets:
  github_token:
    external: true
  organization:
    external: true
```

### Option 3: With Vault (Docker Compose)

See `examples/docker-compose.yml` for a complete example with HashiCorp Vault integration.

---

## Local Development

### Option 1: Environment Files

```bash
# Create local env files
mkdir -p ~/.config/with-env/envs

# Create production env file
cat > ~/.config/with-env/envs/production.env <<EOF
DATABASE_URL=postgresql://localhost/mydb
API_KEY=abc123
SECRET_TOKEN=xyz789
EOF

# Secure the file
chmod 600 ~/.config/with-env/envs/production.env

# Initialize with-env
with-env init --org your-org --token ghp_xxxxxxxxxxxxx

# Run command with environment
with-env run production npm start
```

### Option 2: Using direnv

```bash
# Install direnv
brew install direnv

# Create .envrc in project directory
cat > .envrc <<EOF
export GITHUB_TOKEN=ghp_xxxxxxxxxxxxx
export ORGANIZATION=your-org

# Load secrets from with-env
eval "$(with-env run production env | sed 's/^/export /')"
EOF

# Add .envrc to .gitignore
echo ".envrc" >> .gitignore

# Allow direnv
direnv allow
```

### Option 3: Using Doppler

```bash
# Install Doppler CLI
brew install dopplerhq/cli/doppler

# Login
doppler login

# Setup project
doppler setup

# Run with-env with Doppler
doppler run -- with-env run production npm start
```

---

## CI/CD Integration Examples

### GitHub Actions

```yaml
# .github/workflows/deploy.yml
name: Deploy with with-env

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build with-env
        run: cargo build --release

      - name: Initialize
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          target/release/with-env init --org ${{ github.repository_owner }}

      - name: Deploy secrets to production
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          DATABASE_URL: ${{ secrets.PROD_DATABASE_URL }}
        run: |
          echo "$DATABASE_URL" | target/release/with-env set-secret production DATABASE_URL
```

### GitLab CI

```yaml
# .gitlab-ci.yml
deploy:
  stage: deploy
  image: rust:1.70
  script:
    - cargo build --release
    - ./target/release/with-env init --org $CI_PROJECT_NAMESPACE
    - ./target/release/with-env list-envs
  variables:
    GITHUB_TOKEN: $GITHUB_TOKEN  # Set in GitLab CI/CD settings
```

### CircleCI

```yaml
# .circleci/config.yml
version: 2.1

jobs:
  deploy:
    docker:
      - image: rust:1.70
    steps:
      - checkout
      - run:
          name: Build with-env
          command: cargo build --release
      - run:
          name: Deploy secrets
          command: |
            ./target/release/with-env init --org $CIRCLE_PROJECT_USERNAME
            ./target/release/with-env list-envs
```

---

## Production Checklist

Before deploying to production, ensure:

- [ ] Secrets are stored in a secure secrets manager (not environment variables)
- [ ] IAM/RBAC permissions follow least privilege principle
- [ ] File permissions are set correctly (600 for env files)
- [ ] Audit logging is enabled
- [ ] Secrets rotation policy is in place
- [ ] Backup and recovery procedures are documented
- [ ] Monitoring and alerting are configured
- [ ] Security scanning is enabled (cargo audit, container scanning)
- [ ] All secrets are encrypted at rest and in transit
- [ ] `.env` files are in `.gitignore`

---

## Troubleshooting

### Issue: "Permission denied" when accessing secrets

**Solution:**
```bash
# For AWS
aws iam get-role-policy --role-name ecsTaskExecutionRole --policy-name SecretsAccess

# For Kubernetes
kubectl auth can-i get secrets --as=system:serviceaccount:default:with-env
```

### Issue: "Secret not found"

**Solution:**
```bash
# Verify secret exists in AWS Secrets Manager
aws secretsmanager describe-secret --secret-id with-env/github-token

# Verify in Kubernetes
kubectl get secret with-env-secrets -o yaml
```

### Issue: Docker build fails

**Solution:**
```bash
# Clear cache and rebuild
docker build --no-cache -t with-env:latest .

# Check for build dependencies
docker run --rm -it rust:1.70 cargo --version
```

---

## Next Steps

1. Choose a secrets management solution from [SECRETS_MANAGER_ALTERNATIVES.md](../docs/SECRETS_MANAGER_ALTERNATIVES.md)
2. Set up your deployment infrastructure (ECS, Kubernetes, etc.)
3. Configure secrets and environment variables
4. Test in a non-production environment
5. Deploy to production
6. Monitor and maintain

For more information, see:
- [Main README](../README.md)
- [Security Best Practices](../SECURITY.md)
- [Secrets Manager Alternatives](../docs/SECRETS_MANAGER_ALTERNATIVES.md)
