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
  description = "Platform preview deploy identity granted read on preview-app and preview-static-app secrets."
  type        = string
  default     = "github-firna-preview@firna-498513.iam.gserviceaccount.com"
}
