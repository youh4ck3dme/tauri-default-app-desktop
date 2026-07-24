# Websupport account pairing via sidebar context menu

## Idea

When the app starts, right-clicking (and possibly long-pressing, for touch)
the "Websupport" item in the left sidebar should pop up a small "connect your
account" UI, instead of only being reachable via Preferences → Integrations.

Once paired, the Websupport nav group's subitems (Domény / Email / DNS)
should show real data from the account instead of static placeholders.

## Unresolved — needs verification before implementing

**Email + password role is not yet defined.** Websupport's public REST API
(v1 and v2) only supports Identifier + Secret Key HMAC authentication — there
is no email/password login endpoint. Before building this popup, decide one
of:

1. Email is just a display label ("connected as name@example.com") next to
   the real auth fields (Identifier + Secret Key, same ones already stored
   via `save_secret`/`get_secret` in the OS keychain). No password is sent
   anywhere. — Likely correct interpretation, but not yet confirmed.
2. Email + password are meant to validate against the actual Websupport.sk
   website login (not the REST API). This would require browser automation
   of their login form — fragile, could violate their ToS, and breaks on any
   UI change on their end. Do NOT build this without explicit confirmation
   and a review of Websupport's terms of service first.

**Do not implement until this is clarified** — building the wrong one risks
either a broken feature or a ToS violation.

## Long-press (touch) gesture

Raised as an idea for touch/trackpad long-press in addition to right-click.
Decided for now: **desktop right-click context menu only** — no custom
long-press gesture detection. Revisit if the app ever targets a touch-first
device.

## Depends on

The Websupport v1/v2 API integration currently being built (DNS records via
v2, domains/mailboxes via v1) and the existing secrets storage in
`src-tauri/src/commands/secrets.rs` / `src/services/secrets.ts` /
`IntegrationsPane.tsx`.
