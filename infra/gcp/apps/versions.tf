terraform {
  required_version = ">= 1.9.0"

  backend "gcs" {
    bucket = "firna-apps-terraform-state"
    prefix = "infra/apps"
  }

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = ">= 6.0.0, < 8.0.0"
    }
  }
}

provider "google" {
  project = var.project_id
}
