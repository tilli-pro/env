# Secrets Manager Alternatives

When deploying `with-env` or any application to production, you need a secure way to manage and inject secrets at runtime. Here are comprehensive alternatives to AWS Secrets Manager, organized by category.

## Table of Contents

1. [Cloud Provider Solutions](#cloud-provider-solutions)
2. [HashiCorp Vault](#hashicorp-vault)
3. [Kubernetes Native](#kubernetes-native)
4. [Lightweight/Self-Hosted](#lightweightself-hosted)
5. [CI/CD Integrated](#cicd-integrated)
6. [Comparison Matrix](#comparison-matrix)

---

## Cloud Provider Solutions

### 1. **AWS Secrets Manager** (Baseline)
- **Best for**: AWS-native applications
- **Pricing**: ~$0.40/secret/month + $0.05 per 10,000 API calls
- **Features**: Automatic rotation, cross-region replication, integration with RDS
- **Pros**: Native AWS integration, automatic rotation, audit logging
- **Cons**: More expensive than Parameter Store, AWS-locked

**Example with `with-env`:**
```bash
# Store GitHub token in Secrets Manager
aws secretsmanager create-secret \
  --name github-token \
  --secret-string "ghp_xxxxxxxxxxxxx"

# Reference in ECS task definition (see examples/ecs-task-definition.json)
```

### 2. **AWS Systems Manager Parameter Store**
- **Best for**: Cost-conscious AWS users
- **Pricing**: Standard parameters are FREE, Advanced ~$0.05/parameter/month
- **Features**: Hierarchical organization, version history, KMS encryption
- **Pros**: Free for standard parameters, simple, native AWS integration
- **Cons**: No automatic rotation, 10,000 parameter limit per region

**Example:**
```bash
# Store secret in Parameter Store
aws ssm put-parameter \
  --name "/with-env/github-token" \
  --value "ghp_xxxxxxxxxxxxx" \
  --type "SecureString" \
  --key-id "alias/aws/ssm"

# Retrieve at runtime
GITHUB_TOKEN=$(aws ssm get-parameter \
  --name "/with-env/github-token" \
  --with-decryption \
  --query "Parameter.Value" \
  --output text)
```

### 3. **Google Cloud Secret Manager**
- **Best for**: Google Cloud Platform applications
- **Pricing**: $0.06 per secret version/month + $0.03 per 10,000 access operations
- **Features**: Automatic replication, IAM integration, audit logging
- **Pros**: Global replication, versioning, GCP-native
- **Cons**: GCP-locked, pricing can add up

**Example:**
```bash
# Create secret
echo "ghp_xxxxxxxxxxxxx" | gcloud secrets create github-token --data-file=-

# Access in Cloud Run
gcloud run deploy with-env \
  --image gcr.io/project/with-env \
  --update-secrets GITHUB_TOKEN=github-token:latest
```

### 4. **Azure Key Vault**
- **Best for**: Azure-based applications
- **Pricing**: $0.03 per 10,000 transactions
- **Features**: Hardware Security Module (HSM) support, managed identities
- **Pros**: HSM support, certificates management, Azure-native
- **Cons**: Azure-locked, complex permissions model

**Example:**
```bash
# Create secret
az keyvault secret set \
  --vault-name "with-env-vault" \
  --name "github-token" \
  --value "ghp_xxxxxxxxxxxxx"

# Access in Azure Container Instances
az container create \
  --name with-env \
  --image myregistry.azurecr.io/with-env:latest \
  --secrets GITHUB_TOKEN=$(az keyvault secret show \
    --vault-name with-env-vault \
    --name github-token \
    --query value -o tsv)
```

---

## HashiCorp Vault

### 5. **HashiCorp Vault**
- **Best for**: Multi-cloud, security-first organizations
- **Pricing**: Open-source (free), Enterprise starts at ~$5/node/month
- **Features**: Dynamic secrets, automatic rotation, encryption as a service, PKI
- **Pros**: Multi-cloud, dynamic secrets, advanced features, secrets rotation
- **Cons**: Complex setup, operational overhead, requires infrastructure

**Deployment Options:**
- Self-hosted (open source)
- HashiCorp Cloud Platform (HCP) Vault (managed)
- Vault Enterprise (on-premises)

**Example Integration:**
```bash
# Store secret in Vault
vault kv put secret/with-env github-token="ghp_xxxxxxxxxxxxx"

# Access with Vault agent in Docker
# vault.hcl
auto_auth {
  method {
    type = "aws"
    config = {
      role = "with-env-role"
    }
  }
}

template {
  source      = "/templates/env.tpl"
  destination = "/home/appuser/.config/with-env/envs/production.env"
}
```

**Docker Compose with Vault:**
```yaml
services:
  vault-agent:
    image: vault:latest
    command: agent -config=/vault/config/agent.hcl
    volumes:
      - vault-config:/vault/config
      - shared-secrets:/secrets
    environment:
      - VAULT_ADDR=https://vault.example.com

  with-env:
    image: with-env:latest
    depends_on:
      - vault-agent
    volumes:
      - shared-secrets:/home/appuser/.config/with-env/envs:ro
```

---

## Kubernetes Native

### 6. **Kubernetes Secrets**
- **Best for**: Kubernetes deployments
- **Pricing**: Free (included with Kubernetes)
- **Features**: Native K8s integration, automatic mounting, RBAC
- **Pros**: Native to Kubernetes, simple, free
- **Cons**: Stored as base64 (not encrypted at rest by default), limited features

**Example:**
```bash
# Create Kubernetes secret
kubectl create secret generic with-env-secrets \
  --from-literal=github-token=ghp_xxxxxxxxxxxxx

# Use in deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: with-env
spec:
  template:
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
```

### 7. **Sealed Secrets** (Kubernetes)
- **Best for**: GitOps workflows with Kubernetes
- **Pricing**: Free (open source)
- **Features**: Encrypted secrets in Git, public key encryption
- **Pros**: GitOps-friendly, encrypted at rest in Git, open source
- **Cons**: Kubernetes-only, additional controller needed

**Example:**
```bash
# Install sealed-secrets controller
kubectl apply -f https://github.com/bitnami-labs/sealed-secrets/releases/download/v0.24.0/controller.yaml

# Create sealed secret
echo -n "ghp_xxxxxxxxxxxxx" | kubectl create secret generic with-env-secrets \
  --dry-run=client \
  --from-file=github-token=/dev/stdin \
  -o yaml | \
  kubeseal -o yaml > sealed-secret.yaml

# Commit to Git safely
git add sealed-secret.yaml
```

### 8. **External Secrets Operator**
- **Best for**: Multi-cloud Kubernetes with existing secrets managers
- **Pricing**: Free (open source)
- **Features**: Sync from external secret stores (AWS, GCP, Azure, Vault, etc.)
- **Pros**: Multi-cloud, existing secrets store integration, automatic sync
- **Cons**: Additional dependency, complexity

**Example:**
```yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: with-env-secrets
spec:
  secretStoreRef:
    name: aws-secrets-manager
    kind: SecretStore
  target:
    name: with-env-secrets
  data:
  - secretKey: github-token
    remoteRef:
      key: github-token
```

---

## Lightweight/Self-Hosted

### 9. **Infisical**
- **Best for**: Startups, small teams, modern developer experience
- **Pricing**: Free tier available, Pro starts at $18/month
- **Features**: Web UI, CLI, SDK, automatic syncing, versioning
- **Pros**: Modern UI, easy to use, self-hostable, good DX
- **Cons**: Relatively new, smaller community

**Example:**
```bash
# Install CLI
brew install infisical/get-cli/infisical

# Login and fetch secrets
infisical login
infisical run -- with-env run production npm start
```

### 10. **Doppler**
- **Best for**: Developer-first teams, all environments
- **Pricing**: Free for 5 users, Pro starts at $10/user/month
- **Features**: Web UI, CLI, automatic sync, integrations, audit logs
- **Pros**: Excellent DX, multi-environment, great integrations
- **Cons**: Hosted only (not self-hostable), pricing scales with users

**Example:**
```bash
# Install CLI
brew install dopplerhq/cli/doppler

# Run with-env with Doppler secrets
doppler run -- with-env run production npm start
```

### 11. **SOPS (Secrets OPerationS)**
- **Best for**: GitOps, encrypted files in Git
- **Pricing**: Free (open source)
- **Features**: File encryption with KMS/PGP/Age, Git-friendly
- **Pros**: Simple, Git-friendly, works with existing KMS
- **Cons**: No centralized server, manual key management

**Example:**
```bash
# Install
brew install sops

# Encrypt env file
sops --encrypt \
  --kms 'arn:aws:kms:us-east-1:123456789:key/abc-def' \
  production.env > production.enc.env

# Decrypt and use
sops --decrypt production.enc.env > /tmp/production.env
with-env run production npm start
```

### 12. **git-crypt**
- **Best for**: Small teams, simple encryption
- **Pricing**: Free (open source)
- **Features**: Transparent encryption/decryption in Git
- **Pros**: Simple, transparent, Git-integrated
- **Cons**: Not designed for secrets at scale, GPG key management

**Example:**
```bash
# Initialize
cd /path/to/repo
git-crypt init

# Mark files to encrypt
echo "*.env filter=git-crypt diff=git-crypt" >> .gitattributes

# Add collaborators
git-crypt add-gpg-user USER_ID
```

---

## CI/CD Integrated

### 13. **GitHub Secrets**
- **Best for**: GitHub Actions workflows
- **Pricing**: Free (included with GitHub)
- **Features**: Encrypted storage, environment-specific, OIDC support
- **Pros**: GitHub-native, free, automatic injection
- **Cons**: GitHub Actions only, manual rotation

**Example:**
```yaml
# Already covered in examples/github-actions-secrets-management.yml
jobs:
  deploy:
    steps:
      - name: Use secrets
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: with-env list-envs
```

### 14. **GitLab CI/CD Variables**
- **Best for**: GitLab CI/CD pipelines
- **Pricing**: Free (included with GitLab)
- **Features**: Protected/masked variables, environment-specific, file variables
- **Pros**: GitLab-native, free, scoped to environments
- **Cons**: GitLab only

**Example:**
```yaml
# .gitlab-ci.yml
deploy:
  stage: deploy
  script:
    - with-env --repo $CI_PROJECT_PATH list-envs
  variables:
    GITHUB_TOKEN: $GITHUB_TOKEN  # From GitLab CI/CD settings
```

### 15. **CircleCI Contexts**
- **Best for**: CircleCI workflows
- **Pricing**: Free (included with CircleCI)
- **Features**: Context-based secrets, RBAC, environment variables
- **Pros**: CircleCI-native, access control
- **Cons**: CircleCI only

---

## Comparison Matrix

| Solution | Pricing | Cloud | Self-Host | Rotation | Multi-Cloud | Complexity |
|----------|---------|-------|-----------|----------|-------------|------------|
| **AWS Secrets Manager** | $$ | AWS | ❌ | ✅ Auto | ❌ | Low |
| **AWS Parameter Store** | $ | AWS | ❌ | ❌ | ❌ | Very Low |
| **GCP Secret Manager** | $$ | GCP | ❌ | ❌ | ❌ | Low |
| **Azure Key Vault** | $$ | Azure | ❌ | ✅ | ❌ | Medium |
| **HashiCorp Vault** | Free-$$$ | Any | ✅ | ✅ Auto | ✅ | High |
| **K8s Secrets** | Free | Any | ✅ | ❌ | ✅ | Low |
| **Sealed Secrets** | Free | K8s | ✅ | ❌ | ✅ | Medium |
| **External Secrets** | Free | K8s | ✅ | ✅ | ✅ | Medium |
| **Infisical** | Free-$$ | Any | ✅ | ❌ | ✅ | Low |
| **Doppler** | Free-$$ | Any | ❌ | ❌ | ✅ | Very Low |
| **SOPS** | Free | Any | ✅ | ❌ | ✅ | Low |
| **git-crypt** | Free | Any | ✅ | ❌ | ✅ | Very Low |

**Legend:**
- **Pricing**: $ (cheap/free), $$ (moderate), $$$ (enterprise)
- **Rotation**: Auto rotation support
- **Complexity**: Implementation and operational complexity

---

## Recommendations by Use Case

### For `with-env` in AWS ECS/Fargate:
1. **Best**: AWS Secrets Manager (native integration, automatic rotation)
2. **Budget**: AWS Parameter Store (free for standard parameters)
3. **Advanced**: HashiCorp Vault (if multi-cloud or advanced features needed)

### For `with-env` in Kubernetes:
1. **Simple**: Kubernetes Secrets + External Secrets Operator
2. **GitOps**: Sealed Secrets or SOPS
3. **Enterprise**: HashiCorp Vault with Vault Agent Injector

### For `with-env` in CI/CD:
1. **GitHub Actions**: GitHub Secrets (native)
2. **Multi-platform**: Doppler or Infisical
3. **Self-hosted**: HashiCorp Vault

### For Development:
1. **Local**: `with-env` with local `.env` files
2. **Team sync**: Doppler or Infisical
3. **Simple**: SOPS-encrypted files in Git

---

## Implementation Example: ECS with Parameter Store

Here's a complete example using AWS Parameter Store instead of Secrets Manager:

```bash
# Store secrets in Parameter Store
aws ssm put-parameter \
  --name "/with-env/production/github-token" \
  --value "ghp_xxxxxxxxxxxxx" \
  --type "SecureString"

aws ssm put-parameter \
  --name "/with-env/production/api-key" \
  --value "api_key_value" \
  --type "SecureString"
```

**ECS Task Definition:**
```json
{
  "containerDefinitions": [{
    "secrets": [
      {
        "name": "GITHUB_TOKEN",
        "valueFrom": "arn:aws:ssm:us-east-1:123456789:parameter/with-env/production/github-token"
      },
      {
        "name": "API_KEY",
        "valueFrom": "arn:aws:ssm:us-east-1:123456789:parameter/with-env/production/api-key"
      }
    ]
  }]
}
```

**Required IAM Policy:**
```json
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Action": [
      "ssm:GetParameters",
      "ssm:GetParameter"
    ],
    "Resource": "arn:aws:ssm:us-east-1:123456789:parameter/with-env/*"
  }]
}
```

---

## Security Best Practices

Regardless of which solution you choose:

1. **Encryption at Rest**: Always encrypt secrets when stored
2. **Encryption in Transit**: Use TLS for all secret transfers
3. **Least Privilege**: Grant minimal IAM/RBAC permissions
4. **Audit Logging**: Enable and monitor access logs
5. **Rotation**: Implement regular secret rotation
6. **No Hardcoding**: Never commit secrets to Git
7. **Separate Environments**: Use different secrets per environment
8. **Access Control**: Limit who can read/write secrets

---

## Migration Path

If you're currently using one solution and want to migrate:

1. **Inventory**: List all current secrets
2. **Parallel Run**: Set up new solution alongside existing
3. **Test**: Verify new solution works in non-prod
4. **Migrate**: Move secrets to new solution
5. **Update**: Update applications to use new solution
6. **Verify**: Confirm everything works
7. **Cleanup**: Remove old secrets after grace period

---

## Additional Resources

- [AWS Secrets Manager vs Parameter Store](https://docs.aws.amazon.com/systems-manager/latest/userguide/integration-ps-secretsmanager.html)
- [HashiCorp Vault Documentation](https://www.vaultproject.io/docs)
- [External Secrets Operator](https://external-secrets.io/)
- [SOPS Documentation](https://github.com/mozilla/sops)
- [Infisical Documentation](https://infisical.com/docs)
- [Doppler Documentation](https://docs.doppler.com/)
