variable "project_id" {
  description = "App-secrets Google Cloud project id, matching deploy.toml [gcp] project_id."
  type        = string
  default     = "firna-apps"
}

variable "repository" {
  description = "GitHub repository allowed to mint workload identity tokens."
  type        = string
  default     = "futex-ai/firna-apps"
}

variable "preview_deploy_service_account_email" {
  description = "Platform preview deploy identity granted read on preview-app secrets for pr-N seeding."
  type        = string
  default     = "github-firna-preview@firna-498513.iam.gserviceaccount.com"
}

variable "app_review_deploy_service_account_email" {
  description = "Platform br-apps deploy identity granted read on review-app secrets only. Leave empty until the platform receiver publishes its identity."
  type        = string
  default     = ""
}
