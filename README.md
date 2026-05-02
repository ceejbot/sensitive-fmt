# sensitive-fmt

[![Tests](https://github.com/ceejbot/sensitive-fmt/actions/workflows/test.yaml/badge.svg)](https://github.com/ceejbot/sensitive-fmt/actions/workflows/test.yaml) [![audit-dependencies](https://github.com/ceejbot/sensitive-fmt/actions/workflows/audit.yaml/badge.svg)](https://github.com/ceejbot/sensitive-fmt/actions/workflows/audit.yaml)

Derive `Debug` and `Display` for Rust structs while honoring sensitive field annotations. Designed for one job: keeping
PHI, PII, secrets, and tokens out of log lines.

I wrote this because none of the crates I could find (including my own [safe-debug](https://github.com/ceejbot/safe-debug) ) did exactly what I wanted as simply as I wanted to do it. This sensitive data redactor:

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

| Attribute                    | Effect                                                                                 | required trait                                                 |
| ---------------------------- | -------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| _(none)_                     | normal `Debug` / `Display`                                                             | `Debug` for `SensitiveDebug`, `Display` for `SensitiveDisplay` |
| `#[sensitive(redact)]`       | writes `REDACTED`                                                                      | none                                                           |
| `#[sensitive(truncate = N)]` | writes `****<last N code points>`, or `REDACTED` if value has fewer than N code points | `Display` (in both derives)                                    |
| `#[sensitive(skip)]`         | writes `<skipped>`                                                                     | none                                                           |

## Scope

- Tuples fail at compile time with a clear message. Wrap the data in a named-field struct or enum instead.
- Container types (`Option<T>`, `Vec<T>`, etc.) are treated as opaque. If you have an `Option<String>` field you can't
  `Display`-format directly, reach for `redact` or `skip`.
- Generic structs are supported by adding trait bounds on the actual formatted field type. Plain fields require
  `Debug` for `SensitiveDebug` or `Display` for `SensitiveDisplay`; `truncate` always requires `Display`; `redact` and
  `skip` do not add formatting bounds.
- `no_std` compatible. Requires `alloc` for the `truncate` modifier.
- Minimum supported Rust version: 1.88.0.
- Truncate counts code points, not graphemes. `"café"` is 4 code points.

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
