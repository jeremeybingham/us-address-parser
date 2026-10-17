"""Polars helper: parse a column of addresses into component columns.

Needs Polars (`pip install us-address-parser[polars]`); it is imported when
`parse_column` is called, not when this module is. This library formats and
parses; it does not validate that an address exists or can receive mail.
"""
from __future__ import annotations

from collections.abc import Sequence
from typing import TYPE_CHECKING

from ._columns import AVAILABLE_COLUMNS, DEFAULT_COLUMNS, check_columns, extract

if TYPE_CHECKING:
    import polars as pl

__all__ = ["AVAILABLE_COLUMNS", "DEFAULT_COLUMNS", "parse_column"]


def parse_column(
    series: "pl.Series",
    *,
    columns: Sequence[str] | None = None,
    prefix: str = "addr_",
    default_state: str | None = None,
) -> "pl.DataFrame":
    """Parse each one-line address in `series`.

    Returns a DataFrame with the same length and order as `series` and one
    `String` column per name in `columns`, each prefixed with `prefix`. A
    blank, null or non-string row (and one with no address text) is null in
    every column. `columns` defaults to `DEFAULT_COLUMNS`; `AVAILABLE_COLUMNS`
    lists the others. `warnings` is the warning codes joined by commas. Add
    the result to a frame with `df.hstack(parse_column(df["address"]))`.
    """
    try:
        import polars as pl
    except ImportError as e:  # pragma: no cover - exercised only without polars
        raise ImportError(
            "us_address_parser.polars needs Polars: pip install 'us-address-parser[polars]'"
        ) from e
    check_columns(columns)
    names, data = extract(series, columns, default_state)
    # An explicit schema: a column that is null in every row would otherwise
    # be typed Null.
    return pl.DataFrame(
        {f"{prefix}{n}": data[n] for n in names},
        schema={f"{prefix}{n}": pl.String for n in names},
    )
