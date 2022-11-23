CREATE TABLE acme_accounts (
  id INTEGER PRIMARY KEY,
  name text not null,
  provider text not null,
  creds text not null
);

CREATE UNIQUE INDEX acme_accounts_name ON acme_accounts (name);

CREATE TABLE dns_providers (
  id INTEGER PRIMARY KEY,
  name text not null,
  provider text not null,
  creds text not null
);

CREATE UNIQUE INDEX dns_provider_name ON dns_providers (name);

CREATE TABLE endpoints (
  id INTEGER PRIMARY KEY,
  name text not null,
  provider text not null,
  creds text not null
);

CREATE UNIQUE INDEX endpoints_name ON endpoints (name);

CREATE TABLE subdomains (
  name text not null primary key,
  acme_account bigint not null references acme_accounts(id),
  dns_provider bigint not null references dns_providers (id),
  endpoint bigint not null references endpoints (id),
  last_cert text,
  expires bigint,
  enabled boolean not null default true
);

CREATE INDEX subdomains_expires ON subdomains(expires);
