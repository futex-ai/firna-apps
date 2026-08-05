# Implementation Plans

This directory tracks implementation plans for changes to the Firna-owned app
catalog. A plan remains active until every milestone, verification step, and
post-push review has been completed.

## Active

- [Inbuilt app deploy automation](inbuilt-app-deploy-automation.md): move app
  secret provisioning into a dedicated repository-owned Google Cloud project,
  gate merges on provisioned secret values, and deploy production and br-main
  directly from this repository so the platform repository needs no per-app
  changes.
- [X API access](x-api-access.md): add a cost-bounded, workspace-authorized X
  app for reading and publishing posts through X's pay-per-use API.

## Completed

None yet.
