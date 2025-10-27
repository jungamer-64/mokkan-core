// src/presentation/http/openapi/openapi_mutation.rs
use serde_json;

/// Inject an `application/x-www-form-urlencoded` media entry for the
/// `/api/v1/auth/token` operation into a serde_json::Value representing
/// an OpenAPI document. This function is defensive and will create
/// missing path / requestBody / content objects as needed.
pub fn inject_form_media_into_value(v: &mut serde_json::Value) {
    const TOKEN_PATH: &str = "/api/v1/auth/token";
    let schema_ref = serde_json::json!({ "$ref": "#/components/schemas/TokenExchangeRequest" });

    let paths = match v
        .as_object_mut()
        .and_then(|m| m.get_mut("paths"))
        .and_then(|v| v.as_object_mut())
    {
        Some(p) => p,
        None => return,
    };

    let path_item = paths
        .entry(TOKEN_PATH)
        .or_insert_with(|| serde_json::json!({}));
    let path_obj = match path_item.as_object_mut() {
        Some(obj) => obj,
        None => return,
    };

    let post = path_obj
        .entry("post")
        .or_insert_with(|| serde_json::json!({}));
    let post_obj = match post.as_object_mut() {
        Some(obj) => obj,
        None => return,
    };

    let rb_val = post_obj
        .entry("requestBody")
        .or_insert_with(|| serde_json::json!({}));
    let rb_obj = match rb_val.as_object_mut() {
        Some(obj) => obj,
        None => return,
    };

    let content_val = rb_obj
        .entry("content")
        .or_insert_with(|| serde_json::json!({}));
    let content_obj = match content_val.as_object_mut() {
        Some(obj) => obj,
        None => return,
    };

    // If application/json exists, reuse it for the form media type.
    if let Some(json_media) = content_obj.get("application/json").cloned() {
        content_obj
            .entry("application/x-www-form-urlencoded")
            .or_insert(json_media);
    } else {
        content_obj
            .entry("application/x-www-form-urlencoded")
            .or_insert_with(|| serde_json::json!({ "schema": schema_ref }));
    }
}
