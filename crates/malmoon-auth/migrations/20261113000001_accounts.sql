-- Accounts: a user is a person, an identity is one way that person signs in.
--
-- Email sign-in is an identity like any other: its provider is 'email' and its subject is the
-- address. Nothing about a user is keyed on an email address, so a person can link providers
-- that report different addresses, and a provider can change the address it reports.

create table users (
    id uuid primary key,
    display_name text,
    created_at timestamptz not null default now()
);

create table identities (
    id uuid primary key,
    user_id uuid not null references users (id) on delete cascade,
    provider text not null check (provider in ('email', 'google', 'apple', 'kakao', 'line')),
    -- The provider's stable id for the person (the OpenID Connect `sub`), or the address for
    -- 'email'. Never the email a social provider reports: that can change.
    subject text not null,
    -- Lowercased. What the provider last reported; null when it shares none.
    email text,
    -- True only when the provider vouches that the person controls the address.
    email_verified boolean not null default false,
    display_name text,
    created_at timestamptz not null default now(),
    last_login_at timestamptz not null default now(),
    unique (provider, subject)
);

create index identities_user_id on identities (user_id);
-- Finds the account that already owns a verified address when a new identity arrives.
create index identities_verified_email on identities (email) where email_verified;

-- Only the SHA-256 of a session token is stored, so a database leak does not leak sessions.
create table sessions (
    id uuid primary key,
    user_id uuid not null references users (id) on delete cascade,
    token_hash bytea not null unique,
    client text not null check (client in ('desktop', 'web', 'mobile')),
    label text,
    created_at timestamptz not null default now(),
    last_used_at timestamptz not null default now(),
    expires_at timestamptz not null
);

create index sessions_user_id on sessions (user_id);

-- One browser round trip to a provider, from `start` to the provider's callback.
create table oauth_flows (
    state_hash bytea primary key,
    provider text not null,
    nonce text not null,
    -- PKCE toward the provider, for the providers that support it.
    pkce_verifier text,
    client text not null,
    -- Where the app wants the browser sent afterwards, already checked against the allow list.
    redirect_uri text not null,
    app_state text,
    -- PKCE toward our own token endpoint: only the app that started the flow can finish it.
    code_challenge text not null,
    -- Set when a signed-in user is adding a provider rather than signing in.
    link_user_id uuid references users (id) on delete cascade,
    expires_at timestamptz not null
);

-- The one-time code the app receives on its redirect URI and trades for a session.
create table login_codes (
    code_hash bytea primary key,
    user_id uuid not null references users (id) on delete cascade,
    client text not null,
    code_challenge text not null,
    expires_at timestamptz not null
);

create table email_challenges (
    id uuid primary key,
    email text not null,
    code_hash bytea not null,
    attempts integer not null default 0,
    link_user_id uuid references users (id) on delete cascade,
    created_at timestamptz not null default now(),
    expires_at timestamptz not null,
    consumed_at timestamptz
);

create index email_challenges_email on email_challenges (email, created_at);
