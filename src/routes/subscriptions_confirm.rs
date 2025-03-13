use actix_web::{web, HttpResponse, ResponseError};
use sqlx::PgPool;
use uuid::Uuid;
use thiserror::Error;
use anyhow::Context;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[derive(Error, Debug)]
pub enum ConfirmError {
    #[error("{0}")]
    ValidationError(String),
    
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for ConfirmError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            ConfirmError::ValidationError(_) => actix_web::http::StatusCode::UNAUTHORIZED,
            ConfirmError::UnexpectedError(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
pub async fn confirm(
    parameters: web::Query<Parameters>, 
    pool: web::Data<PgPool>
) -> Result<HttpResponse, ConfirmError> {
    let id = get_subscriber_id_from_token(&pool, &parameters.subscription_token)
        .await?
        .ok_or_else(|| ConfirmError::ValidationError("Invalid subscription token.".into()))?;

    confirm_subscriber(&pool, id).await?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, pool))]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), ConfirmError> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id
    )
    .execute(pool)
    .await
    .context("Failed to update subscription status.")?;
    
    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(subscription_token, pool))]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, ConfirmError> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token
    )
    .fetch_optional(pool)
    .await
    .context("Failed to retrieve subscriber ID from token.")?;

    Ok(result.map(|r| r.subscriber_id))
}
