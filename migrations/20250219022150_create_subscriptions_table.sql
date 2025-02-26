-- Add migration script here


-- Crear tabla de suscripciones
CREATE TABLE subscriptions(
    id UUID NOT NULL,
    PRIMARY KEY (id),
    email TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    subscribed_at TIMESTAMPTZ
)