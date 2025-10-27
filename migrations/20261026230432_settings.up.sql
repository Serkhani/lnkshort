-- Add up migration script here
create table if not exists settings
(
    id text default 'DEFAULT_SETTINGS' not null primary key,
    encrypted_global_api_key text not null
);

insert into settings (encrypted_global_api_key)
values ('14097f0dc9185f562a26a1db67d5d0a70cc1ab4749b9a47fea202a15a0690b01');