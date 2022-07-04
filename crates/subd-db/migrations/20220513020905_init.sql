-- Table for maintaining a singular user.
CREATE TABLE users (
    -- TODO: This should probably be something like a UUID
    id integer PRIMARY KEY UNIQUE NOT NULL,

    twitch_id TEXT UNIQUE,
    github_id TEXT UNIQUE,

    -- TODO: Add this back in for metadata things
    -- last_updated DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL

    FOREIGN KEY(twitch_id) REFERENCES twitch_users(id),
    FOREIGN KEY(github_id) REFERENCES github_users(id)
);

CREATE TABLE github_users (
  id    TEXT PRIMARY KEY NOT NULL,
  name  TEXT NOT NULL,
  login TEXT NOT NULL
);

CREATE TABLE twitch_users (
  id                INTEGER PRIMARY KEY NOT NULL,
  login             TEXT UNIQUE NOT NULL,
  display_name      TEXT NOT NULL,
  offline_image_url TEXT,
  profile_image_url TEXT,
  account_created_at TEXT,

  -- TODO: Should we periodically update people's user stuff? Once a day? Cron job? etc?
  -- User’s broadcaster type: "partner", "affiliate", or "".
  broadcaster_type TEXT NOT NULL,

  -- User’s account type: "staff", "admin", "global_mod", or "".
  account_type TEXT NOT NULL

  -- UNUSED:
  -- description	string	User’s channel description.
  -- email	string	User’s verified email address. Returned if the request includes the user:read:email scope.
  -- view_count	integer	Total number of views of the user’s channel.
);


CREATE TABLE twitch_moderators (
  broadcaster_id    INTEGER NOT NULL,
  user_id           INTEGER NOT NULL,
  last_updated      DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,

  FOREIGN KEY(broadcaster_id) REFERENCES twitch_users(id),
  FOREIGN KEY(user_id)      REFERENCES twitch_users(id)
);

CREATE TABLE twitch_gifted_subscriptions (
  id integer PRIMARY KEY AUTOINCREMENT,
  broadcaster_id  INTEGER NOT NULL,
  user_id         INTEGER NOT NULL,
  gifter_id       INTEGER NOT NULL,

  FOREIGN KEY(broadcaster_id)   REFERENCES twitch_users(id),
  FOREIGN KEY(user_id)        REFERENCES twitch_users(id),
  FOREIGN KEY(gifter_id)      REFERENCES twitch_users(id)
);

CREATE TABLE twitch_subscriptions (
  user_id     INTEGER NOT NULL,

  -- 0 to 3, Twitch Sub Tiers (0 represents
  tier        INTEGER NOT NULL CHECK(tier >= 0 AND tier <= 3),

  -- if gift_id is NOT NULL, then the sub is a gift
  gift_id     INTEGER,

  -- Start Date can be NULL, because we may not have been running when the
  start_date  DATETIME,
  -- noted date is when we mark down that this subscription is active
  noted_date  DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,

  PRIMARY KEY (user_id, tier, start_date),
  FOREIGN KEY(user_id)      REFERENCES twitch_users(id),
  FOREIGN KEY(gift_id)      REFERENCES twitch_gifted_subscriptions(id)
);

CREATE TABLE TWITCH_CHAT_HISTORY (
    id integer PRIMARY KEY AUTOINCREMENT,
    user_id BLOB,
    msg TEXT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY(user_id) REFERENCES USERS(id)
);

CREATE TABLE USER_THEME_SONGS (
    user_id INTEGER unique NOT NULL,
    song BLOB NOT NULL,

    FOREIGN KEY(user_id) REFERENCES USERS(id)
);

CREATE TABLE USER_THEME_SONG_HISTORY (
  user_id INTEGER NOT NULL,
  played_at DATETIME DEFAULT CURRENT_TIMESTAMP,

  FOREIGN KEY(user_id) REFERENCES USERS(id)
);

-- quickly look up user information in table.
CREATE INDEX twitch_chat_history__user_id on TWITCH_CHAT_HISTORY (user_id);
-- TODO: Should I use another index here to sort by datetime? maybe it will do automatically.

CREATE TABLE USER_ROLES (
  user_id         INTEGER NOT NULL,
  verified_date   DATETIME DEFAULT CURRENT_TIMESTAMP,

  is_github_sponsor boolean NOT NULL,

  is_twitch_mod     boolean NOT NULL,
  is_twitch_vip     boolean NOT NULL,
  is_twitch_founder boolean NOT NULL,
  is_twitch_sub     boolean NOT NULL,

  FOREIGN KEY(user_id) REFERENCES USERS(id)
);

-- CREATE TABLE KNOWN_BAD_GITHUB (
--   user_id INTEGER NOT NULL,
--   github_user TEXT
-- );
