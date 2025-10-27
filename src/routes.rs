use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose;
use metrics::counter;
use sqlx::error::ErrorKind;
use sqlx::{Error, PgPool};
use url::Url;

use crate::utils::internal_error;

const DEFAULT_CACHE_CONTROL_HEADER_VALUE: &str =
    "public, max-age=300, s-maxage=300, state-while-revalidate=300, stale-if-error=300";

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub id: String,
    pub target_url: String,
}

fn generate_id() -> String {
    // let bytes: [u8; 9] = rand::random();
    // base64::encode(bytes)
    let randnum = rand::random_range(0..u64::MAX);
    general_purpose::URL_SAFE_NO_PAD.encode(randnum.to_string())
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkTarget {
    pub target_url: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkStats {
    pub amount: Option<i64>,
    pub referer: Option<String>,
    pub user_agent: Option<String>,
}

pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

pub async fn redirect(
    State(pool): State<PgPool>,
    Path(requested_link): Path<String>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    let select_timeout = tokio::time::Duration::from_millis(300);

    let link = tokio::time::timeout(
        select_timeout,
        sqlx::query_as!(
            Link,
            "select id, target_url from links where id = $1",
            requested_link
        )
        .fetch_optional(&pool),
    )
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?
    .ok_or_else(|| "Not found".to_string())
    .map_err(|err| (StatusCode::NOT_FOUND, err))?;
    tracing::debug!(
        "Redirecting link id {} to {}",
        requested_link,
        link.target_url
    );

    let referer = headers
        .get("referer")
        .map(|val| val.to_str().unwrap_or_default().to_string());
    let user_agent = headers
        .get("user-agent")
        .map(|val| val.to_str().unwrap_or_default().to_string());

    let stat_insert_timeout = tokio::time::Duration::from_millis(300);

    let savedstats = tokio::time::timeout(
        stat_insert_timeout,
        sqlx::query("insert into lnkstats (link_id, user_agent, referer) values ($1, $2, $3)")
            .bind(link.id)
            .bind(&user_agent)
            .bind(&referer)
            .execute(&pool),
    )
    .await;

    match savedstats {
        Err(elapsed) => tracing::error!("could not save stat in time: {}", elapsed),
        Ok(Err(err)) => tracing::error!("error while saving stat: {}", err),
        _ => tracing::debug!(
            "id: {}, referer: {:?}, user_agent: {:?} saved",
            requested_link,
            referer.unwrap_or_default(),
            user_agent.unwrap_or_default()
        ),
    }

    Ok(Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header("Location", link.target_url)
        .header("Cache-Control", DEFAULT_CACHE_CONTROL_HEADER_VALUE)
        .body(axum::body::Body::empty())
        .expect("This response should be constructable"))
}

pub async fn create_link(
    State(pool): State<PgPool>,
    Json(new_link): Json<LinkTarget>,
) -> Result<Json<Link>, (StatusCode, String)> {
    let url = Url::parse(&new_link.target_url)
        .map_err(|_| (StatusCode::CONFLICT, "url malformed".into()))?
        .to_string();

    let insert_link_timeout = tokio::time::Duration::from_millis(300);

    for _ in 1..=3 {
        let new_link_id = generate_id();
        let created_link = tokio::time::timeout(
            insert_link_timeout,
            sqlx::query_as!(
                Link,
                "insert into links (id, target_url) values ($1, $2) returning id, target_url",
                new_link_id,
                url
            )
            .fetch_one(&pool),
        )
        .await
        // .map_err(internal_error)?
        .map_err(internal_error)?;
        match created_link {
            Ok(link) => {
                tracing::debug!("Created link with id {} targeting: {}", new_link_id, url);
                return Ok(Json(link))
            }
            Err(err)=> match err {
                Error::Database(db_err) if db_err.kind() == ErrorKind::UniqueViolation=>{}
                _=> return Err(internal_error(err))
            }
        }
    }
    tracing::error!("Could not persist new short link. Exhausted all retries");
    counter!("exhausting_unique_entries",).increment(1);
    Err((StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".into()))
    // Ok(Json(created_link))
    // .ok_or_else(|| "Link should have been created".to_string())
    // .map_err(|err| (StatusCode::CONFLICT, err))?;

}

pub async fn update_link(
    State(pool): State<PgPool>,
    Path(old_link_id): Path<String>,
    Json(new_link_target): Json<LinkTarget>,
) -> Result<Json<Link>, (StatusCode, String)> {
    let url = Url::parse(&new_link_target.target_url)
        .map_err(|_| (StatusCode::CONFLICT, "url malformed".to_string()))?
        .to_string();
    let update_timeout = tokio::time::Duration::from_millis(300);

    let new_link = tokio::time::timeout(
        update_timeout,
        sqlx::query_as!(
            Link,
            "update links set target_url=$1 where id=$2 returning id, target_url",
            url,
            old_link_id
        )
        .fetch_one(&pool),
    )
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?;

    tracing::debug!("Updated link with id {} to {}", old_link_id, url);
    Ok(Json(new_link))
}

pub async fn getstats(
    State(pool): State<PgPool>,
    Path(link_id): Path<String>,
) -> Result<Json<Vec<LinkStats>>, (StatusCode, String)> {
    let get_stats_timeout = tokio::time::Duration::from_millis(300);
    print!("{}", link_id);
    let stats = tokio::time::timeout(
        get_stats_timeout,
        sqlx::query_as!(LinkStats,"select count(*) as amount, referer, user_agent from lnkstats where link_id=$1 group by link_id, referer, user_agent",
        link_id)
        .fetch_all(&pool),
    )
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?;

    tracing::debug!("Link Stats with id: {} requested", link_id);

    Ok(Json(stats))
}
