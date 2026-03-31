# Release Signing

Public GitHub Releases do not currently publish Rust player archives during the
beta, so there is no active public `SHA256SUMS.txt` manifest on the release
page right now.

This page keeps the signing key and verification flow documented for direct
test handoffs and for the future point when public Rust archives return.

## Public Key

Download the public key from
[release-signing-public.asc](release-signing-public.asc).

Current fingerprint:

```text
C350 4EE1 EE38 410C E1C4 33BC 372B 8AAA CB86 7F13
```

## Verify A Release

When a signed Rust player build is distributed, download the archive you want
plus:

- `SHA256SUMS.txt`
- `SHA256SUMS.txt.asc`

Import the public key:

```bash
gpg --import release-signing-public.asc
```

Verify the signed checksum manifest:

```bash
gpg --verify SHA256SUMS.txt.asc SHA256SUMS.txt
```

Check the archive you downloaded against the signed manifest:

```bash
shasum -a 256 -c SHA256SUMS.txt
```
