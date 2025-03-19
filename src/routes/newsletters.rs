//! src/routes/newsletters.rs
use actix_web::HttpResponse;

// Implementación dummy
pub async fn publish_newsletter() -> HttpResponse {
    HttpResponse::Ok().finish()
}