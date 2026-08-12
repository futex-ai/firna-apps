//! X tool validation, request construction, and response mapping.

mod accounts;
mod discovery_reads;
mod feeds;
mod list_action_request;
mod list_actions;
mod lists;
mod media_actions;
mod messaging_actions;
mod messaging_reads;
mod metrics;
mod output;
mod post_actions;
mod post_reads;
mod read;
mod relationship_actions;
pub(super) mod runner;
mod spaces;
mod usage;
mod validation;
mod write;

#[cfg(test)]
#[path = "../_tests_/service/mod.rs"]
mod service_tests;
