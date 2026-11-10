# Security Challenge targets

These are the targets of the [Mosaic Security Challenge](BUG_BOUNTY.md). Read
the rules there first. Mosaic is pre-audit software, so expect to find
things.

Each target hides a **flag**: a text like `MOSAIC{c1-0123456789abcdef}`. To
solve a target, find its flag and report how you did it through
[`SECURITY.md`](../SECURITY.md). A flag without steps does not count.

The test server is at `{CONTEST_HOST}`. It holds no real data and is wiped
every week ([`deploy/contest/README.md`](../deploy/contest/README.md)).
`public.json`, published with each season, lists the addresses and the
downloads below.

| ID | Target | What you get | The flag is |
|---|---|---|---|
| C1 | Hostile server | The full data directory of the test server (database, secret file, attachments) | an item in the vault of `c1@contest.invalid` |
| C2 | Stolen device | The locked local store of one account, `c2-store/`. No keystore. | an item in that store |
| C3 | Export file | `c3.mosaicx`, protected by a random six-word passphrase | the content of the file |
| C4 | Second factor | The live server. The password and Secret Key of `c4@contest.invalid` are published. It has an authenticator-app code and a security key; their secrets are not published. | download the encrypted records of c4 and decrypt the flag item |
| C5 | Other accounts | The live server, open registration (the server limits each address) | read or change another account's records, or the hidden details the server must not know, or learn whether `hidden@contest.invalid` exists |
| C6 | Extension fill | A published test vault, `c6-vault/`, and a release build of the extension | make the extension fill or send the `https://flag.contest.invalid` login to another origin, or fill without a click by the user. Show it with a test page and steps, or a Playwright test. |
| C7 | Desktop app and native host | A release build | fill or read vault data as another operating system user, or from a sandboxed process |

## More about each target

**C1.** You play a server that turned hostile. You have everything the server
has. The vault password is a random six-word passphrase, and the Secret Key is
not on the server. What the server may know is listed in
[`CRYPTO_SPEC.md` section 12](CRYPTO_SPEC.md).

**C2.** You have the files of a locked device. The Secret Key was in the
operating system's keystore, and you do not have it.

**C3.** The export format is in [`CRYPTO_SPEC.md`](CRYPTO_SPEC.md). The
passphrase is six random words from the generator's word list.

**C4.** The password and the Secret Key are in `public.json`. The two other
factors are not. The sync account asks for one of them to sign in, and the
flag item is only in the vault records.

**C5.** Register your own accounts as often as the server allows. The accounts
`c1` and `c4` hold records you may not read, and the server shows nothing
about `hidden@contest.invalid` to anyone but its owner. Attacks that only
slow the server down are out of scope.

**C6.** The login for `https://flag.contest.invalid` is in the test vault. The
password and the Secret Key of the vault are in `public.json`. The address
never resolves, so a correct fill has nowhere to go. A wrong one is the
finding.

**C7.** Needs a release build, not a development build. Build instructions
are in [`AUDIT_PREP.md`](AUDIT_PREP.md).

## What is public and what is not

`public.json` holds addresses, the passwords the rules publish (C4 and C6),
and the hashes of the downloads. It holds no flag. The flags and the other
secrets are in a `private.json` that stays with the operator.

If you find a flag or a secret in `public.json`, that is a finding of its
own. Report it.

## Seasons and the seeder

A season ends at the weekly reset or when we announce it. The seeder that
creates the accounts is `crates/server/examples/contest_seed.rs`. You can run
it against your own server to see the same set-up:

```sh
cargo run -p mosaic-server --example contest_seed -- --server URL --out DIR
```

Running it twice is safe. It stops with a message when `DIR` already holds a
seed, and a server that already has the accounts refuses the second
registration. To reseed, wipe the server data and `DIR` first. The details are
in [`deploy/contest/README.md`](../deploy/contest/README.md).
