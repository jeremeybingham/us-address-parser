"""Column extraction shared by the pandas and Polars helpers.

Neither library is imported here. The helpers parse a batch with
`parse_many` and read the chosen columns from the results.
"""
from __future__ import annotations

from collections.abc import Callable, Iterable, Sequence

from ._core import ParsedAddress, parse_many

DEFAULT_COLUMNS: tuple[str, ...] = (
    "street_number",
    "street_dir_prefix",
    "street_name",
    "street_suffix",
    "street_dir_suffix",
    "unit_type",
    "unit_number",
    "city",
    "state",
    "zip5",
    "zip4",
    "address_type",
    "key",
    "slug",
    "warnings",
)

# Columns that are computed, not plain attributes. All values are strings or None.
_DERIVED: dict[str, Callable[[ParsedAddress], str | None]] = {
    "address_type": lambda a: a.address_type(),
    "key": lambda a: a.key(),
    "key_unit": lambda a: a.key(unit=True),
    "slug": lambda a: a.slug(),
    "warnings": lambda a: ",".join(a.warnings),
    "usps_line1": lambda a: a.to_usps()["line1"],
    "usps_line2": lambda a: a.to_usps()["line2"],
    "usps_last_line": lambda a: a.to_usps()["last_line"],
}

# Plain attributes that hold a string or None.
_ATTRIBUTES = frozenset(
    {
        "street_number",
        "street_dir_prefix",
        "street_name",
        "street_suffix",
        "street_dir_suffix",
        "unit_type",
        "unit_number",
        "second_unit_type",
        "second_unit_number",
        "po_box",
        "military_post_office",
        "attention",
        "building",
        "city",
        "state",
        "zip5",
        "zip4",
        "unparsed",
    }
)

AVAILABLE_COLUMNS: frozenset[str] = _ATTRIBUTES | frozenset(_DERIVED)


def check_columns(columns: Sequence[str] | None) -> tuple[str, ...]:
    """Validate a column choice; `None` means the defaults."""
    if columns is None:
        return DEFAULT_COLUMNS
    if isinstance(columns, str):
        raise TypeError("columns must be a list of names, not a single string")
    names = tuple(columns)
    unknown = [c for c in names if c not in AVAILABLE_COLUMNS]
    if unknown:
        raise ValueError(
            f"unknown column(s) {unknown}; available: {sorted(AVAILABLE_COLUMNS)}"
        )
    return names


def texts_of(values: Iterable[object]) -> list[str]:
    """Strings for `parse_many`: anything that is not a string (None, NaN,
    pandas NA) becomes "", which parses to no result."""
    return [v if isinstance(v, str) else "" for v in values]


def extract(
    values: Iterable[object],
    columns: Sequence[str] | None,
    default_state: str | None,
) -> tuple[tuple[str, ...], dict[str, list[str | None]]]:
    """Parse `values` and return the column names and one list per column.
    A row with no address text is `None` in every column."""
    names = check_columns(columns)
    parsed = parse_many(texts_of(values), default_state=default_state)
    out: dict[str, list[str | None]] = {}
    for name in names:
        if name in _DERIVED:
            get = _DERIVED[name]
            out[name] = [get(a) if a is not None else None for a in parsed]
        else:
            out[name] = [getattr(a, name) if a is not None else None for a in parsed]
    return names, out
