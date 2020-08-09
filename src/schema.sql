CREATE TABLE IF NOT EXISTS providers (
    id            TEXT PRIMARY KEY,
    display_name  TEXT NOT NULL,
    logo_url      TEXT NOT NULL,
    access_token  TEXT,
    expires_at    DATETIME,
    refresh_token TEXT
);
