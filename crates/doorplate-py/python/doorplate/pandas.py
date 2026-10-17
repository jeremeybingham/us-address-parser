"""pandas helper: parse a column of addresses into component columns.

Needs pandas (`pip install doorplate[pandas]`); it is imported when
`parse_column` is called, not when this module is. This library formats and
parses; it does not validate that an address exists or can receive mail.
"""
from __future__ import annotations

from collections.abc import Sequence
from typing import TYPE_CHECKING

from ._columns import AVAILABLE_COLUMNS, DEFAULT_COLUMNS, check_columns, extract

if TYPE_CHECKING:
    import pandas as pd

__all__ = ["AVAILABLE_COLUMNS", "DEFAULT_COLUMNS", "parse_column"]


def parse_column(
    series: "pd.Series",
    *,
    columns: Sequence[str] | None = None,
    prefix: str = "addr_",
    default_state: str | None = None,
) -> "pd.DataFrame":
    """Parse each one-line address in `series`.

    Returns a DataFrame with the same index as `series` and one `string`-dtype
    column per name in `columns`, each prefixed with `prefix`. A blank, null
    or non-string row (and one with no address text) is null in every column.
    `columns` defaults to `DEFAULT_COLUMNS`; `AVAILABLE_COLUMNS` lists the
    others. `warnings` is the warning codes joined by commas. Join the result
    back with `df.join(parse_column(df["address"]))`.
    """
    try:
        import pandas as pd
    except ImportError as e:  # pragma: no cover - exercised only without pandas
        raise ImportError(
            "doorplate.pandas needs pandas: pip install 'doorplate[pandas]'"
        ) from e
    check_columns(columns)
    names, data = extract(series, columns, default_state)
    return pd.DataFrame(
        {f"{prefix}{n}": data[n] for n in names},
        index=series.index,
        dtype="string",
    )
