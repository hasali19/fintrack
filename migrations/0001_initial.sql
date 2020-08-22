CREATE TABLE providers (
    id            TEXT PRIMARY KEY,
    display_name  TEXT NOT NULL,
    logo_url      TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    access_token  TEXT,
    expires_at    TIMESTAMP
);

CREATE TABLE accounts (
    id           TEXT PRIMARY KEY,
    provider_id  TEXT NOT NULL,
    display_name TEXT NOT NULL,

    FOREIGN KEY (provider_id) REFERENCES providers (id)
);

CREATE TABLE transactions (
    id            TEXT PRIMARY KEY,
    account_id    TEXT NOT NULL,
    timestamp     TIMESTAMP NOT NULL,
    amount        REAL NOT NULL,
    currency      TEXT NOT NULL,
    type          TEXT,
    category      TEXT,
    description   TEXT,
    merchant_name TEXT,

    FOREIGN KEY (account_id) REFERENCES accounts (id)
);
