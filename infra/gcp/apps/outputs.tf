output "project_number" {
  description = "App-secrets project number used in IAM condition resource names."
  value       = data.google_project.current.number
}

output "workload_identity_provider" {
  description = "Full provider resource name for google-github-actions/auth."
  value       = google_iam_workload_identity_pool_provider.github_firna_apps.name
}

output "apps_ci_service_account_email" {
  description = "Merge-gate identity; set as APPS_CI_SERVICE_ACCOUNT repository variable."
  value       = google_service_account.apps_ci.email
}

output "apps_deploy_service_account_email" {
  description = "Deployment identity; set as APPS_DEPLOY_SERVICE_ACCOUNT repository variable."
  value       = google_service_account.apps_deploy.email
}
