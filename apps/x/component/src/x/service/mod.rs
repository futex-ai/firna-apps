//! X tool validation, request construction, and response mapping.

mod metrics;
mod output;
mod read;
pub(super) mod runner;
mod validation;
mod write;

#[cfg(test)]
#[path = "../_tests_/service/mod.rs"]
mod service_tests;
