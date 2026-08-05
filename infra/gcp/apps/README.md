# App-Secrets Project Infrastructure

Terraform for the dedicated Google Cloud project that holds first-party app
provider secrets (`prod-app-*` and `preview-app-*` containers) and the
workload identities this repository's automation uses. The contract is
defined in [`docs/protocol/app-deployment.md`](../../../docs/protocol/app-deployment.md);
the project id lives in the repository root [`deploy.toml`](../../../deploy.toml).

The project intentionally contains nothing but app secrets: secret creation
cannot be prefix-scoped by IAM conditions, so granting the merge-gate
identity `secretmanager.secrets.create` is safe only because there is
nothing else here to create or shadow.

## Identities

| Identity | Assumable from | Access |
| --- | --- | --- |
| `apps-ci` | any ref of `futex-ai/firna-apps` | create containers, read metadata, never values |
| `apps-deploy` | `refs/heads/main` only | read secret values |
| platform preview deploy | `futex-ai/firna` preview workflows | read `preview-app-*` values only |

## Bootstrap

One-time operator setup before the first apply:

```sh
gcloud billing projects link firna-apps --billing-account=<ACCOUNT_ID>
gcloud storage buckets create gs://firna-apps-terraform-state \
  --project=firna-apps --location=us-central1 \
  --uniform-bucket-level-access
```

## Apply

```sh
cd infra/gcp/apps
terraform init
terraform fmt -check
terraform validate
terraform apply
```

After apply, set the GitHub repository variables from the outputs:

- `APPS_GCP_WORKLOAD_IDENTITY_PROVIDER` = `workload_identity_provider`
- `APPS_CI_SERVICE_ACCOUNT` = `apps_ci_service_account_email`
- `APPS_DEPLOY_SERVICE_ACCOUNT` = `apps_deploy_service_account_email`

Secret values are never managed by Terraform or CI. Operators add them with
`gcloud secrets versions add <container> --project=firna-apps --data-file=-`;
the merge gate prints the exact command for anything missing.
