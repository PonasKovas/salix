CREATE EXTENSION IF NOT EXISTS citext;

CREATE TABLE users (
    id UUID PRIMARY KEY,
    username citext NOT NULL UNIQUE,
    email citext NOT NULL UNIQUE,
    password TEXT NOT NULL
);

CREATE TABLE active_sessions (
    token UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE chatrooms (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL
);

CREATE TABLE messages (
    id UUID PRIMARY KEY,
    chatroom UUID NOT NULL REFERENCES chatrooms(id) ON DELETE CASCADE,
    sequence_id BIGSERIAL NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    sent_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT messages_sequence_id_chatroom_key UNIQUE (sequence_id, chatroom)
);


CREATE FUNCTION notify_new_message() RETURNS TRIGGER AS $$
DECLARE
  payload TEXT;
BEGIN
  payload := jsonb_build_object(
    'id', NEW.id,
    'sequence_id', NEW.sequence_id,
    'user_id', NEW.user_id,
    'message', NEW.message,
    'sent_at', NEW.sent_at
  )::text;

  -- pg_notify's limit is strictly less than 8000 bytes.
  IF octet_length(payload) >= 8000 THEN
    payload := jsonb_build_object(
      'id', NEW.id,
      'sequence_id', NEW.sequence_id,
      'user_id', NEW.user_id,
      -- no message
      'sent_at', NEW.sent_at
    )::text;
  END IF;

  PERFORM pg_notify(
    'chat-' || NEW.chatroom,
    payload
  );
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER messages_insert_trigger
    AFTER INSERT ON messages
    FOR EACH ROW EXECUTE FUNCTION notify_new_message();
