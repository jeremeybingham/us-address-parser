# Choosing the name (item 58, decision 42)

**Chosen: `doorplate`** (decision 46). The rest records how.

Checked 2026-10-17. One name for the Rust crates, the Python package and
import, the CLI and the repository. The current `us-address-parser` collides
with the existing `us_address_parser` crate (decision 33's names are dropped).
This library formats and parses; the name must not suggest it validates,
verifies or delivers anything.

## What was checked

- **crates.io and PyPI:** exact-name lookups through the registries' APIs
  (a 404 is free), plus a crates.io search for near names. PyPI treats `-`
  and `_` as the same; crates.io does too.
- **GitHub:** repositories with the name in their name (count, the top few,
  stars, language). The repository will live under the author's account, so
  only search confusion matters.
- **Trademarks and companies:** web searches for the name beside address,
  geocoding and real-estate data. This is not a USPTO search. A clearance
  search (USPTO TESS and a state search) is still the author's step before
  publishing.
- **Domains:** whether `.com`, `.dev` and `.io` resolve in DNS. "Resolves"
  means someone has it (maybe parked); "no DNS" does not prove it is for sale.
- Avoided on purpose: anything containing USPS, RESO or Pub 28 (marks and
  specifications we cite but are not affiliated with), and "valid", "verify",
  "deliver" words.

## Ruled out

| Name | Why |
|---|---|
| `situs` | Free on both registries, and the right word (the legal location of property), but SitusAMC and Situs Group LLC are commercial real estate finance companies with registered SITUS marks, in the space this library serves |
| `curbline` | Free everywhere, but Curbline Properties (a listed real estate company) uses it |
| `pub28` | An existing Rust project (`1npo/pub28`) and the USPS publication number |
| `kerb`, `locus`, `plat`, `signpost`, `threshold`, `frontdoor` | Taken on crates.io and PyPI |
| `frontage`, `stoop`, `mailplate`, `addrx` | Free on crates.io, taken on PyPI (one name must be free on both) |
| `milepost` | Free on both, but several active projects and a company share the name |
| `usaddr`, `usaddrs` | Free, but one letter from `usaddress`, the library people compare us to |

## Options

| Name | crates.io | PyPI | GitHub repos named so | Domains | Risks |
|---|---|---|---|---|---|
| **`doorplate`** | free | free | 30; the top ones have 0 to 2 stars; one is a Rust project with 0 stars | `.com`, `.dev` resolve; `.io` no DNS | No company found in address or geocoding data. Meaning: the plate with a house number |
| **`houseplate`** | free | free | none | `.com` resolves; `.dev`, `.io` no DNS | Cleanest search results; reads as "house plant" to some |
| **`addrkey`** | free | free | none | none resolve | Says what the key is, which undersells the parser; sits next to vendors' "address key" products (Melissa Address Key, Smarty's key articles) |
| **`addrline`** | free | free | none | none resolve | Forgettable; generic enough that someone else will use it |
| **`kerbstone`** | free | free | 9 tiny repos | `.com`, `.io` resolve | Unknown companies of that name; the curb image fits less than a house number |
| **`addrust`** | free | free | not searched | not searched | Ties the name to Rust, but most users will arrive through Python |

## Recommendation

**`doorplate`**, with `houseplate` as the fallback. It is memorable, free on
both registries, and has no company in this space. `houseplate` has the
cleanest search results but is easy to misread.

If `doorplate` is picked, the names would be:

| What | Name |
|---|---|
| Rust core crate | `doorplate` (the library people `use`) |
| CLI crate and binary | `doorplate-cli`, binary `doorplate` (a short alias `addr` can stay as a second binary name) |
| Python package and import | `doorplate` on PyPI; `import doorplate` |
| Pandas and Polars helpers | `doorplate.pandas`, `doorplate.polars` |
| Repository | `doorplate` |
| Key prefix | unchanged (`k1:`); the key format is not tied to the name |

Check the exact names again just before publishing (item 45).

## Rename map (applied by item 45)

Counts are files in the current tree (`git grep -l`) and show the work:

| Old | New | Files |
|---|---|---|
| `us-address-parser` (PyPI name, repo URL) | `<name>` | 14 |
| `us_address_parser` (Python import, `python/` directory) | `<name>` | 19 |
| `us-addr-core`, `us-addr-cli` (planned crate names) | `<name>`, `<name>-cli` | 14 |
| `addr-core`, `addr_core` (crate and Rust path) | `<name>` | 27 / 15 |
| `addr-cli` (crate) | `<name>-cli` | 19 |
| `addr-py` (crate and directory) | `<name>-py` | 14 |
| `addr` (CLI binary) | `<name>` (keep `addr` as an alias if wanted) | docs, CI, examples |
| `addr-fuzz` | `<name>-fuzz` | `fuzz/` |
| `_core` (compiled module) | unchanged: `<name>._core` | |

Also: `pyproject.toml`, the Cargo manifests, `.github/workflows/`, the wheel
names, the README, `crates/addr-py/README.md`, `examples/`, the notebook
text, `docs/`, and the CHANGELOG. Then `git grep` every old spelling.
