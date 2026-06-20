# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-06-20

Initial release.

### Added

- Asynchronous `Client` with a builder, credential configuration, and a choice
  of production or sandbox environment.
- `domains().check()` for domain availability (`namecheap.domains.check`).
- `domains().create()` for domain registration (`namecheap.domains.create`).
- `domains().list()` for listing account domains with their auto-renew status
  and expiry (`namecheap.domains.getList`).
- `domains().set_auto_renew()` to enable or disable a domain's auto-renewal
  (`namecheap.domains.setAutoRenew`).
- `domains().dns().get_hosts()` for reading DNS host records
  (`namecheap.domains.dns.getHosts`), plus `GetHostsResult::to_host_records` and
  `HostInfo::to_host_record` to convert them back into writable records for
  read-modify-write updates.
- `domains().dns().set_hosts()` for DNS host records
  (`namecheap.domains.dns.setHosts`), with helpers for A, AAAA, CNAME, MX, and
  TXT records to support email setup (MX, SPF, DKIM, DMARC).
- `users().get_balances()` for account balances
  (`namecheap.users.getBalances`).
- Typed request and response structs for every supported command.
- An `Error` type that separates HTTP transport failures, API error responses,
  and XML decode failures.
- Selectable TLS backend through the `rustls` (default) and `native-tls`
  feature flags.
