-- Add migration script here

ALTER TABLE Queue RENAME TO all_backgrounds;

CREATE TABLE IF NOT EXISTS queue(
    path TEXT NOT NULL,
    play_order INT NOT NULL,
    PRIMARY KEY (path),
    FOREIGN KEY (path)
        REFERENCES all_backgrounds (path)
            ON DELETE CASCADE
            ON UPDATE CASCADE
);


CREATE TABLE IF NOT EXISTS day_night_playlist(
    path TEXT NOT NULL,
    play_order INT NOT NULL,
    daytime INT NOT NULL,
    PRIMARY KEY (path),
    FOREIGN KEY (path)
        REFERENCES all_backgrounds (path)
            ON DELETE CASCADE
            ON UPDATE CASCADE
);

CREATE TABLE IF NOT EXISTS app_settings(
    setting TEXT PRIMARY KEY NOT NULL,
    enabled INT DEFAULT NULL,
    number INT DEFAULT NULL,
    string TEXT DEFAULT NULL
);

INSERT INTO app_settings (setting, enabled) VALUES ("day_night_playlist", 0);
