# EC Invite Format

## Overview

The canonical public invite format is:

```text
{token}@{relay-host[:port]}
```

Examples:

```text
amber-river@relay.example.com
amber-river@relay.example.com:7447
```

The invite is intentionally human-readable. It replaces the older bech32
invite flow and the older requirement for players to manually type relay URLs
and gate public keys.

## Components

- `token` is the two-word seat claim code stored in the hosted-seat rows.
- `relay-host[:port]` is derived from the sysop's configured `relay` URL in
  `nc-gate` config.

`nc-connect` rebuilds the relay URL from that suffix:

- public hosts default to `wss://`
- localhost/private hosts default to `ws://`
- an explicit port is preserved when present

## Sysop Requirements

Hosted EC requires a Nostr relay. The sysop configures the full relay URL in
`nc-gate` config, for example:

```kdl
relay "wss://relay.example.com"
```

or:

```kdl
relay "wss://relay.example.com:7447"
```

The configured relay URL must be convertible to an invite host[:port]:

- scheme must be `ws://` or `wss://`
- username/password are not allowed
- query/fragment are not allowed
- path is not allowed

`nc-sysop nostr seats` renders the public player invite string from that relay
configuration:

```text
amber-river@relay.example.com
```

## Client Join Flow

Given `amber-river@relay.example.com`:

1. Parse the token and relay host[:port].
2. Rebuild the relay URL (`wss://relay.example.com` here).
3. Connect to the relay.
4. Discover the matching 30500 game definition by invite hash.
5. Claim the hosted seat.
6. Use the discovered `ssh-host`, `ssh-port`, `game-id`, and gate identity for
   the normal session handshake.

The invite no longer carries SSH coordinates, game id, or gate key directly.
Those come from relay discovery.

## Notes

- Bare two-word public invites are no longer the normal supported join shape.
- `ecinv1...` bech32 invites are removed from the public flow.
- NIP-05 `nostr.json` bootstrap is not part of the normal invite path.
