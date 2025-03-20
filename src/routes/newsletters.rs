//! src/routes/newsletters.rs
use actix_web::http::header::{self, HeaderMap, HeaderValue};
use actix_web::web;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::{http::StatusCode, ResponseError};
use anyhow::Context;
use secrecy::Secret;
use sqlx::PgPool;

use crate::domain::subscriber_email::SubscriberEmail;
use crate::email_client::EmailClient;

use super::subscriptions::error_chain_fmt;

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}
//Manejadores de error:
#[derive(thiserror::Error)]
pub enum PublishError {
    // Nueva variante de error
    #[error("La autenticación ha fallado.")]
    AuthError(#[source] anyhow::Error),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

// Implementamos Debug para mostrar la cadena completa de errores
impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

// Definimos cómo `PublishError` se traduce en un código de estado HTTP
impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                
                response
                    .headers_mut()
                    // `actix_web::http::header` proporciona una colección de constantes
                    // para los nombres de varios encabezados HTTP estándar
                    .insert(header::WWW_AUTHENTICATE, header_value);
                
                response
            }
        }
    }
}

impl std::fmt::Display for SubscriberEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Usamos la implementación de Display de `String`
        self.0.fmt(f)
    }
}



struct Credentials {
    username: String,
    password: Secret<String>,
}


//Implementacion principal de envio de correos
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest
) -> Result<HttpResponse, PublishError> {
    let _credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;
    let subscribers = get_confirmed_subscribers(&pool).await?;

    for subscriber in subscribers {
        // El compilador nos obliga a manejar ambos casos
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email, // ⬅️ Ahora pasamos una referencia
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error, // Registramos la cadena de errores
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid",
                );
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();
    Ok(confirmed_subscribers)
}


fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    // El valor del encabezado, si está presente, debe ser una cadena UTF8 válida
    let header_value = headers
        .get("Authorization")
        .context("El encabezado 'Authorization' no estaba presente")?
        .to_str()
        .context("El encabezado 'Authorization' no era una cadena UTF8 válida.")?;
    
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("El esquema de autorización no era 'Basic'.")?;
    
    let decoded_bytes = base64::decode_config(base64encoded_segment, base64::STANDARD)
        .context("Error al decodificar en base64 las credenciales 'Basic'.")?;
    
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("La cadena de credenciales decodificada no es UTF8 válida.")?;
    
    // Dividir en dos segmentos, usando ':' como delimitador
    let mut credentials = decoded_credentials.splitn(2, ':');
    
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("Se debe proporcionar un nombre de usuario en la autenticación 'Basic'."))?
        .to_string();
    
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("Se debe proporcionar una contraseña en la autenticación 'Basic'."))?
        .to_string();
    
    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}