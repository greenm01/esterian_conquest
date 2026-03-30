# Release Signing

Public Rust player downloads are published with a signed checksum manifest.

Current scope:

- the signed files cover the public `ec-connect` archives on GitHub Releases
- the preserved DOS compatibility bundles on the same release page are not part
  of the signed checksum set

## Public Key

Download the public key from
[release-signing-public.asc](release-signing-public.asc).

Current fingerprint:

```text
C350 4EE1 EE38 410C E1C4 33BC 372B 8AAA CB86 7F13
```

## Verify A Release

Download the release assets you want plus:

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
