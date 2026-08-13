create table if not exists ship_sessions (
  id          text primary key,
  user_id     text not null,
  title       text not null,
  status      text not null default 'idle',
  source      text not null default '',
  module_name text,
  gate_json   text,
  created_at  timestamptz not null default now(),
  updated_at  timestamptz not null default now()
);
create index if not exists ship_sessions_user_id_idx on ship_sessions (user_id, updated_at desc);

create table if not exists ship_messages (
  id          text primary key,
  session_id  text not null,
  user_id     text not null,
  role        text not null,
  kind        text not null default 'text',
  content     text not null,
  meta_json   text,
  created_at  timestamptz not null default now()
);
create index if not exists ship_messages_session_idx on ship_messages (session_id, created_at);
