# sensitive-fmt

[![Tests](https://github.com/ceejbot/sensitive-fmt/actions/workflows/test.yaml/badge.svg)](https://github.com/ceejbot/sensitive-fmt/actions/workflows/test.yaml) [![audit-dependencies](https://github.com/ceejbot/sensitive-fmt/actions/workflows/audit.yaml/badge.svg)](https://github.com/ceejbot/sensitive-fmt/actions/workflows/audit.yaml)

Derive `Debug` and `Display` for Rust structs and enums while honoring sensitive field annotations. Designed for one
job: keeping PHI, PII, secrets, and tokens out of log lines.

I wrote this because none of the crates I could find, including my own [safe-debug](https://github.com/ceejbot/safe-debug), did exactly what I wanted as simply as I wanted it done. This one:

- derives both `Debug` and `Display`
- leans on existing implementations of both for non-primitive types, or lets you skip the field
- is as stupid and simple as I could make it

```rust
use sensitive_fmt::{SensitiveDebug, SensitiveDisplay};

#[derive(SensitiveDebug, SensitiveDisplay)]
struct Patient {
    id: u64,
    name: String,
    #[sensitive(truncate = 4)]
    mrn: String,
    #[sensitive(redact)]
    email: String,
    #[sensitive(skip)]
    raw: NotFormattable,
}

struct NotFormattable;

let p = Patient {
    id: 42,
    name: "Alice".into(),
    mrn: "MRN-12345-WXYZ".into(),
    email: "alice@example.com".into(),
    raw: NotFormattable,
};

assert_eq!(
    format!("{p:?}"),
    r#"Patient { id: 42, name: "Alice", mrn: ****WXYZ, email: REDACTED, raw: <skipped> }"#,
);
assert_eq!(
    format!("{p}"),
    "Patient { id: 42, name: Alice, mrn: ****WXYZ, email: REDACTED, raw: <skipped> }",
);
```

## Field attributes

One attribute, three modifiers (mutually exclusive):

| Attribute                    | Effect                                                                                 | Required trait                                                 |
| ---------------------------- | -------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| _(none)_                     | normal `Debug` / `Display`                                                             | `Debug` for `SensitiveDebug`, `Display` for `SensitiveDisplay` |
| `#[sensitive(redact)]`       | writes `REDACTED`                                                                      | none                                                           |
| `#[sensitive(truncate = N)]` | writes `****<last N code points>`, or `REDACTED` if the value has fewer than N         | `Display` (in both derives)                                    |
| `#[sensitive(skip)]`         | writes `<skipped>`                                                                     | none                                                           |

Enums with unit and named-field variants work the same way. Output starts with the variant name, as std's
`#[derive(Debug)]` does:

```rust
use sensitive_fmt::{SensitiveDebug, SensitiveDisplay};

#[derive(SensitiveDebug, SensitiveDisplay)]
enum Event {
    Heartbeat,
    Login {
        user_id: u64,
        #[sensitive(redact)]
        token: String,
    },
}

let e = Event::Login { user_id: 7, token: "sk_live_abc".into() };
assert_eq!(format!("{e:?}"), "Login { user_id: 7, token: REDACTED }");
assert_eq!(format!("{}", Event::Heartbeat), "Heartbeat");
```

## Scope

- The attribute goes on fields only. Placing it on a struct, enum, or variant is a compile error rather than a
  silent no-op.
- Tuple structs and tuple variants fail at compile time with a clear message. Wrap the data in a named-field struct
  or variant instead.
- The derives treat container types (`Option<T>`, `Vec<T>`, and so on) as opaque. An `Option<String>` field has no
  `Display`, so reach for `redact` or `skip`.
- Generic types work. The derive bounds the field type that is actually formatted, not the type parameter: plain
  fields need `Debug` or `Display` to match the derive, `truncate` always needs `Display`, and `redact` and `skip` add
  no bound at all.
- `no_std` compatible. Only `truncate` needs `alloc`.
- Minimum supported Rust version: 1.88.0.
- Truncate counts code points, not graphemes. `"café"` is 4 code points.
- Truncate works on any `Display` type, not just strings. It formats the value first and keeps the tail of that
  text, so `#[sensitive(truncate = 4)]` on a `u64` holding `1234567890` prints `****7890`, and a value that formats
  to fewer than four characters prints `REDACTED`.

## Why not [other crate]?

| Crate                    | Debug   | Display | Truncate                         | Why this crate exists           |
| ------------------------ | ------- | ------- | -------------------------------- | ------------------------------- |
| [`veil`]                 | yes     | no      | head + tail of `*`s              | no Display, no last-N           |
| [`redactable`]           | yes     | yes     | named policies (Token / PII / …) | grew slog/tracing/json features |
| [`secrecy`] / [`redact`] | wrapper | wrapper | none                             | wrapper-type model, not derives |
| [`safe-debug`]           | yes     | no      | n/a                              | requires the `facet` ecosystem  |

[`veil`]: https://crates.io/crates/veil
[`redactable`]: https://crates.io/crates/redactable
[`secrecy`]: https://crates.io/crates/secrecy
[`redact`]: https://crates.io/crates/redact
[`safe-debug`]: https://crates.io/crates/safe-debug

## License

MIT OR Apache-2.0
