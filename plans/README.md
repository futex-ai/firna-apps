# Implementation Plans

This directory tracks implementation plans for changes to the Firna-owned app
catalog. A plan remains active until every milestone, verification step, and
post-push review has been completed.

## Active

- [Browser Use app](browser-use-app.md): add a bounded asynchronous Browser
  Use Cloud app with installation-owned task handles and exact workspace-wallet
  settlement of provider-reported run cost.
- [X API access](x-api-access.md): add a cost-bounded, workspace-authorized X
  app for reading and publishing posts through X's pay-per-use API.
- [X post metrics](x-post-metrics.md): add bounded public and owned-Post
  engagement metrics without implying total profile views or Enterprise
  analytics support.

## Completed

- [Inbuilt app deploy automation](inbuilt-app-deploy-automation.md): moved app
  secret provisioning into a dedicated repository-owned Google Cloud project,
  gated merges on provisioned secret values, and deployed production and
  br-main directly from this repository so the platform repository needs no
  per-app changes.
