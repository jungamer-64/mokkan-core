// src/presentation/http/openapi.rs
use crate::application::dto::{ArticleDto, CursorPage, UserDto};
use axum::{
    Router,
    body::Body,
    http::{self, HeaderMap, HeaderValue, Method, StatusCode, header},
    response::{Redirect, Response},
    routing::get,
};
use blake3::hash;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, env, fs, path::Path, sync::OnceLock};
mod openapi_meta;
use openapi_meta::last_modified_str;
use tower_http::cors::{Any, CorsLayer};
use tower_http::compression::CompressionLayer;
use utoipa::openapi::{
    Components,
    security::{Http, HttpAuthScheme, SecurityScheme},
    server::Server,
};
use utoipa::{Modify, OpenApi, ToSchema};
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::{Config, SwaggerUi, Url};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StatusResponse {
    #[schema(example = "ok")]
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserListResponse {
    pub items: Vec<UserDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ArticleListResponse {
    pub items: Vec<ArticleDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(OpenApi)]
#[openapi(
    paths(
    crate::presentation::http::controllers::auth::register,
    crate::presentation::http::controllers::auth::login,
    crate::presentation::http::controllers::auth_sessions::list_sessions,
    crate::presentation::http::controllers::auth_sessions::revoke_session,
        crate::presentation::http::controllers::auth::refresh_token,
        crate::presentation::http::controllers::auth::profile,
        crate::presentation::http::controllers::auth::list_users,
        crate::presentation::http::controllers::auth::update_user,
        crate::presentation::http::controllers::auth::change_password,
        crate::presentation::http::controllers::articles::list_articles,
        crate::presentation::http::controllers::articles::get_article_by_slug,
        crate::presentation::http::controllers::articles::create_article,
        crate::presentation::http::controllers::articles::update_article,
        crate::presentation::http::controllers::articles::delete_article,
        crate::presentation::http::controllers::articles::list_article_revisions,
        crate::presentation::http::controllers::articles::set_publish_state,
        super::routes::health
    ),
    components(
        schemas(
            StatusResponse,
            UserListResponse,
            ArticleListResponse,
            crate::presentation::http::error::ErrorResponse,
            crate::presentation::http::controllers::user_requests::RegisterRequest,
            crate::presentation::http::controllers::user_requests::LoginRequest,
            crate::presentation::http::controllers::user_requests::LoginResponse,
            crate::presentation::http::controllers::user_requests::RefreshTokenRequest,
            crate::application::dto::SessionInfoDto,
            crate::presentation::http::controllers::user_requests::ListUsersParams,
            crate::presentation::http::controllers::user_requests::UpdateUserRequest,
            crate::presentation::http::controllers::user_requests::ChangePasswordRequest,
            crate::presentation::http::controllers::articles::ArticleListParams,
            crate::presentation::http::controllers::articles::CreateArticleRequest,
            crate::presentation::http::controllers::articles::UpdateArticleRequest,
            crate::presentation::http::controllers::articles::PublishRequest,
            crate::application::dto::UserDto,
            crate::application::dto::UserProfileDto,
            crate::application::dto::AuthTokenDto,
            crate::application::dto::CapabilityView,
            crate::application::dto::ArticleDto,
            crate::application::dto::ArticleRevisionDto
        )
    ),
    tags(
        (name = "Auth", description = "Authentication and session endpoints"),
        (name = "Users", description = "User management endpoints"),
        (name = "Articles", description = "Article management endpoints"),
        (name = "System", description = "System level endpoints")
    ),
    modifiers(&ApiDocCustomizer),
    security(("bearerAuth" = [])),
    info(
        title = "Mokkan API",
        description = "Headless CMS backend",
        version = env!("CARGO_PKG_VERSION")
    )
)]
pub struct ApiDoc;

struct ApiDocCustomizer;

impl Modify for ApiDocCustomizer {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        // security scheme
        let components = openapi.components.get_or_insert_with(Components::default);
        let mut http = Http::new(HttpAuthScheme::Bearer);
        http.bearer_format = Some("JWT".into());
        components.add_security_scheme("bearerAuth", SecurityScheme::Http(http));

        // servers (from env)
        let servers = openapi.servers.get_or_insert_with(Vec::new);
        servers.clear();

        for url in collect_server_urls() {
            servers.push(Server::new(url));
        }
    }
}

// ---- OpenAPI lazy singletons ----
static OPENAPI: OnceLock<utoipa::openapi::OpenApi> = OnceLock::new();
static OPENAPI_JSON: OnceLock<String> = OnceLock::new();
static OPENAPI_BYTES: OnceLock<Bytes> = OnceLock::new();
static OPENAPI_ETAG: OnceLock<String> = OnceLock::new();
static OPENAPI_CONTENT_LENGTH: OnceLock<usize> = OnceLock::new();
// process-startup timestamp for Last-Modified when BUILD_DATE is not set

fn openapi_spec() -> &'static utoipa::openapi::OpenApi {
    OPENAPI.get_or_init(|| ApiDoc::openapi())
}

fn openapi_json() -> &'static str {
    OPENAPI_JSON.get_or_init(|| {
        serde_json::to_string_pretty(openapi_spec()).expect("serialize OpenAPI (pretty)")
    })
}

fn openapi_bytes() -> &'static Bytes {
    OPENAPI_BYTES.get_or_init(|| {
        Bytes::from(serde_json::to_vec(openapi_spec()).expect("serialize OpenAPI (compact bytes)"))
    })
}

fn openapi_etag() -> &'static str {
    OPENAPI_ETAG.get_or_init(|| {
        let h = hash(openapi_bytes().as_ref());
        let hex = h.to_hex();
        format!("W/\"{hex}\"")
    })
}

// ---- HTTP handlers / router ----
fn strip_weak_prefix(tag: &str) -> &str {
    let t = tag.trim();
    let t = t.strip_prefix("W/").or_else(|| t.strip_prefix("w/")).unwrap_or(t);
    t.trim()
}

// Return the Last-Modified string the server actually sends (BUILD_DATE or STARTUP_DATE)
// last_modified_str is implemented in the sibling module openapi_meta

fn weak_match(a: &str, b: &str) -> bool {
    strip_weak_prefix(a) == strip_weak_prefix(b)
}

fn set_common_headers(
    mut builder: http::response::Builder,
    etag: &str,
    with_ct: bool,
) -> http::response::Builder {
    if with_ct {
        builder = builder.header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
    }

    let mut b = builder;
    b = b
        .header(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        )
        .header(
            header::CACHE_CONTROL,
            // prefer the server's prerogative to avoid intermediate transforms that
            // could invalidate the ETag/Content-Length coupling; keep revalidation semantics
            HeaderValue::from_static("public, max-age=0, must-revalidate, no-transform"),
        );

    // Last-Modified: use the value we actually send back (BUILD_DATE preferred, otherwise startup time)
    if let Some(v) = last_modified_str() {
        if let Ok(hv) = HeaderValue::from_str(v) {
            b = b.header(header::LAST_MODIFIED, hv);
        }
    }

    b = b
        .header(header::VARY, HeaderValue::from_static("Accept-Encoding"))
        .header(header::ETAG, etag);

    b
}

fn openapi_content_length() -> usize {
    *OPENAPI_CONTENT_LENGTH.get_or_init(|| openapi_bytes().len())
}

fn inm_matches(inm: &HeaderMap, etag: &str) -> bool {
    match inm.get(header::IF_NONE_MATCH) {
        None => false,
        Some(v) => match v.to_str() {
            Err(_) => false,
            Ok(s) => {
                let s = s.trim();
                if s == "*" {
                    return true;
                }
                s.split(',').map(str::trim).any(|t| weak_match(t, etag))
            }
        },
    }
}

fn has_if_none_match(headers: &HeaderMap) -> bool {
    headers.get(header::IF_NONE_MATCH).is_some()
}

fn ims_matches(headers: &HeaderMap) -> bool {
    // Use the actual Last-Modified value we send (BUILD_DATE or STARTUP_DATE)
    let lm = match last_modified_str() {
        Some(v) => v,
        None => return false,
    };

    let ims = match headers.get(header::IF_MODIFIED_SINCE) {
        Some(v) => match v.to_str() {
            Ok(s) => s,
            Err(_) => return false,
        },
        None => return false,
    };

    // parse both dates and compare
    if let (Ok(lm_time), Ok(ims_time)) = (httpdate::parse_http_date(lm), httpdate::parse_http_date(ims)) {
        return ims_time >= lm_time;
    }
    false
}

pub async fn head_openapi(headers: HeaderMap) -> Response {
    let etag = openapi_etag();
    let not_modified = if has_if_none_match(&headers) {
        inm_matches(&headers, etag)
    } else {
        ims_matches(&headers)
    };

    if not_modified {
        let mut resp = set_common_headers(
            Response::builder().status(StatusCode::NOT_MODIFIED),
            etag,
            false,
        )
        .body(Body::empty())
        .unwrap();
        // Ensure compression-related headers are not present for HEAD responses to avoid
        // mismatches between Content-Length and Content-Encoding applied by middleware.
        resp.headers_mut().remove(header::CONTENT_LENGTH);
        resp.headers_mut().remove(header::CONTENT_ENCODING);
        return resp;
    }
    let content_length = openapi_content_length();
    let builder = set_common_headers(Response::builder().status(StatusCode::OK), etag, true)
        .header(header::CONTENT_LENGTH, content_length.to_string());
    let mut resp = builder.body(Body::empty()).unwrap();
    // Remove compression header just in case a middleware adds Content-Encoding on HEAD
    resp.headers_mut().remove(header::CONTENT_ENCODING);
    resp
}

pub async fn serve_openapi(headers: HeaderMap) -> Response {
    let etag = openapi_etag();
    let not_modified = if has_if_none_match(&headers) {
        inm_matches(&headers, etag)
    } else {
        ims_matches(&headers)
    };

    if not_modified {
        let mut resp = set_common_headers(
            Response::builder().status(StatusCode::NOT_MODIFIED),
            etag,
            false,
        )
        .body(Body::empty())
        .unwrap();
        resp.headers_mut().remove(header::CONTENT_LENGTH);
        // Also remove Content-Encoding for 304 responses to avoid any middleware-induced
        // ambiguity between encoding and length.
        resp.headers_mut().remove(header::CONTENT_ENCODING);
        return resp;
    }

    let body = openapi_bytes().clone();
    // Do NOT set Content-Length on GET responses so downstream compression/chunking
    // layers can adjust the response without causing a mismatch.
    set_common_headers(Response::builder().status(StatusCode::OK), etag, true)
        .body(Body::from(body))
        .unwrap()
}

pub fn docs_router_with_options(expose_openapi_json: bool, enable_api_docs: bool) -> Router {
    let mut base = Router::new();

    // If docs UI is enabled, always provide /openapi.json on the same origin so the UI can load it.
    if enable_api_docs || expose_openapi_json {
        base = base.route("/openapi.json", get(serve_openapi).head(head_openapi));
    }

    // Apply CORS only when the JSON is meant to be exposed to other origins.
    if expose_openapi_json {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::HEAD])
            // allow non-simple headers used by browsers to send caching checks
            .allow_headers([header::IF_NONE_MATCH, header::IF_MODIFIED_SINCE])
            // allow client JS to read ETag/cache/vary/last-modified and optionally content-encoding
            .expose_headers([
                header::ETAG,
                header::CACHE_CONTROL,
                header::VARY,
                header::LAST_MODIFIED,
                header::CONTENT_ENCODING,
                header::CONTENT_LENGTH,
            ]);
        base = base.layer(cors);
    }

    if enable_api_docs {
        base.merge(
            SwaggerUi::new("/docs").config(Config::new([Url::new("Mokkan API", "/openapi.json")])),
        )
        .merge(Redoc::with_url("/redoc", "/openapi.json"))
        .route("/", get(|| async { Redirect::permanent("/docs") }))
    } else {
        base
    }
}

pub fn docs_router() -> Router {
    let expose = matches!(env::var("EXPOSE_OPENAPI_JSON").as_deref(), Ok("1"));
    let enable_docs = matches!(env::var("ENABLE_API_DOCS").as_deref(), Ok("1"));
    // Apply compression at the router level when using env-driven router
    docs_router_with_options(expose, enable_docs).layer(CompressionLayer::new())
}

// ---- snapshot writer ----
pub fn write_openapi_snapshot() -> std::io::Result<()> {
    let output_path = env::var("OPENAPI_SNAPSHOT_PATH")
        .unwrap_or_else(|_| "spec/openapi.json".to_string());
    let path = Path::new(&output_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp_path = format!("{}.tmp", output_path);
    let mut file = std::fs::File::create(&tmp_path)?;
    use std::io::Write;
    file.write_all(openapi_json().as_bytes())?;
    file.flush()?;
    fs::rename(tmp_path, path)?;
    Ok(())
}

// ---- helpers ----
fn collect_server_urls() -> Vec<String> {
    let mut urls = Vec::new();

    if let Ok(list) = env::var("PUBLIC_API_URLS") {
        urls.extend(
            list.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.trim_end_matches('/').to_string()),
        );
    }

    if urls.is_empty() {
        if let Ok(url) = env::var("PUBLIC_API_URL") {
            let sanitized = url.trim().trim_end_matches('/').to_string();
            if !sanitized.is_empty() {
                urls.push(sanitized);
            }
        }
    }

    if let Ok(port) = env::var("SERVER_PORT") {
        let port = port.trim();
        if !port.is_empty() {
            urls.push(format!("http://localhost:{port}"));
        }
    }

    if let Some(base_path) = env::var("PUBLIC_API_BASE_PATH")
        .ok()
        .map(|base| base.trim().trim_matches('/').to_string())
        .filter(|base| !base.is_empty())
    {
        urls = urls
            .into_iter()
            .map(|url| format!("{}/{base_path}", url.trim_end_matches('/')))
            .collect();
    }

    let mut seen = HashSet::new();
    urls.retain(|url| seen.insert(url.clone()));
    urls
}

// ---- conversions ----
impl From<CursorPage<UserDto>> for UserListResponse {
    fn from(page: CursorPage<UserDto>) -> Self {
        Self {
            items: page.items,
            next_cursor: page.next_cursor,
            has_more: page.has_more,
        }
    }
}

impl From<CursorPage<ArticleDto>> for ArticleListResponse {
    fn from(page: CursorPage<ArticleDto>) -> Self {
        Self {
            items: page.items,
            next_cursor: page.next_cursor,
            has_more: page.has_more,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, header};

    #[test]
    fn weak_match_handles_strong_and_weak_tags() {
        assert!(weak_match(r#"W/"abc""#, r#""abc""#));
        assert!(weak_match(r#""abc""#, r#"W/"abc""#));
        assert!(!weak_match(r#""abc""#, r#""def""#));
    }

    #[test]
    fn inm_matches_star_header() {
        let mut headers = HeaderMap::new();
        headers.insert(header::IF_NONE_MATCH, HeaderValue::from_static("*"));
        assert!(inm_matches(&headers, r#""anything""#));
    }

    #[test]
    fn inm_matches_comma_separated_values() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::IF_NONE_MATCH,
            HeaderValue::from_static("\"foo\", W/\"bar\""),
        );
        let header_str = headers
            .get(header::IF_NONE_MATCH)
            .unwrap()
            .to_str()
            .unwrap();
        let candidates: Vec<&str> = header_str.split(',').map(str::trim).collect();
        assert!(weak_match(candidates[1], r#"W/"bar""#));
        assert!(inm_matches(&headers, r#"W/"bar""#));
        assert_eq!(candidates, vec!["\"foo\"", "W/\"bar\""]);
    }

    #[test]
    fn weak_match_handles_lowercase_prefix() {
        assert!(weak_match(r#"w/"abc""#, r#""abc""#));
        assert!(weak_match(r#""abc""#, r#"w/"abc""#));
    }

    #[tokio::test]
    async fn serve_openapi_returns_not_modified_when_if_none_match_matches() {
        // build headers that match current etag
        let mut headers = HeaderMap::new();
        headers.insert(
            header::IF_NONE_MATCH,
            HeaderValue::from_static(openapi_etag()),
        );

        let resp = serve_openapi(headers).await;
        assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
    }

    #[tokio::test]
    async fn head_openapi_ok_sets_headers_and_no_body() {
        let headers = HeaderMap::new();
        let resp = head_openapi(headers).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let hs = resp.headers();
        assert!(hs.get(header::ETAG).is_some());
        assert!(hs.get(header::CONTENT_TYPE).is_some());
        assert!(hs.get(header::CONTENT_LENGTH).is_some());
        // HEAD so body must be empty — Content-Length should equal the OpenAPI length
        let cl = hs.get(header::CONTENT_LENGTH).unwrap().to_str().unwrap();
        assert_eq!(cl, openapi_content_length().to_string());
    }

    #[tokio::test]
    async fn head_openapi_returns_not_modified_when_if_none_match_matches() {
        let mut headers = HeaderMap::new();
        headers.insert(header::IF_NONE_MATCH, HeaderValue::from_static(openapi_etag()));
        let resp = head_openapi(headers).await;
        assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
        assert!(resp.headers().get(header::ETAG).is_some());
    }

    #[tokio::test]
    async fn get_openapi_returns_not_modified_on_ims() {
        // Skip if BUILD_DATE not embedded during build
        if option_env!("BUILD_DATE").is_none() {
            return;
        }
        let mut headers = HeaderMap::new();
        headers.insert(
            header::IF_MODIFIED_SINCE,
            HeaderValue::from_static(option_env!("BUILD_DATE").unwrap()),
        );
        let resp = serve_openapi(headers).await;
        assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
    }

    #[tokio::test]
    async fn get_openapi_inm_takes_precedence_over_ims() {
        // If both INM and IMS are present, INM must take precedence per RFC.
        let mut headers = HeaderMap::new();
        headers.insert(header::IF_NONE_MATCH, HeaderValue::from_static(openapi_etag()));
        headers.insert(
            header::IF_MODIFIED_SINCE,
            HeaderValue::from_static("Thu, 01 Jan 1970 00:00:00 GMT"),
        );
        let resp = serve_openapi(headers).await;
        assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
    }

    #[tokio::test]
    async fn get_openapi_returns_ok_when_inm_mismatch_even_if_ims_matches() {
        // Only meaningful when BUILD_DATE is present (we compare against it)
        if option_env!("BUILD_DATE").is_none() {
            return;
        }
        let lm = option_env!("BUILD_DATE").unwrap();
        let mut headers = HeaderMap::new();
        // intentionally mismatching ETag
        headers.insert(header::IF_NONE_MATCH, HeaderValue::from_static("\"some-other\""));
        headers.insert(header::IF_MODIFIED_SINCE, HeaderValue::from_static(lm));
        let resp = serve_openapi(headers).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[test]
    fn build_date_parses_as_httpdate_when_present() {
        if let Some(b) = option_env!("BUILD_DATE") {
            // ensure the compile-time BUILD_DATE (when present) is a valid HTTP date
            assert!(httpdate::parse_http_date(b).is_ok(), "BUILD_DATE must be httpdate format");
        }
    }
}
