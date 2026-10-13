use dotenvy::var;
use sqlx::{Pool, Postgres};

pub async fn setup_database_pool() -> Result<Pool<Postgres>, sqlx::Error>{
    let url = get_db_url();
    let pool = sqlx::PgPool::connect(url.as_str())
        .await
        .expect("Failed to connect to database");
    Ok(pool)
}

pub fn get_db_url() -> String {
    if cfg!(debug_assertions) {
        return var("POSTGRES_CONNECT_DEV").unwrap();
    }
    var("POSTGRES_CONNECT").unwrap()
}