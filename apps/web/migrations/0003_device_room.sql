-- Chats hang off one computer room. The room is desktop-{deviceId};
-- it is not a new UUID per "New session".
alter table ship_sessions add column room_id text;
alter table ship_sessions add column device_id text;
create index if not exists ship_sessions_user_room_idx
  on ship_sessions (user_id, room_id);
