use serde_json::json;

use super::support::{capturing_http, invoke, response, success_output};

#[test]
fn media_alt_text_uses_typed_metadata_shape() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({"data": {"id": "31", "associated_metadata": {}}})),
    ));

    let output = invoke(
        &http,
        "x_manage_media",
        json!({"action": "set_alt_text", "media_id": "31", "alt_text": "Diagram"}),
    );

    assert_eq!(success_output(&output)["applied"], true);
    assert_eq!(output["usage"]["cost_usd_micros"], 5_000);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].url, "https://api.x.com/2/media/metadata");
    assert_eq!(
        requests[0].body_json,
        Some(json!({"id": "31", "metadata": {"alt_text": {"text": "Diagram"}}}))
    );
}

#[test]
fn media_subtitle_add_and_delete_use_closed_provider_shapes() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({"data": {"id": "31", "media_category": "TweetVideo"}})),
    ));
    let output = invoke(
        &http,
        "x_manage_media",
        json!({
            "action": "add_subtitles", "media_id": "31", "subtitle_media_id": "32",
            "display_name": "English", "language_code": "EN", "media_category": "TweetVideo"
        }),
    );
    assert_eq!(success_output(&output)["applied"], true);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].url, "https://api.x.com/2/media/subtitles");
    assert_eq!(
        requests[0].body_json,
        Some(json!({
            "id": "31", "media_category": "TweetVideo",
            "subtitles": {"display_name": "English", "id": "32", "language_code": "EN"}
        }))
    );

    let (http, requests) = capturing_http(response(200, Some(json!({"data": {"deleted": true}}))));
    let output = invoke(
        &http,
        "x_manage_media",
        json!({
            "action": "delete_subtitles", "media_id": "31",
            "language_code": "EN", "media_category": "TweetVideo"
        }),
    );
    assert_eq!(success_output(&output)["applied"], true);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].method, "DELETE");
    assert_eq!(requests[0].url, "https://api.x.com/2/media/subtitles");
}
