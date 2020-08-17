CREATE TABLE IF NOT EXISTS providers (
    id            TEXT PRIMARY KEY,
    display_name  TEXT NOT NULL,
    logo_url      TEXT NOT NULL,
    access_token  TEXT,
    expires_at    DATETIME,
    refresh_token TEXT
);

CREATE TABLE IF NOT EXISTS accounts (
    id           TEXT PRIMARY KEY,
    provider_id  TEXT NOT NULL,
    display_name TEXT NOT NULL,
    last_sync    DATETIME,

    FOREIGN KEY (provider_id) REFERENCES providers (id)
);
