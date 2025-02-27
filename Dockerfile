# Usamos la última versión estable de Rust como imagen base
FROM lukemathwalker/cargo-chef:latest-rust-1.85.0 as chef
WORKDIR /app
RUN apt update && apt install lld clang -y

FROM chef as planner
COPY . .
# Generar un archivo de "receta" similar a un lockfile
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# Construir las dependencias del proyecto, ¡no la aplicación!
RUN cargo chef cook --release --recipe-path recipe.json
# Hasta este punto, si nuestro árbol de dependencias sigue igual,
# todas las capas deberían estar en caché.

COPY . .
COPY sqlx-data.json .
ENV SQLX_OFFLINE true
# Construir nuestra aplicación
RUN cargo build --release --bin zero2prod

FROM debian:stable-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Limpieza
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/zero2prod zero2prod
COPY configuration configuration
ENV APP_ENVIRONMENT production
ENTRYPOINT ["./zero2prod"]