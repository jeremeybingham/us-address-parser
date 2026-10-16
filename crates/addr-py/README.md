# us-address-parser (Python)

Parse US addresses into components, with USPS Publication 28 and RESO
output, a match key, a URL-safe slug, and diagnostics. A Rust library with
Python bindings (PyO3, abi3 wheels for Python 3.10 and later); no runtime
dependencies and no data to download.

This library formats and parses; it **does not validate** that an address
exists or can receive mail.

```python
import us_address_parser as uap

a = uap.parse("123 north main street apt 4, Miami FL 33101")
a.street_name          # "MAIN"
a.unit_number          # "4"
a.place                # {"name": "MIAMI", "state": "FL", "county": "DADE", ...}
a.warnings             # []
a.to_usps()["line1"]   # "123 N MAIN ST APT 4"
a.key()                # "k1:S|123|N|MAIN|ST||FL|33101"
a.slug()               # "123-n-main-st-apt-4-miami-fl-33101"
a.to_reso()            # RESO Data Dictionary 2.0 fields
a.to_dict()            # everything above as plain Python values

uap.parse_fields(line1="THE COVE", line2="1111 ASBURY WAY",
                 city="Orlando", state="FL", zip="32801")
uap.parse_many(["1 Main St, Miami FL", "PO Box 5, Tampa FL"])  # GIL released
uap.parse("281 EVELYN DR DOTHAN", default_state="AL").city     # "DOTHAN"
```

- Unusable input (no address text at all) raises `ParseError`, a
  `ValueError`; in `parse_many` that element is `None`. Anything else comes
  back with `warnings`.
- `default_state` is a lookup hint for input with no state; it never fills
  `state`. A value that is not a US state, DC or territory raises
  `ValueError`.
- `confidence` is `None` until it is calibrated.

## Coming from usaddress

| usaddress | `us_address_parser` |
|---|---|
| `usaddress.parse(s)` | `parse_tokens(s)`: `[(token, label)]` |
| `usaddress.tag(s)` | `tag(s)`: `(dict, "Street Address" \| "PO Box" \| "Ambiguous")` |
| `RepeatedLabelError` | not raised: the runs are joined and `parse(s).repeated_labels()` lists the labels |
| (none) | `parse(s).address_type()`: `street`, `po_box`, `rural_route`, `military`, `general_delivery`, `dual`, `unknown` |
| (none) | `parse(s, default_state="FL")`: a state hint for input with no state |

The differences (tokens that cannot be placed are left out of `tag()`, the
labels this library does not model) are in the repository README, "Coming from
usaddress".

## Build from source

```sh
pip install maturin
maturin develop      # in this directory; builds the extension into your venv
pytest tests
```
