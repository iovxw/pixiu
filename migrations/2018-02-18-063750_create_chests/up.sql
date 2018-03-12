CREATE TABLE chests (
  id       INTEGER PRIMARY KEY NOT NULL,
  position BIGINT              NOT NULL,
  lv       SMALLINT            NOT NULL,
  found_by BIGINT
)
