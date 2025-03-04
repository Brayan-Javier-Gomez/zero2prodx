use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::email_client::EmailClient;


pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}
// Lanzamos nuestra aplicación en segundo plano de alguna manera...
pub async fn spawn_app() -> TestApp {

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");

    // Obtenemos el puerto asignado por el sistema operativo
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();

    let connection_pool = configure_database(&configuration.database).await;

    // Construir un nuevo `EmailClient`
    let sender_email = configuration.email_client.sender()
        .expect("Invalid sender email address.");

    let timeout = configuration.email_client.timeout();
    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
        timeout
    );

    let server =
        zero2prod::startup::run(listener, connection_pool.clone(), email_client).expect("Failed to bind address");

    // Ejecutamos la aplicación en segundo plano
    let _ = tokio::spawn(server);

    // Devolvemos la URL de la aplicación
    TestApp {
        address,
        db_pool: connection_pool,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Crear la base de datos
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database");

    // Migrar la base de datos
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}


