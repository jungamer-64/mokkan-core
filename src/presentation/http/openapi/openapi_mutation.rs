// src/presentation/http/openapi/openapi_mutation.rs
use serde_json;

fn ensure_entry_object<'a>(
    map: &'a mut serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<&'a mut serde_json::Map<String, serde_json::Value>> {
    let entry = map.entry(key).or_insert_with(|| serde_json::json!({}));
    entry.as_object_mut()
}

/// Inject an `application/x-www-form-urlencoded` media entry.
///
/// This updates the `/api/v1/auth/token` operation in a `serde_json::Value`
/// representing an `OpenAPI` document. It defensively creates missing
/// path, `requestBody`, and `content` objects as needed.
pub fn inject_form_media_into_value(v: &mut serde_json::Value) {
    const TOKEN_PATH: &str = "/api/v1/auth/token";
    let schema_ref = serde_json::json!({ "$ref": "#/components/schemas/TokenExchangeRequest" });

    // Helper: get the `paths` map if it exists and is an object. We don't
    // create the top-level `paths` key if it's missing; that matches the
    // original defensive behavior.
    let Some(paths) = v
        .as_object_mut()
        .and_then(|m| m.get_mut("paths"))
        .and_then(|v| v.as_object_mut())
    else {
        return;
    };

    let Some(path_obj) = ensure_entry_object(paths, TOKEN_PATH) else {
        return;
    };

    let Some(post_obj) = ensure_entry_object(path_obj, "post") else {
        return;
    };

    let Some(rb_obj) = ensure_entry_object(post_obj, "requestBody") else {
        return;
    };

    let Some(content_obj) = ensure_entry_object(rb_obj, "content") else {
        return;
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
