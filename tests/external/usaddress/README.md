# usaddress test data (vendored)

Labeled US address test files from [datamade/usaddress](https://github.com/datamade/usaddress),
used to compare this parser with usaddress (`docs/usaddress-comparison.md`).

- Source commit: `aa7699b53a0843fc443f9e87285b88cbd9eaf50a` (2025-08-07).
- License: MIT, copyright 2014 The Atlanta Journal Constitution. The license
  text is in `LICENSE`, copied as published (it repeats one paragraph).
- Files are unchanged copies.

| File | Upstream path | Addresses | What it is |
|---|---|---|---|
| `us50_test_tagged.xml` | `measure_performance/test_data/` | 687 | Real addresses from all 50 states. Labels a whole street as `StreetName`, so use the coarse score. |
| `labeled.xml` | `measure_performance/test_data/` | 146 | Hand-labeled hard cases: PO Boxes, routes, units, highways, recipients. |
| `synthetic_osm_data.xml`, `synthetic_clean_osm_data.xml` | `measure_performance/test_data/` | 4,122 + 4,120 | Built from OpenStreetMap. |
| `simple_address_patterns.xml`, `multi_word_state_addresses.xml` | `measure_performance/test_data/` | 8 + 5 | Smoke tests. |
| `unparseable.csv` | `training/` | 1,322 | Raw strings, no labels. |
| `in_training.txt` | generated here | 6,677 | Test addresses that also appear in usaddress's training files. |

**These are usaddress's own regression tests.** Its `tests/test_labeling.py`
asserts these labels on every release (`us50` with a fuzzy match), and its
comments say new address formats go in both the training and the test data.
So usaddress scores near 100% on them whether or not an address is in its
training files. They show where this parser differs from usaddress; they are
not a blind test of either parser.

`in_training.txt` is regenerated with `python tools/compare/training_overlap.py`
(it downloads the training files at the pinned commit).

The XML format is usaddress's training format: one `<AddressString>` per
address, one child element per token, the element name being the token's
label.
