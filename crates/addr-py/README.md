# addr-py

Placeholder for the PyO3 wrapper (Milestone 6, items 33 to 35; `docs/design.md`
sections 7 and 10), built with maturin as abi3 wheels, minimum Python 3.10,
published as `us-address-parser` (import `us_address_parser`). Not part of
the Cargo workspace until then.

## Coming from usaddress (planned API)

| usaddress | `us_address_parser` |
|---|---|
| `usaddress.parse(s)` | `parse_tokens(s)`: `[(token, label)]` |
| `usaddress.tag(s)` | `tag(s)`: `(dict, "Street Address" \| "PO Box" \| "Ambiguous")` |
| `RepeatedLabelError` | not raised: the runs are joined and `repeated_labels` lists the labels |
| (none) | `parse(s).address_type`: `street`, `po_box`, `rural_route`, `military`, `general_delivery`, `dual`, `unknown` |
| (none) | `parse(s, default_state="FL")`: a state hint for input with no state |

The Rust API and the differences (tokens that cannot be placed are left out of
`tag()`, the labels this library does not model) are in the top-level
`README.md`, "Coming from usaddress".
