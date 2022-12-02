# remote-ssl-renewal

A command-line utility to Renew LetsEncrypt SSL Certificates for a CDN when the DNS is hosted elsewhere.

This tool sets up a LetsEncrypt DNS challenge, adds the appropriate DNS entry to answer the challenge, and adds the
resulting certificate to the CDN host.

I'm currently using this to generate an SSL certificate for a DigitalOcean Spaces CDN, where the domain's DNS is managed
through Vercel.

## Installation

As of now, this can only be installed using the Rust toolchain's `cargo` command. You can use `cargo install remote-ssl-renewal`, or just clone this repository and build it yourself.

## Concepts

This tool has four concepts which join together into a full SSL renewal workflow.

An **account** corresponds to a LetsEncrypt account, which is essentially just giving them an email address to contact.

A **DNS provider** is the service that manages the DNS entries for your domain. The tool is designed to easily allow
adding new providers, but currently it only supports Vercel.

An **endpoint** is the service that hosts your files, to which the SSL certificate should be uploaded. Currently the tool supports DigitalOcean Spaces CDN.

Finally, a **subdomain** is your subdomain that the endpoint will serve the files from, and for which this tool should
generate the SSL certificate. Each subdomain is linked to an account, DNS provider, and endpoint.

## Usage

To get started the first time, you can run `remote-ssl-renewal init` to generate one of each of the above entities.
After that, you can renew your certificates using `remote-ssl-renewal renew`. This command will only renew certificates
that are within 14 days of expiration, so it can be run daily without worrying about violating LetsEncrypt rate limits.

The full set of commands can be discovered by running `remote-ssl-renewal --help`.

## Data Storage

The data is stored locally in an SQLite3 database in the standard configuration directory for your OS, at the path `remote-ssl-renewal/data.sqlite3`.
The specific directory used for your system can be found [in this documentation for the dirs crate](https://docs.rs/dirs/4.0.0/dirs/fn.config_dir.html).

For example, on MacOS the database will be stored at `$HOME/Library/Application Support/remote-ssl-renewal/data.sqlite3`.
