# ADR 0010: Desktop URI handoff and restart recovery

- Status: Accepted
- Date: 2026-10-29
- Related: [ADR 0005](0005-github-authorization-contract.md), [ADR 0008](0008-application-authentication-state.md), [ADR 0009](0009-application-sessions.md), issues #271 and #193

## Desktop return decision

Use a private-use URI scheme to return from the browser to the desktop
application, including launching the application when needed. GitHub still
returns its authorization response to the fixed Cloudflare HTTPS callback in
ADR 0005. The custom URI is a separate application handoff step.

Treat every incoming URI as untrusted. It may identify a pending attempt, but
must not contain a provider token, application session token, refresh token,
identity, or the desktop's private claim verifier. Receiving the URI does not
authenticate the user. The originating desktop must prove possession of a
separate per-attempt verifier to consume the handoff once through the HTTPS API.
Never reconstruct that verifier from URI parameters or accept an unknown
attempt as a new login. Do not log raw URI or command-line arguments.

The desktop callback has this configuration-defined shape:

```text
<configured-scheme>:<configured-absolute-path>?attempt_id=<opaque-handle>
```

The placeholders are not literal URI characters. The absolute path starts with
`/`; the handle is server-issued unpadded base64url. Neither `io.github` nor any
particular account name is a protocol requirement, and GitHub Pages hosting is
not required. Registration, native validation and server rendering use the
build/deployment profile, rather than copying a concrete URI from this document.

### Official build defaults

These values describe the official distribution's defaults only. Fork builds
may replace both scheme and path through their profile definition.

| Environment | Default scheme | Default absolute path |
| --- | --- | --- |
| Release | `win.9renpoto.time-wise` | `/auth/complete` |
| Development | `win.9renpoto.time-wise.dev` | `/auth/complete` |

The official deployment allocation uses the controlled `9renpoto.win` domain:

| Environment | Authentication origin | GitHub provider callback |
| --- | --- | --- |
| Release | `https://auth.time-wise.9renpoto.win` | `https://auth.time-wise.9renpoto.win/auth/github/callback` |
| Development | `https://auth-dev.time-wise.9renpoto.win` | `https://auth-dev.time-wise.9renpoto.win/auth/github/callback` |

The desktop scheme uses the reverse-domain `win.9renpoto` namespace. These are
allocated official deployment settings, not a requirement for forks or other
self-hosted deployments. They do not change the existing release bundle ID or
history storage identity. DNS records, Worker custom-domain bindings, TLS and
GitHub App registration are implementation/acceptance work in #194; this ADR
alone does not establish that the addresses are provisioned or reachable.


Maintain one versioned, nonsecret connection-profile definition for profile ID,
environment, registered scheme and fixed return path. Generate Tauri scheme
registration, native validation constants and the server's available profile
catalog from that definition; verify they agree during build/deployment checks.
Each deployment explicitly enables an allowlisted subset of those profiles.
Fork maintainers may freely specify their own desktop callback URI at build
time through this definition, including its scheme and callback path; they are
not restricted to the official profile IDs, namespace or `/auth/complete` path.
Validate the configured URI against the supported private-use URI grammar
(scheme and absolute path, no authority, user information, port, query or fragment).
Append only the protocol-defined `attempt_id` query when constructing a return.
Generate packaging registration and exact native validation from those configured
values. The installed binary accepts its build-time callback, not arbitrary
destinations received at runtime. Fork operators install the corresponding
versioned profile in their server's allowlist before login. Changing a desktop
server URL does not change the installed application's registered callback.

Here "desktop callback URI" means the custom-protocol browser-to-app return.
The separate GitHub provider callback is the configured deployment's HTTPS
`/auth/github/callback`, registered in that operator's GitHub App. Neither is
selected from an untrusted login request.

Attempt creation selects a `client_profile_id` advertised by server configuration
and already known to the desktop build. Reject missing, unknown, disabled or
wrong-environment profiles before creating an attempt. Bind the selected profile
and its versioned values immutably to the attempt, including after restart.
Construct preview/return links only from that binding and the opaque handle.
Never accept an arbitrary scheme, return URI, path or origin in a login request,
and never learn a new accepted scheme from an incoming deep link. A profile
change must not silently change an in-flight attempt's return destination.

These URIs have no authority. Reject a host, user information, port, fragment,
unexpected path, duplicate or additional query fields, invalid handle encoding,
and a scheme from the other environment. Never normalize an alternate URI shape
into an accepted one. Scheme registration and both cold and existing-instance
delivery require Windows and macOS acceptance checks.

Isolate development from release across the bundle/single-instance identity,
authentication origin and credential namespace, local data directory, and
autostart identity. Preserve the existing release bundle identifier and history
key entry. This authentication change must not incidentally move existing
history or share its key with development. Do not assume a different URI scheme
alone isolates processes or storage.
Use a development configuration overlay whose bundle identifier is the existing
release identifier plus `.dev`, product/autostart name is `time-wise-dev`, and
deep-link scheme array contains only the development scheme. Authentication
credential service names are `time-wise.auth.release.v1` and
`time-wise.auth.development.v1`; entries are further prefixed by a SHA-256 namespace of the canonical selected
origin, followed by `attempt/HANDLE` or `session/SESSION_ID`. The nonsecret environment-specific
index records handles, hard deadlines, terminal flags and selected session, but
no verifier, token or launch ticket. Use the environment's own Tauri application
data directory and history key namespace for development, without migrating the
existing release history entry. The bundle overlay does not automatically
change independently configured credential or autostart names.

### Restart recovery decision

Allow a restarted desktop to resume its original authentication attempt while
that attempt or its handoff remains valid. Store the attempt's independent
private verifier temporarily in device-local OS credential storage, together
with the identity and expiry needed to find and validate the pending attempt.
Do not include provider tokens or history-decryption keys in this pending state.

Persist the pending secret successfully before opening the browser. On restart,
require the matching local record and check expiry before any claim; the server
also enforces its deadline independently. A missing or invalid verifier requires
a new authentication attempt, never a replacement verifier generated from the
incoming URI. Credential-store unavailability must not fall back to plaintext.

Remove pending secrets after terminal completion or cancellation, and purge
expired records when the application next runs. OS credential storage does not
provide an automatic TTL while the app is stopped. Use separate environment-
and attempt-specific credential identities, outside the existing usage-history
database-key entry. Removal of authentication state must not delete history keys.
The recovery contract below defines retention and deletion failures.

### Browser/desktop comparison decision

Show the same short handoff confirmation code in the originating desktop and
the browser. Require the user to compare them before confirming return to the
application. This code identifies the intended login attempt; it is not the
GitHub authorization code, a claim verifier, or history-decryption enrollment.
Require an explicit browser confirmation followed by a separate return button.
The server records confirmation before allowing a desktop claim. Protect that
same-origin POST with the initiating-browser binding and CSRF validation.
A preview link may open a closed app to show the matching local attempt's code;
preview must not consume the handoff or bypass browser confirmation. Do not
depend on automatic custom-protocol navigation. The protocol below defines
the browser binding and APIs.

### Parallel attempts and reauthentication decisions

Only the locally selected attempt may be claimed. Another attempt's URI can
show its matching local record and comparison code, but cannot select it or
replace the current application session automatically. Selecting another
attempt is explicit; adopting a login while a session exists requires a switch
confirmation before claim. Preserve the old session until the new refresh
credential is securely stored. Do not cancel unrelated attempts, change the
history key, or grant history decryption as part of switching accounts.
After adopting the replacement, attempt current-session logout of the old
session and discard its credentials. If remote revocation is unconfirmed, say
so; do not keep a hidden retry credential. Existing history and synchronized
records retain their ownership; switching does not reassign or upload them to
the new account.

Reauthentication starts with an active application access credential. Bind the
attempt to that session and account on the server, without accepting a target
account from the request body. Require the verified GitHub identity to map to
that same account. A different identity fails without rebinding or silently
turning reauthentication into login. One-time claim records the new provider
verification time on the original active session; it issues no new session or
credential pair. It must not extend the session's original thirty-day deadline.
ADR 0009's global logout uses this fresh-verification time, not a refresh time.
Keep `provider_verified_at_ms` as the original absolute-lifetime anchor. Extend
session inspection with nullable `reauthenticated_at_ms` for the latest explicit
reverification. Global logout requires the same session's fresh verification
within five minutes, using its initial verification or latest reauthentication;
an existing old session does not inherit freshness from a different session.

### Handoff deadline decision

Keep ADR 0005's provider-callback admission deadline at ten minutes after
attempt creation. Browser comparison, confirmation, and desktop claim together
expire five minutes after successful GitHub identity verification. Reject at
or after the deadline. Restart, repeated URI delivery, or confirmation cannot
extend either deadline. Bound provider processing separately so an admitted
callback cannot keep the attempt alive indefinitely; the exact processing
budget and local retention bound remain to be specified.

### Official Tauri integration checked

Tauri provides the official `tauri-plugin-deep-link` for Windows and macOS.
Use its configured desktop schemes, startup `get_current()` API, and running-app
`on_open_url()` events as the integration path. The repository does
not currently include this plugin.

The existing `tauri-plugin-single-instance` is already registered first. Its
`deep-link` feature must be enabled to forward Windows deep-link launches to
the existing instance. The existing official `tauri-plugin-opener` can open the
external HTTPS authorization page. Keep receipt and validation in Rust; these
operations do not require adding frontend scheme-registration privileges.

macOS cannot register schemes at runtime. Tauri's documented acceptance path
uses a bundled application installed in `/Applications`; a normal
`cargo tauri dev` launch is insufficient to validate OS delivery. Windows
supports runtime registration for development, which must use an isolated
development namespace. Validate and deduplicate incoming attempts regardless
of which plugin API delivers the URI.

This check establishes an official integration path, not a tested dependency
change. URI validation, proof of possession, one-time D1 consumption, and
session handling remain application responsibilities.

### Required return-page amendment

ADR 0005 currently specifies a static outcome-only callback page with no
external links. The custom-URI choice requires a narrow amendment to that page
contract. This amendment permits a server-issued comparison code, a first-party
confirmation form, and a user-activated return button after confirmation. Its
URI uses fixed allowlisted configuration and an opaque attempt handle;
provider secrets and arbitrary caller-selected destinations remain forbidden.
This ADR supersedes ADR 0005 only for the page exception and explicit
reauthentication mismatch outcome described below. No
deployed authentication flow or data requires migration.

## Detailed protocol

### Self-hosted deployment premise

Support operator-owned Cloudflare deployments through a Deploy to Cloudflare
button and reproducible setup instructions. There is no single global
authentication hostname. `AUTH_ORIGIN` is fixed within each deployment, while
the official desktop selects its connection explicitly. The previous
single-service client assumption is superseded by the connection boundaries below.

Each deployment owns its Worker, D1, GitHub App registration, callback URL,
secrets and external restore generation. The deployment button is an entry to
setup, not proof that GitHub registration, schema migration, secret generation
or HTTPS acceptance completed. Publish only a working deployment target once
the server package and deployment checks exist; do not add a placeholder button.

Allow the official desktop binary to select an HTTPS deployment
explicitly in settings. A deep link must never select or change that server.
Partition account/session/pending records by deployment identity as well as
release/development. A GitHub numeric ID on two deployments does not imply a
shared application account; changing servers must not transfer credentials,
history ownership or decryption authority. Validate the selected server's
protocol before login, and bind each pending attempt to its selected origin.
This client-distribution choice is accepted. The official profile IDs are `official-release-v1` and `official-development-v1`.
Keep the selected origin in
environment-specific local settings; changing it requires an explicit settings
action and stops authenticated work against the previous selection. Never send
an old deployment's tokens, claim verifiers or history to the new origin.
Pending attempts retain their original origin and cannot complete into the
newly selected deployment. Their URI may show the original connection, but
resuming requires explicitly selecting that connection again.

Use the canonical HTTPS origin as the initial connection namespace, including
the effective port. Treat origin changes as a new connection; do not discover
aliases or migrate credentials automatically. Reject user information, query,
fragment and non-root paths. Native authenticated HTTP calls must not follow
cross-origin redirects. A selected operator controls the authentication server;
show the exact origin before opening browser authentication. The desktop's
server URL setting never changes the fixed release/development URI scheme.
If a handle matches multiple connection records, reject ambiguity rather than
choose a server from the URI. Expose `GET /v1/auth/config` without credentials
with `protocol_version: 1`, the enabled `client_profile_ids`, and the deployment's
canonical `auth_origin`; require
an exact origin match and supported version before authentication. This is
compatibility discovery, not independent proof of operator trust.
Connection to a user-selected host does not establish permission to decrypt
or upload existing history. Endpoint compatibility is checked before login;
unsupported servers fail without sending credentials from another connection.

Reference: [Cloudflare deployment buttons](https://developers.cloudflare.com/workers/platform/deploy-buttons/).

This layer builds on the persistence and session contracts. The policies and complete protocol, timeout values, API and recovery details
below are accepted. Deployment origin values remain
operator-selected per deployment, rather than one repository-wide hostname.
#194 owns the deploy template, origin configuration and GitHub registration
checks; real provider traffic stays disabled until those checks pass.

### Browser binding and processing bounds

Use the following browser binding and processing bounds:

1. Native Rust creates an independent 32-byte random claim verifier, persists it
   in device-local OS credentials, and sends only its SHA-256 challenge when
   creating an attempt. The server issues an independent random attempt handle,
   comparison code, and one-use browser-launch ticket. Persist the returned
   attempt association before opening the fixed-origin HTTPS launch URL. A
   lost create response leaves an expiring orphan, never recoverable by guessing
   a handle. A launch ticket cannot claim or refresh an application session.
2. The first valid launch-ticket redemption atomically binds the pending attempt
   to a new server-generated browser secret and consumes the ticket. Set an
   attempt-specific `__Host-` cookie with `Secure; HttpOnly; SameSite=Lax; Path=/`
   and no `Domain`, then redirect to the fixed GitHub authorization endpoint.
   Set `Max-Age` to at most the remaining hard lifetime, never renew it, and
   clear the matching cookie on a subsequent terminal browser response.
   Use independent cookie names and hashes for concurrent attempts. A lost cookie
   response or a different browser requires a new attempt, without rebinding.
   Remove the launch ticket from subsequent navigation and redact it from logs.
3. The fixed HTTPS GET callback must match both state and the corresponding
   browser cookie before the one-time processing claim. Missing, evicted, or
   mismatched cookies fail closed without consuming another attempt. Cookie
   attributes do not replace primary-side expiry/generation checks. Never use
   browser cookies to authenticate native APIs.
4. After provider verification, the same browser may view the attempt's
   comparison page and submit a confirmation with a separate CSRF nonce.
   Require the exact configured `Origin`, matching cookie, nonce, unexpired
   verified state, and an atomic confirmation transition. A duplicate valid
   confirmation can show the already-ready page; it cannot extend the deadline
   or publish another handoff. Browser pages contain neither account identity
   nor native claim/session credentials. The comparison code is display-only,
   never an authentication credential.
5. Bound callback processing, including exchange, identification, cleanup and
   publication, to thirty seconds from its atomic admission. Accept provider
   identity only before that budget ends, and give each outbound call at most
   ten seconds within the remaining budget. Do not retry exchange automatically.
   Reserve cleanup time; cleanup failure still does not reverse a verified
   identity, but publication after the total budget fails closed. The final
   handoff deadline is `provider_verified_at + 300 seconds`, bounded above by
   `attempt_created_at + 930 seconds`. Late results cannot publish or extend it.
   A stopped callback remains consumed. Pending native records use that same
   hard upper bound, and shorten to an earlier known terminal deadline.

These are application choices, not mandated cookie lifetimes or RFC timeout
values. Limit one native environment to four unfinished attempts, and one
browser environment to four live attempt cookies. Reject excess creation or
bootstrap before consuming a ticket; do not evict another independent attempt.
Primary-side rate limiting precedes one-time transitions and returns 429 with
`Retry-After`. Broader abuse thresholds belong to #202 and cannot weaken the
one-time, isolation, expiry, or fail-closed guarantees.

### State, confirmation and transaction boundaries

The server states are `created` (launch ticket unused), `pending` (browser
bound), `processing` (callback claimed), `verified` (identity/account published),
`ready` (browser confirmed), and `consumed` (handoff claimed). `failed`,
`cancelled`, and `expired` are terminal. No terminal state is reopened. Cookie
response loss after bootstrap and process loss after callback claim require a
new attempt. A valid browser may reload a clean result page without replaying
the callback. If confirmation committed but its response was lost, reloading
shows `ready` without issuing a second handoff.

Create the handle, launch ticket, cookie secret, CSRF nonce and native verifier
independently using at least 32 cryptographically random bytes. Encode binary
handles/verifiers as 43 unpadded base64url characters. Native creates its claim
challenge as unpadded base64url SHA-256 of the ASCII verifier. Persist only
purpose-separated hashes for equality checks. The launch ticket expires with
the ten-minute admission window; it is not a replacement for the claim verifier.
Generate a random ten-character comparison code from the unambiguous alphabet
`0123456789ABCDEFGHJKMNPQRSTVWXYZ`, displayed as two groups of five. Its only
purpose is comparison; collision or knowledge of the code conveys no authority.

Apply ADR 0008 guards to every dependent mutation. Login callback publication
binds the verified identity to an account and creates only the waiting handoff,
not a session. Reauthentication instead checks the existing target association
and must not create or rebind another account on mismatch. Confirmation requires
the matching browser, `verified`/`ready`, deadline,
generation and account cutoff. Claim requires the original verifier, `ready`,
deadline, generation, account authorization generation and the D1-ordered
pre-global-logout cutoff from ADR 0009. A losing operation never issues anything.
For login, consumption and session issuance commit together. For reauthentication,
consumption and the active target session's fresh-verification timestamp commit
together. If the target was revoked or globally logged out meanwhile, fail closed.

Authenticated cancellation of a nonconsumed attempt competes with publication
and claim under the same guards. Cancellation committed first prevents their
dependent writes, even if a provider request is already in flight. Cancellation
does not undo account association or revoke an already-issued session. Return a
consumed conflict when claim won, and use explicit issued-session revocation for
that case. Cancellation never targets another attempt or a browser's other cookies.

### HTTP interfaces and browser page exception

Use one deployment-configured `AUTH_ORIGIN` per environment, HTTPS with no user
information, path, query or fragment. Do not infer it from request headers or
accept redirect/origin/scheme parameters. The environment also fixes its D1,
Worker secrets, GitHub App and enabled return-profile catalog. Distinct HTTPS hostnames prevent
cookie sharing; different ports on one hostname are insufficient. No default
production hostname or implicit localhost fallback exists. Local celld HTTP is
for synthetic providers/data only. Actual HTTPS values and registration gates
must be resolved before enabling real provider traffic.

Native endpoints use JSON, `Cache-Control: no-store`, the fixed origin and Rust
HTTP calls. For attempt operations, `Authorization: Bearer CLAIM_VERIFIER`
authorizes only that attempt. Native APIs reject browser-cookie authentication
and do not expose permissive CORS. Bodies cannot select an account or return URL.

| Endpoint | Request / authentication | Success |
| --- | --- | --- |
| `POST /v1/auth/attempts` | `purpose: login`, `claim_challenge`, `client_profile_id`; no application session required | 201, attempt descriptor below. |
| `POST /v1/auth/attempts` | `purpose: reauthenticate`, `claim_challenge`, `client_profile_id`, active application Bearer access | 201, descriptor bound to that session/account. |
| `GET /v1/auth/attempts/{id}` | Claim verifier | 200, state, comparison code, callback/handoff/hard deadlines and purpose; no identity or session secrets. |
| `POST /v1/auth/attempts/{id}/claim` | Claim verifier, empty JSON | 200, ADR 0009 credential pair for login, or `{ "result": "reauthenticated", "verified_at_ms": 0 }` for reauthentication. |
| `POST /v1/auth/attempts/{id}/cancel` | Claim verifier, empty JSON | 204, idempotent nonconsumed cancellation; 409 if already consumed. |
| `POST /v1/auth/attempts/{id}/revoke-issued-session` | Claim verifier, empty JSON | 204, atomically prevent pending issuance or revoke only the session created by this login attempt; never issue or reveal credentials. |
| `GET /auth/github/start?ticket=VALUE` | One-use launch ticket | 303, atomically bind cookie and redirect to fixed GitHub authorization. |
| `GET /auth/github/callback` | ADR 0005 state/code/error plus bound cookie | ADR 0005 statuses plus 409 `identity_mismatch` for a reauthentication target mismatch; verified success shows comparison/form directly. |
| `GET /auth/github/attempts/{id}` | Bound browser cookie | 200, minimal comparison/result page appropriate to the state. |
| `POST /auth/github/attempts/{id}/confirm` | Bound cookie, exact Origin and form `csrf_token` | 303 to the same-origin result page, now showing the return link if ready. |

Creation request, with a synthetic challenge placeholder (actual values must
meet the encoding and entropy requirements):

```json
{"purpose":"login","claim_challenge":"synthetic-challenge","client_profile_id":"official-release-v1"}
```

Creation descriptor (synthetic clock starts at zero):

```json
{
  "attempt_id": "synthetic-attempt",
  "purpose": "login",
  "client_profile_id": "official-release-v1",
  "comparison_code": "7K3DM-P9W2R",
  "launch_url": "https://auth.example.com/auth/github/start?ticket=synthetic-ticket",
  "created_at_ms": 0,
  "callback_expires_at_ms": 600000,
  "hard_expires_at_ms": 930000
}
```

Status includes the descriptor's nonsecret identifiers, purpose, code and
deadlines plus `state`; omit `launch_url` and include nullable
`handoff_expires_at_ms` until verified. Never recover a consumed launch ticket.
Accept only exact purpose values, an enabled profile ID, an encoded SHA-256 challenge, and no additional
fields; return 400 `invalid_request` otherwise. Protect status and mutations
before revealing existence: unknown ID and invalid verifier both return 404
`attempt_unavailable`. A verified caller sees 409 `attempt_not_ready` or
`attempt_consumed`, 410 `attempt_expired`/`attempt_cancelled`, 401
`invalid_session` for a missing/revoked reauthentication target, 429
`rate_limited`, or 503 `authentication_unavailable`. Reauthentication account
mismatch is terminal `failed` with a generic `identity_mismatch` result to the
proof holder; do not disclose the other identity. A random proof cannot cancel,
inspect or revoke anything. Rate limiting and primary failures cannot turn a
one-time operation into a safe blind retry.
State-conflict errors apply to mutations; authenticated status reads report the
current terminal state with 200 while the proof remains within its hard lifetime.

The narrow ADR 0005 exception permits a first-party result-page link, comparison
code, confirmation form with CSRF nonce, and the configured preview/return URI.
The callback keeps ADR 0005's HTTP status table with the explicit 409
`identity_mismatch` addition for same-account reauthentication; a clean result
page does not process the callback again. Error outcomes keep static text and reveal
no identity, provider errors, code, state or credentials. Use no third-party
assets, script, analytics or caller-selected links. Set `Referrer-Policy:
no-referrer`, `Cache-Control: no-store`, and a CSP restricting all resources,
frames and form actions to the necessary first-party form; forbid embedding.
Redact authorization/cookie headers, callback and launch queries, form secrets,
and raw deep links from application, ingress, tracing and error sinks. Prevent
new leakage and use clean result navigation; do not claim to erase a callback
URL already retained in browser history.

### Native recovery, lost responses and cleanup

The local pending record contains environment, fixed origin, verifier, server
handle, bound client profile, purpose, timestamps and selection state, with no GitHub tokens or
history keys. Persist before opening the browser and before any claim. Do not
store the launch URL durably or expose proof-bearing requests to frontend code.
Never use the history key's `load_or_create` behavior to recover a missing proof.

Deduplicate startup and running-instance events by attempt. First show the
matching local record; use its authenticated status once on receipt, restart
or an explicit retry action. There is no background polling loop. `verified`
opens the comparison view only. `ready` may claim only after local selection
and any session-switch confirmation; an unknown, malformed, expired or other-
environment URI never starts an attempt. Reject raw URI text over 256 bytes and
encoded alternate spellings of the fixed grammar before dispatch.

Record a durable claim-in-flight marker before sending. Serialize claim per
attempt and never automatically resend it after an ambiguous response. If login
credentials arrive, validate the response and securely store the refresh token
with the issuing attempt ID before adopting the session. If reception, validation
or secure storage fails, require a new login; the attempt cannot reissue secrets.
Use the still-valid original proof for best-effort `revoke-issued-session`.
This is an explicit, narrow extension to ADR 0009's revocation authority, not
an alternative refresh-token logout or changed cancellation semantics. It
requires `purpose=login`, current generation and the proof's hard deadline.
Atomically prevent future issuance if claim has not committed, or revoke only
the immutable attempt-to-session association if it has. Thus an acknowledgement
cannot race with a late successful issuance. No access, renewal, or other-session
authority follows from it. It never revokes an old session preserved during a
failed account switch. Retain the proof hash and issuance link until the hard
deadline even if the client has already erased its copy.

Before writing a received refresh credential, durably record its returned
session ID in the pending record as a nonsecret receipt. This makes the exact
session entry discoverable after a crash between OS storage and index adoption.
Do not report successful adoption until both credential storage and the selected
session record are durable. A claim-in-flight marker without a saved receipt
and valid credential record never authorizes another claim, even if status still
shows `ready`: use the atomic revocation/abandonment barrier and start again.

On restart, a securely saved application-session record linked to the attempt
establishes whether credentials were already saved before a crash. Validate
that session through ADR 0009 before adoption. Without such a record, a consumed
login is unrecoverable and may only be revoked as above. A consumed reauthentication
can be checked through the original session's inspection response; this reveals
no lost new credentials because none were issued. Status retries do not claim.

After success, cancellation, failure or expiry, delete pending secrets. If OS
deletion fails, durably mark the attempt terminal and never load it for claim
again; retry deletion next run. If neither erasure nor a terminal marker can
be persisted, report cleanup incomplete, stop new authentication operations in
that environment, and require storage recovery. Expiry is enforced before use
even when physical deletion is delayed by a stopped app or unavailable store.
Do not erase history keys or unrelated attempts. Stop using proof at the hard
deadline. Erase recoverable PKCE material at terminal provider processing, and
make all transient D1 secret/hash material unusable at its deadline. Purge it
and minimal attempt tombstones from the primary within twenty-four hours of
the hard deadline; do not assume D1 provides automatic per-record TTL deletion.
Keep session replay/revocation metadata for ADR 0009's longer lifetime. Database
restore still requires ADR 0008's external generation change.

### Acceptance specifications and delivery boundary

- Verify generated registration, validation and server profiles agree; reject unknown/disabled profiles and preserve in-flight profile bindings.
- Test exact release/development grammar, authority/fragment/encoding rejection,
  length limits, unknown attempts and wrong-environment records.
- Race browser bootstrap, callback, confirmation, cancellation and claim; assert
  one winner and no dependent write from a losing condition.
- Reject missing/swapped/evicted cookies and wrong Origin/CSRF without consuming
  another attempt. A leaked URI, comparison code, launch ticket or submitted
  challenge never suffices as the original claim verifier.
- Test callback admission at 599/600 seconds, processing at its thirty-second
  boundary, and claim exactly at verified plus 300 seconds or creation plus 930.
- Test reverse-order parallel completion, explicit selection and account switch,
  preserving an old session if new secure storage fails.
- Reauthentication rejects another GitHub ID, revoked targets and pre-global-
  logout anonymous attempts; it neither issues tokens nor extends absolute expiry.
- Inject create, cookie, confirmation and claim response loss, process crashes,
  OS store errors and deletion failures; never rebind or reissue secrets.
- Verify proof-scoped issued-session revocation cannot target an existing or
  unrelated session, and fails after its own hard deadline/generation change.
- On Windows and macOS, test bundled release and development side by side,
  cold/warm delivery, duplicate events, browser confirmation, preview and return,
  restart recovery, data/credential/autostart isolation and no secret-bearing logs.

These are implementation acceptance specifications, not executed runtime tests.
#199 owns the protocol implementation, #200 the official plugin/OS integration,
#201 native credential lifecycle, #202 operational/abuse gates, and #203 physical
platform acceptance. #193 stays open until its children are delivered and the
combined contract, including origin-resolution ownership, is reconciled.

Single-slash syntax follows RFC 8252. Tauri's runtime URL parsing is compatible
by source inspection, while its guide primarily illustrates double-slash URLs.
Neither source review nor scheme registration proves installed OS/browser delivery;
acceptance must test the exact selected shape on both Windows and macOS.

## Integrated design and implementation boundary (#193)

Provider identity is governed by ADR 0005, primary consistency by ADR 0008,
session lifecycle by ADR 0009, and the explicit refinements in this ADR take
precedence for browser pages, session inspection/reverification, attempt-scoped
revocation, configurable callback profiles and self-hosted connections.
No new product decisions remain in #193. The following are implementation and
acceptance obligations, not established runtime evidence:

| Issue | Delivery boundary |
| --- | --- |
| #194 | Rust Worker foundation, deploy button/template, official DNS/HTTPS and operator GitHub App setup checks. |
| #195 | D1 schema, guarded transactions, races and account identity uniqueness. |
| #196 | GitHub provider calls and callback processing. |
| #197 | Session rotation, freshness and revocation. |
| #199 | Attempt, browser binding, confirmation and handoff APIs. |
| #200 | Generated URI profiles and official Tauri/OS integration. |
| #201 | Connection selection, credential storage and restart/lost-response recovery. |
| #202 | Abuse limits, safe logs, restore generation and operational gates. |
| #203 | Physical Windows/macOS and real provider/deployment acceptance. |

Deployment does not imply credential registration or HTTPS acceptance; local
celld does not establish Cloudflare concurrency, restore or OS delivery behavior.
Fork configuration remains supported, and no issue is completed merely by this
implementation mapping. This integration resolves #193's design scope only.

<details>
<summary>日本語</summary>

provider identityはADR 0005、primary整合性は0008、sessionは0009に従う。
画面、inspection/再本人確認、試行限定失効、callback profile、セルフホストは本ADRの明示追加を優先する。
#193に新たなproduct判断は残っていない。上表の担当は実装・受け入れ義務であり、実行成功の証拠ではない。
配置だけで登録・HTTPS受け入れ済みとせず、local celldだけでCloudflare競合・復元・OS配送を確認したとしない。
fork設定を維持し、この対応表だけで実装issueを完了しない。今回解決するのは#193の設計範囲だけ。

</details>

## References

- [Native application URI schemes](https://www.rfc-editor.org/rfc/rfc8252.html#section-7.1)
- [Tauri deep linking](https://v2.tauri.app/plugin/deep-linking/)
- [Tauri single-instance](https://v2.tauri.app/plugin/single-instance/)
- [Tauri opener](https://v2.tauri.app/plugin/opener/)
- [Browser transaction binding](https://www.rfc-editor.org/rfc/rfc9700.html#section-4.7.1)
- [Cookie attributes and prefixes](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie)
- [Cookie scope and expiry](https://www.rfc-editor.org/rfc/rfc6265.html)
- [Tauri deep-link runtime implementation](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/deep-link/src/lib.rs)

<details>
<summary>日本語</summary>

## 決定と境界

GitHubは引き続きCloudflareの固定HTTPS callbackへ戻し、その後に専用URIでdesktopへ復帰する。
URIには試行handleだけを載せ、provider/app token・identity・秘密verifierを載せない。
受信だけでは認証せず、開始元desktopの試行別verifierで一回限りのHTTPS claimを行う。
未知のURIから試行やverifierを作らず、生URIや起動引数をlogへ出さない。

callbackの契約形式は`<configured-scheme>:<configured-absolute-path>?attempt_id=<opaque-handle>`。
山括弧部分は設定値で置き換え、pathは`/`で始める。`io.github`や特定のaccount名は仕様上の必須値ではなく、
GitHub Pagesへの配置も不要。具体的なURIを本文からコピーせず、build/配置profileを登録・検証・生成に使う。

### 公式ビルドの既定値

以下は公式配布の既定値だけを示す。forkではprofile定義でschemeとpathを変更できる。

| 環境 | 既定scheme | 既定絶対path |
| --- | --- | --- |
| 配布版 | `win.9renpoto.time-wise` | `/auth/complete` |
| 開発版 | `win.9renpoto.time-wise.dev` | `/auth/complete` |

公式配置は管理下の9renpoto.win配下に割り当てる。

| 環境 | 認証origin | GitHub provider callback |
| --- | --- | --- |
| 配布版 | `https://auth.time-wise.9renpoto.win` | `https://auth.time-wise.9renpoto.win/auth/github/callback` |
| 開発版 | `https://auth-dev.time-wise.9renpoto.win` | `https://auth-dev.time-wise.9renpoto.win/auth/github/callback` |

専用schemeは逆domainのwin.9renpoto namespaceを使う。これらは公式配置の割当値であり、
forkや別のセルフホストへ強制しない。既存の配布bundle ID・履歴保存identityは変更しない。
DNS、Worker custom domain、TLS、GitHub App登録と疎通の確認は#194で実施する。
このADRの記載だけで配置・到達確認済みとは扱わない。


上記は公式profileの既定値とし、文字列をnative検証・server画面・packageへ重複記述しない。
profile ID・環境・登録scheme・固定復帰pathを一つのversion付き非秘密定義に集約し、
Tauri登録・native検証定数・server catalogを生成してbuild/配置時に一致を検証する。
公式IDはofficial-release-v1とofficial-development-v1。配置は有効にするprofileを明示許可する。
独自buildは管理下のschemeへ定義を変更して再buildできるが、server URL設定だけでは登録schemeを変えない。
forkしたアプリでは、公式profile ID・namespace・/auth/completeに制限せず、ビルド時に独自の
desktop callback URI（schemeとpath）を自由に指定できる。対応するprivate-use形式として
authority・user情報・port・query・fragmentを含まないことを検証し、復帰時にattempt_idだけを追加する。
指定値からpackage登録とnativeの厳密な検証を生成する。実行時に任意の戻り先を受理する意味ではない。
運用者は対応するversion付きprofileをserverの許可設定へ登録してからloginする。
ここでのcallbackはbrowserからアプリへの専用URIであり、GitHubからserverへ戻るHTTPSの
/auth/github/callbackとは別物。後者は配置先originとGitHub App登録に従う。
試行作成で既知かつserverが許可するclient_profile_idを選び、不明・無効・別環境は作成前に拒否する。
選択profileとversion付きの値を試行に固定し、再起動・設定変更で復帰先を変えない。
任意scheme・URI・path・originを要求で受け取らず、受信URIから新しいschemeを学習しない。

HANDLEはserver発行の43文字unpadded base64url。大小文字を区別し、authority・host・user情報・port・
fragment・別path・重複/余分なquery・別encoding・別環境scheme・256byte超を拒否する。
別形式を正規化して受理しない。Windows/macOSで正確な形式を実機検証する。

開発版overlayは既存配布bundle IDに`.dev`を付け、product/autostart名をtime-wise-dev、
scheme配列を開発版だけにする。single-instance、認証origin、data directoryも分離する。
認証credential serviceは配布版time-wise.auth.release.v1、開発版time-wise.auth.development.v1。
entryは正規HTTPS originのSHA-256 namespace配下のattempt/HANDLEとsession/SESSION_ID。非秘密indexにはhandle・期限・終了flag・選択sessionを保存し、
verifier・token・起動ticketを含めない。開発版の履歴鍵namespaceも独立させる。
配布版の既存bundle ID・履歴鍵entryは維持し、既存履歴を移動しない。
overlayだけでは固定実装のcredential serviceやautostart名は自動で変わらない。

再起動後も期限内なら元の試行を再開できる。browser起動前に端末限定OS storeへverifierと
試行・環境・期限情報を保存し、serverでも期限を独立検証する。provider token・履歴鍵を含めず、
欠落proofを再生成したり平文保存へ退避したりしない。停止中のOS storeには自動TTLがない。

browserと元desktopに同じ短いcodeを表示する。利用者は一致を確認し、browserで照合確認を送信してから
復帰buttonを押す。serverは同一origin POST、開始browserへの関連付け、CSRFを検証してからclaimを許可する。
閉じたアプリをcode表示用preview URIで開く操作では、引き渡し消費・確認省略を許可しない。
codeはGitHub認可code・claim verifier・履歴復号の端末承認とは別物で、認証資格情報として使わない。

callback受付は作成から10分、照合・確認・claimを合わせてGitHub本人確認成功から5分とする。
期限ちょうど以降は拒否し、再起動・URI再送・確認で延長しない。

### 並行試行と再本人確認

ローカルで選択した試行だけをclaimする。別試行のURIは対応するcode表示だけとし、
自動選択・session置換をしない。選び直しと既存sessionからの切替を明示確認し、
新しいrefreshの安全な保存までは旧sessionを維持する。成功後は旧sessionのlogoutを試みて資格情報を破棄する。
remote失効未確認ならその旨を表示し、隠れた再送資格情報を保持しない。別試行は取り消さず、
履歴鍵・復号権限・既存履歴の所有先を変更せず、新accountへ既存履歴を自動送信しない。

再本人確認は有効accessで開始し、serverで元session/accountに固定する。bodyのaccount IDを受け付けず、
別GitHub IDでは失敗させ、loginへ自動変換しない。claimは元の有効sessionへ確認時刻を原子的に記録し、
sessionや資格情報を新規発行しない。元provider_verified_at_msと30日期限は変えず、inspectionに
nullableなreauthenticated_at_msを追加する。全体logoutの5分の鮮度は同sessionの初回本人確認または
最新の再本人確認から判定し、別sessionやrefreshから引き継がない。

### 公式Tauri plugin

公式deep-link pluginのget_current()とon_open_url()、既存single-instanceのdeep-link feature、
既存openerを導入経路とする。single-instanceは先頭を維持し、受信・検証はRustで行う。
frontendへscheme登録権限を加える必要はない。macOSは実行時登録不可で、公式手順では
/Applicationsのbundleを使う。cargo tauri devだけではOS配送を確認できない。
Windowsの開発時登録は開発schemeだけに限定する。source確認は実機受け入れの証明ではない。

## 詳細protocol

### セルフホストの前提

Deploy to Cloudflare buttonと再現可能な手順で、運用者自身のCloudflareへ配置できるようにする。
全利用者共通の認証hostnameは前提にせず、AUTH_ORIGINは配置単位で固定する。
従来の単一サービス前提を置き換え、以下のprotocolも接続先選択・配置identityの契約に合わせて再整理する。

配置ごとにWorker・D1・GitHub App・callback URL・秘密情報・復元generationを管理する。
buttonはsetupの入口であり、GitHub登録・migration・秘密生成・HTTPS受け入れの完了を意味しない。
serverと配置検証が整ってから実際に動くbuttonを公開し、仮のbuttonは追加しない。

公式desktopの設定でHTTPS接続先を明示選択する。deep linkで接続先を変更せず、
account・session・保留記録を配置identityと開発/配布環境で分離する。
同じGitHub数値IDでも別配置では別アプリアカウントで、接続先変更により資格情報・履歴所有権・
復号権限を転送しない。login前にprotocolを検証し、保留試行は開始時のoriginに固定する。
このdesktop配布方式は合意済み。環境別の設定に接続先を保存し、明示的な設定操作でだけ変更する。
変更時は旧接続先での認証済み処理を止め、旧token・verifier・履歴を新接続先へ送らない。
保留試行は開始originに固定し、新選択先へ完了させない。元接続先の表示はできるが、
再開にはその接続先を明示選択し直す。HTTPS originと実効portを接続namespaceに使い、
別名発見・資格情報の自動移行はしない。user情報・query・fragment・root以外のpathを拒否する。
native認証要求で別originへのredirectを追わない。browser認証開始前に正確な接続先を表示する。
接続先設定で固定の開発/配布URI schemeを変えず、既存履歴の復号・upload権限も与えない。
handleが複数接続先に一致する場合は曖昧として拒否し、URIからserverを選ばない。
GET /v1/auth/configを資格情報なしで取得し、protocol_version: 1、有効client_profile_ids、auth_originの厳密一致を
login前に検査する。これは互換性確認であり運用者への信頼の証明ではない。
非対応serverへ他接続先の資格情報を送らない。

上記方針と以下のAPI・処理時間・回復手順を含む全体契約は合意済み。
origin値は配置運用者が選び、共通hostnameは要求しない。#194でdeploy template・origin設定・
GitHub登録を検証し、完了まで実provider通信を有効にしない。

### browser関連付けと期限

1. Rustで独立した32byte乱数のclaim verifierを作りOS storeに保存し、ASCII verifierのSHA-256を
   unpadded base64urlにしたchallengeだけを試行作成へ送る。返ったhandleとの関連を保存後に
   固定originの起動URLを開く。作成応答喪失は期限で失効する孤立試行として扱い、推測で回復しない。
2. serverの起動ticketは一回限りで、最初の有効な消費でserver生成のbrowser秘密情報へ原子的に関連付ける。
   試行別名の__Host- cookieにSecure・HttpOnly・SameSite=Lax・Path=/を付け、Domainを指定せず
   固定GitHub endpointへ303で遷移する。Max-Ageは残り絶対期限以内で延長せず、
   終了後のbrowser応答で対応cookieを削除する。cookie応答喪失・別browserではやり直し、付け替えない。
   後続URL・logにticketを残さず、cookieやhashを並行試行で共有しない。
3. GET callbackはstateと対応cookieを確認してから一回限りの処理を確保する。cookie欠落・不一致では
   他試行を消費せず拒否する。期限・generationをserverで確認し、browser cookieでnative APIを認証しない。
4. 本人確認後は同browserでcodeと確認formを表示する。固定Origin、cookie、CSRF nonce、期限と
   verified/ready状態を原子的に検証する。有効な確認の再送ではready画面を再表示できるが延長・再発行しない。
5. callback確保から交換・本人確認・cleanup・公開まで合計30秒、外部通信は残り時間内の最大10秒とする。
   交換を自動再試行せずcleanup時間を確保する。cleanup失敗だけでは本人確認を戻さないが、
   合計期限後の公開は拒否する。引き渡しは本人確認から300秒、作成からの絶対上限は930秒。
   遅い結果や中断済みcallbackを再利用しない。native保留記録にも同じ上限を適用する。

handle・ticket・cookie秘密情報・CSRF nonce・verifierは独立した32byte以上の暗号学的乱数。
handle/verifierは43文字unpadded base64url。照合codeは英語記載の紛らわしくない32文字alphabetから
10文字を生成し、5文字ずつ表示する。比較用には用途別hashだけを保存する。
native環境・browser環境でそれぞれ同時4試行までとし、超過時は作成/ticket消費前に拒否して
別試行を追い出さない。rate limitは一回操作前に判定し429とRetry-Afterを返す。
#202の追加abuse対策も本契約の保証を弱めない。これらの時間・件数はRFC指定値ではなくアプリ方針。

### 状態と原子性

状態はcreated（起動前）、pending（browser関連付け済み）、processing（callback確保済み）、
verified（本人確認/account公開済み）、ready（browser確認済み）、consumed（引き渡し消費済み）。
failed・cancelled・expiredも終了状態で、終了した試行を再開しない。cookie応答喪失・callback中断は
新しい試行にする。確認応答喪失では同じbrowserのcleanな結果画面を再表示できる。

全従属書込みにADR 0008のguardを適用する。login公開はaccountと待機handoffだけを作り、sessionは
claimまで発行しない。再本人確認は既存targetの関連付けを検証し、不一致で別accountを作成・付け替えない。確認・claimで期限、環境generation、account認可generation、
ADR 0009の全体logout前の試行cutoffを検査する。login消費とsession発行、再本人確認消費と
元sessionの時刻更新をそれぞれ原子的に行う。取消が先なら後続公開・発行を拒否する。
消費後の取消は409とし、account関連付けを戻さず、発行済みsessionは別の失効操作で扱う。

### APIと画面の例外

環境別の固定AUTH_ORIGINを必須にし、HTTPS origin以外のpath・query・fragment・user情報を許可しない。
要求headerや引数で決めない。D1・Worker秘密情報・GitHub App・復帰schemeも環境に固定する。
cookieは別hostnameで分離しport違いだけに依存しない。既定本番hostやlocalhostへの暗黙fallbackを置かず、
celld HTTPは合成data/provider専用にする。実GitHub通信前にorigin値・登録・HTTPS疎通を確定する。

nativeは固定originへRustからJSONを送りno-storeを使う。試行操作のBearerはその試行のclaim verifier。
browser cookieや寛容なCORSでnative APIを認証せず、要求でaccountや復帰URLを指定しない。

| Endpoint | 要求・結果 |
| --- | --- |
| POST /v1/auth/attempts | purpose・claim_challenge・client_profile_id。loginはsession不要、reauthenticateは有効accessが必要。201でdescriptor。 |
| GET /v1/auth/attempts/{id} | verifierで200。状態・code・purpose・期限。起動URL・identity・session秘密情報を返さない。 |
| POST /v1/auth/attempts/{id}/claim | verifierと空JSON。200でlogin資格情報または再本人確認のresult/verified_at_ms。 |
| POST /v1/auth/attempts/{id}/cancel | verifierと空JSON。204、既消費は409。 |
| POST /v1/auth/attempts/{id}/revoke-issued-session | verifierと空JSON。204で将来の発行を原子的に防ぐか、この試行が発行したsessionだけを失効。 |
| GET /auth/github/start?ticket=VALUE | 一回ticketを消費しcookie関連付け、303でGitHubへ。 |
| GET /auth/github/callback | ADR 0005のstatusと、再本人確認の別identityに409 identity_mismatch。成功時はcode/確認form。 |
| GET /auth/github/attempts/{id} | 対応cookieで200の結果/照合画面。 |
| POST /auth/github/attempts/{id}/confirm | cookie・厳密Origin・csrf_tokenで確認し、303で同一origin結果画面へ。readyなら復帰link。 |

正確なfield名と合成JSONは英語本文を参照する。descriptorはhandle・purpose・code・起動URL・作成時刻・
600秒受付期限・930秒絶対期限。statusでは起動URLを省き、stateとnullableなhandoff_expires_at_msを加える。
時刻はserver UTCミリ秒の絶対値。不明IDと不正verifierは同じ404 attempt_unavailable。
正当なproofには400 invalid_request、409 attempt_not_ready/attempt_consumed、
410 attempt_expired/attempt_cancelled、401 invalid_session、429 rate_limited、
503 authentication_unavailableを返す。別identityはfailedにし一般化したidentity_mismatchだけを返す。
資格情報・別identityを開示しない。通信断は一回操作の再送を正当化しない。
状態競合errorは変更APIに適用し、正当なproofの状態取得は絶対期限内なら終了状態も200で返す。

ADR 0005の画面規則を限定改訂し、first-party結果link、code、CSRF確認form、固定preview/復帰URIを許可する。
callbackは再本人確認用409 identity_mismatchだけを明示追加し、結果再表示でcallbackを再処理しない。provider code・state・token・identity・
provider errorを表示せず、外部asset・script・analytics・任意linkを使わない。no-referrer、no-store、
必要な同一origin formだけを許可するCSPと埋込み禁止を設定し、header・query・form秘密情報・生URIを
全log経路で秘匿する。cleanな結果画面への遷移でもbrowser履歴の完全消去は保証しない。

### native回復と削除

保留記録は環境・固定origin・verifier・handle・固定profile・purpose・期限・選択状態をOS storeへ保存する。
GitHub token・履歴鍵・起動URLを永続化せず、秘密APIをfrontendへ渡さない。欠落proofを履歴鍵の
load_or_createで再生成しない。起動時/既存instanceのURIを試行ごとに重複抑止し、受信・再起動・手動再確認時に
だけ状態を取得する。背景pollingはしない。verifiedはcode表示だけ、readyは選択・切替確認後にclaimする。

送信前にclaim中markerを永続化し、成否不明のclaimを自動再送しない。受信資格情報を検証し、
発行元attempt ID付きでrefreshを安全に保存してからsessionを採用する。失敗ならGitHubでやり直し、
元proofでrevoke-issued-sessionをbest effort実行する。これはADR 0009への明示的・限定的な失効権限追加で、
取消rollbackやrefresh logoutの別名ではない。login purpose、現generation、proof絶対期限を検査する。
未発行の阻止または不変な試行→session関連だけの失効を原子的に行い、失効応答後に遅いclaimが成功しない。
access・更新・他sessionの権限を与えず、切替前の旧sessionも守る。serverはproof hashと発行関連を絶対期限まで保持する。

refresh保存前に返ったsession IDを非秘密の受領記録として保留記録へ永続化し、OS保存とindex採用の
間に停止しても対応entryを発見できるようにする。資格情報と選択sessionがともに永続化するまで成功にしない。
claim中markerだけがあり受領記録と有効な資格情報がない場合は、statusがreadyでも再claimせず、
原子的な発行阻止/失効を実行して新しい試行にする。

再起動時に安全に保存済みのsessionとattempt IDがあればADR 0009のinspectionで確認する。
保存記録なしの消費済みloginは回復せず限定失効だけを行う。再本人確認は新資格情報を発行しないため、
元sessionのinspectionで確認時刻を調べられる。状態取得の再試行ではclaimしない。

成功・失敗・取消・期限で保留秘密情報を削除する。OS削除失敗なら終了markerを永続化し、
再claimせず次回起動で削除を再試行する。削除もmarker保存も失敗したらcleanup未完了と表示し、
環境の新規認証を止めてstore回復を要求する。物理削除が遅れても期限後は使わない。
他試行や履歴鍵は削除しない。復号可能なPKCE情報はprovider処理終了で消去し、D1一時hash/秘密情報は期限で利用不能にする。
絶対期限から24時間以内に最小終了記録とともにprimaryから削除し、D1の自動TTLは仮定しない。sessionの長い再利用/失効記録はADR 0009、復元はADR 0008の外部generation変更に従う。

### 受け入れ仕様と担当境界

- 生成した登録・検証・server profileの一致、不明/無効profileの拒否と試行の固定先維持を検証する。
- URI grammar・長さ・encoding・別環境・不明試行を検証する。
- 起動、callback、確認、取消、claimの競合で一回性と敗者の従属書込みがないことを検証する。
- cookie欠落・入替・Origin/CSRF不正で他試行を消費せず、URI/code/ticket/送信済みchallengeだけでは元verifierとして認証できないことを検証する。
- callback 599/600秒、処理30秒、本人確認から300秒、開始から930秒の境界を検証する。
- 逆順完了・明示選択・切替、保存失敗時の旧session維持を検証する。
- 別GitHub ID・失効済みtarget・全体logout前試行の拒否と、再本人確認で無発行・絶対期限不変を検証する。
- 応答喪失・process中断・OS store障害で付け替え・再発行しないことを検証する。
- proof限定失効が旧/別sessionへ影響せず、期限・generationを超えないことを検証する。
- Windows/macOSで配布・開発の併用、新規/既存起動、重複event、preview/照合/復帰、再起動、
  data/credential/autostart分離と秘密logがないことを実機確認する。

以上は未実行の受け入れ仕様。#199はprotocol、#200はplugin/OS、#201はnative資格情報、
#202は運用/abuse、#203は実機を担当する。全子契約の提供とorigin確定担当を含む全体整合まで#193をopenにする。

single slashはRFC 8252に沿い、Tauri実装のURL解析でも扱える構造だが、公式guideの例は主にdouble slash。
source確認やscheme登録だけで配送成功とはせず、正確な形式をWindows/macOSで実機検証する。

</details>
