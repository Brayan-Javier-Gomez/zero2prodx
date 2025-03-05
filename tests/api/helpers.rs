use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::startup::{get_connection_pool, Application};

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}


// Lanzamos nuestra aplicación en segundo plano de alguna manera...
pub async fn spawn_app() -> TestApp {


    let email_server = MockServer::start().await;

    // Aleatorizamos la configuración para garantizar el aislamiento de las pruebas
    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        // Usamos una base de datos diferente para cada caso de prueba
        c.database.database_name = Uuid::new_v4().to_string();
        // Usamos un puerto aleatorio asignado por el sistema operativo
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };

    configure_database(&configuration.database).await;


    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");

    // Obtenemos el puerto antes de lanzar la aplicación
    let address = format!("http://127.0.0.1:{}", application.port());
    let _ = tokio::spawn(application.run_until_stopped());

    // Devolvemos la URL de la aplicación
    TestApp {
        address,
        db_pool: get_connection_pool(&configuration.database),
        email_server
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
