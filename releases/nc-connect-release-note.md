<!-- NC-RUST-VERIFY:START -->
## Verify Rust downloads

The Rust-built `nc-connect` downloads in this release can be verified with the signed `SHA256SUMS.txt` manifest.

`gpg --verify SHA256SUMS.txt.asc SHA256SUMS.txt`
`shasum -a 256 -c SHA256SUMS.txt`

Full instructions and public key: https://github.com/greenm01/nostrian-conquest/blob/main/docs/release-signing.md
Signing key fingerprint: `C3504EE1EE38410CE1C433BC372B8AAACB867F13`

The signed manifest covers the public Windows x64, Linux x64, and macOS Apple Silicon `nc-connect` archives on this page, not the DOS compatibility bundles.
<!-- NC-RUST-VERIFY:END -->
