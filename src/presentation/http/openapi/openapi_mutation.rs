// src/presentation/http/openapi/openapi_mutation.rs
use serde_json;

/// Inject an `application/x-www-form-urlencoded` media entry for the
/// `/api/v1/auth/token` operation into a serde_json::Value representing
/// an OpenAPI document. This function is defensive and will create
/// missing path / requestBody / content objects as needed.
pub fn inject_form_media_into_value(v: &mut serde_json::Value) {
    const TOKEN_PATH: &str = "/api/v1/auth/token";
    let schema_ref = serde_json::json!({ "$ref": "#/components/schemas/TokenExchangeRequest" });

    // Helper: get the `paths` map if it exists and is an object. We don't
    // create the top-level `paths` key if it's missing; that matches the
    // original defensive behavior.
    let paths = match v
        .as_object_mut()
        .and_then(|m| m.get_mut("paths"))
        .and_then(|v| v.as_object_mut())
    {
        Some(p) => p,
        None => return,
    };

    // Helper to ensure an entry exists and is an object; create an empty
    // object if the key is missing. Returns the inner object map or None if
    // the existing value is not an object.
    fn ensure_entry_object<'a>(
        map: &'a mut serde_json::Map<String, serde_json::Value>,
        key: &str,
    ) -> Option<&'a mut serde_json::Map<String, serde_json::Value>> {
        let entry = map.entry(key).or_insert_with(|| serde_json::json!({}));
        entry.as_object_mut()
    }

    let path_obj = match ensure_entry_object(paths, TOKEN_PATH) {
        Some(obj) => obj,
        None => return,
    };

    let post_obj = match ensure_entry_object(path_obj, "post") {
        Some(obj) => obj,
        None => return,
    };

    let rb_obj = match ensure_entry_object(post_obj, "requestBody") {
        Some(obj) => obj,
        None => return,
    };

    let content_obj = match ensure_entry_object(rb_obj, "content") {
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
