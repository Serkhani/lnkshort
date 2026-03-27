use sqlx::PgPool;

use crate::models::Link;

#[derive(Clone, Default)]
pub struct LinkRepository;

impl LinkRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn find_by_id(&self, pool: &PgPool, id: &str) -> Result<Option<Link>, sqlx::Error> {
        sqlx::query_as!(
            Link,
            "select id, target_url from links where id = $1",
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        &self,
        pool: &PgPool,
        id: &str,
        target_url: &str,
    ) -> Result<Link, sqlx::Error> {
        sqlx::query_as!(
            Link,
            "insert into links (id, target_url) values ($1, $2) returning id, target_url",
            id,
            target_url
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        &self,
        pool: &PgPool,
        id: &str,
        target_url: &str,
    ) -> Result<Link, sqlx::Error> {
        sqlx::query_as!(
            Link,
            "update links set target_url=$1 where id=$2 returning id, target_url",
            target_url,
            id
        )
        .fetch_one(pool)
        .await
    }
}
