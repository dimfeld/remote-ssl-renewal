CREATE TABLE acme_accounts (
  name text not null primary key,
  creds text not null,
);

CREATE TABLE dns_provider (
  name text not null primary key,
  host text not null,
  creds text not null,
);

CREATE TABLE endpoints (
  name text not null primary key,
  host text not null,
  details text not null,
);

CREATE TABLE subdomains (
  subdomain text not null primary key,
  dns_provider text not null references dns_provider (name),
  endpoint text not null references endpoints (name),
  last_cert text,
  expires bigint
);

CREATE INDEX ON subdomains(expires);
