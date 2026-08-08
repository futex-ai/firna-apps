//! Current public and opt-in owned-Post metrics lookup.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::request;
use crate::x::metrics_types::{
    GetPostMetricsInput, GetPostMetricsOutput, ProviderPostMetrics, ProviderPostMetricsResponse,
};
use crate::x::response::decode_metrics_response;
use crate::x::types::{
    AppToolCall, PricedToolSuccess, ToolSuccess, ToolUsageReport, ToolUsageUnit,
};

use super::runner::ConfiguredXToolRunner;
use super::validation::{decode_input, validate_ids};

const POSTS_URL: &str = "https://api.x.com/2/tweets";
const POST_READ_UNIT: &str = "post_read";

impl ConfiguredXToolRunner<'_> {
    pub(super) fn get_post_metrics(
        &self,
        call: AppToolCall,
    ) -> Result<PricedToolSuccess, ToolError> {
        let input: GetPostMetricsInput = decode_input(call.input, InvalidInputReason::PostIds)?;
        validate_ids(&input.ids)?;
        let query = BTreeMap::from([
            (String::from("ids"), input.ids.join(",")),
            (
                String::from("tweet.fields"),
                metric_fields(input.include_private_metrics),
            ),
        ]);
        let response = self.http.send(request(
            "GET",
            POSTS_URL,
            &call.installation_id,
            query,
            None,
        ));
        let provider = decode_metrics_response(response)?;
        let output = normalize_metrics(provider, input.ids, input.include_private_metrics)?;
        let usage = ToolUsageReport::Metered {
            units: vec![ToolUsageUnit {
                unit: POST_READ_UNIT,
                quantity: output.result_count as u64,
            }],
        };
        Ok(PricedToolSuccess {
            output: ToolSuccess::GetPostMetrics(output),
            usage,
        })
    }
}

fn metric_fields(include_private_metrics: bool) -> String {
    if include_private_metrics {
        String::from("public_metrics,non_public_metrics")
    } else {
        String::from("public_metrics")
    }
}

fn normalize_metrics(
    provider: ProviderPostMetricsResponse,
    requested_ids: Vec<String>,
    include_private_metrics: bool,
) -> Result<GetPostMetricsOutput, ToolError> {
    if provider.data.is_empty() {
        return Err(ToolError::NotFound);
    }
    let requested: HashSet<&str> = requested_ids.iter().map(String::as_str).collect();
    let mut returned = HashMap::<String, ProviderPostMetrics>::new();
    for metrics in provider.data {
        if !requested.contains(metrics.id.as_str())
            || returned.insert(metrics.id.clone(), metrics).is_some()
        {
            return Err(ToolError::ProviderResponseInvalid);
        }
    }
    let mut metrics = Vec::with_capacity(returned.len());
    let mut missing_ids = Vec::new();
    for id in requested_ids {
        match returned.remove(&id) {
            Some(provider_metrics) => {
                metrics.push(provider_metrics.into_output(include_private_metrics));
            }
            None => missing_ids.push(id),
        }
    }
    let result_count = metrics.len();
    Ok(GetPostMetricsOutput {
        metrics,
        missing_ids,
        result_count,
    })
}
