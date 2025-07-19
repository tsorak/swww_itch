-- Add migration script here

CREATE TABLE IF NOT EXISTS Queue(
    path TEXT PRIMARY KEY NOT NULL,
    play_order INT NOT NULL
);
