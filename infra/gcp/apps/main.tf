locals {
  services = toset([
    "cloudresourcemanager.googleapis.com",
    "iam.googleapis.com",
    "iamcredentials.googleapis.com",
    "secretmanager.googleapis.com",
    "sts.googleapis.com",
  ])
  preview_secret_name_prefixes = [
    "projects/${var.project_id}/secrets/preview-app-",
    "projects/${data.google_project.current.number}/secrets/preview-app-",
  ]
  preview_secret_condition = join(" || ", [
    for prefix in local.preview_secret_name_prefixes :
    "resource.name.startsWith(\"${prefix}\")"
  ])
}

data "google_project" "current" {
  project_id = var.project_id
}

resource "google_project_service" "required" {
  for_each = local.services

  service            = each.value
  disable_on_destroy = false
}

resource "google_iam_workload_identity_pool" "github" {
  workload_identity_pool_id = "github"
  display_name              = "GitHub Actions"

  depends_on = [google_project_service.required]
}

resource "google_iam_workload_identity_pool_provider" "github_firna_apps" {
  workload_identity_pool_id          = google_iam_workload_identity_pool.github.workload_identity_pool_id
  workload_identity_pool_provider_id = "github-firna-apps"
  display_name                       = "firna-apps repository"
  attribute_condition                = "assertion.repository == \"${var.repository}\""

  attribute_mapping = {
    "google.subject"           = "assertion.sub"
    "attribute.repository"     = "assertion.repository"
    "attribute.repository_ref" = "assertion.repository + \":\" + assertion.ref"
  }

  oidc {
    issuer_uri = "https://token.actions.githubusercontent.com"
  }
}

resource "google_service_account" "apps_ci" {
  account_id   = "apps-ci"
  display_name = "firna-apps merge gate (container create and metadata read)"

  depends_on = [google_project_service.required]
}

resource "google_service_account" "apps_deploy" {
  account_id   = "apps-deploy"
  display_name = "firna-apps deployment (secret value read)"

  depends_on = [google_project_service.required]
}

resource "google_service_account_iam_member" "apps_ci_workload_identity" {
  service_account_id = google_service_account.apps_ci.name
  role               = "roles/iam.workloadIdentityUser"
  member             = "principalSet://iam.googleapis.com/${google_iam_workload_identity_pool.github.name}/attribute.repository/${var.repository}"
}

resource "google_service_account_iam_member" "apps_deploy_workload_identity" {
  service_account_id = google_service_account.apps_deploy.name
  role               = "roles/iam.workloadIdentityUser"
  member             = "principalSet://iam.googleapis.com/${google_iam_workload_identity_pool.github.name}/attribute.repository_ref/${var.repository}:refs/heads/main"
}

resource "google_project_iam_custom_role" "app_secret_creator" {
  role_id     = "appSecretCreator"
  title       = "App secret container creator"
  description = "Create app secret containers without reading or writing values."
  permissions = ["secretmanager.secrets.create"]

  depends_on = [google_project_service.required]
}

resource "google_project_iam_member" "apps_ci_viewer" {
  project = var.project_id
  role    = "roles/secretmanager.viewer"
  member  = "serviceAccount:${google_service_account.apps_ci.email}"
}

resource "google_project_iam_member" "apps_ci_creator" {
  project = var.project_id
  role    = google_project_iam_custom_role.app_secret_creator.id
  member  = "serviceAccount:${google_service_account.apps_ci.email}"
}

resource "google_project_iam_member" "apps_deploy_secret_accessor" {
  project = var.project_id
  role    = "roles/secretmanager.secretAccessor"
  member  = "serviceAccount:${google_service_account.apps_deploy.email}"
}

resource "google_project_iam_member" "platform_preview_secret_accessor" {
  project = var.project_id
  role    = "roles/secretmanager.secretAccessor"
  member  = "serviceAccount:${var.preview_deploy_service_account_email}"

  condition {
    title       = "fna_apps_preview_secret_read"
    description = "Allow platform pr-N seeding to read only preview-app secret values."
    expression  = local.preview_secret_condition
  }
}
