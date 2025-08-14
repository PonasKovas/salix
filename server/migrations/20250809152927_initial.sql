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

CREATE TABLE messages (
    id UUID PRIMARY KEY,
    sequence_id BIGSERIAL NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    sent_at TIMESTAMPTZ NOT NULL,

    CONSTRAINT messages_sequence_id_key UNIQUE (sequence_id)
);


CREATE FUNCTION notify_new_message_with_id() RETURNS TRIGGER AS $$
BEGIN
  PERFORM pg_notify(
    'chat', -- || NEW.room_id,
    NEW.id::text -- cast uuid to text, because triggers can only have text payload
  );
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER messages_insert_trigger
    AFTER INSERT ON messages
    FOR EACH ROW EXECUTE FUNCTION notify_new_message_with_id();
