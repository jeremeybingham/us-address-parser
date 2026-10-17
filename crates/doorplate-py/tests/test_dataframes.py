"""Dataframe helpers (`doorplate.pandas` and `.polars`): the columns
equal what `parse` gives per row, blank rows are null, order and index are
kept. Each class skips when its library is not installed."""
import pytest

import doorplate as uap
from doorplate import _columns

TEXTS = [
    "123 N Main St Apt 4, Miami FL 33101",
    "",
    "PO Box 12, Tampa FL 33601",
    "!!!",
    "281 EVELYN DR DOTHAN",
]
BLANK_ROWS = {1, 3}


def test_columns_validation():
    assert _columns.check_columns(None) == _columns.DEFAULT_COLUMNS
    with pytest.raises(ValueError, match="unknown column"):
        _columns.check_columns(["street_name", "nope"])
    with pytest.raises(TypeError):
        _columns.check_columns("street_name")


def test_every_available_column_is_readable():
    names, data = _columns.extract(TEXTS, sorted(_columns.AVAILABLE_COLUMNS), None)
    assert len(names) == len(_columns.AVAILABLE_COLUMNS)
    for col in data.values():
        assert len(col) == len(TEXTS)
        assert all(col[i] is None for i in BLANK_ROWS)
        assert all(v is None or isinstance(v, str) for v in col)


def test_extract_matches_parse():
    _, data = _columns.extract(TEXTS, None, None)
    a = uap.parse(TEXTS[0])
    assert data["street_name"][0] == a.street_name == "MAIN"
    assert data["key"][0] == a.key()
    assert data["slug"][0] == a.slug()
    assert data["address_type"][2] == "po_box"
    assert data["warnings"][0] == ",".join(a.warnings)
    assert _columns.extract(TEXTS, ["usps_line1"], None)[1]["usps_line1"][0] == "123 N MAIN ST APT 4"


class TestPandas:
    pd = pytest.importorskip("pandas")

    def test_columns_and_values(self):
        from doorplate.pandas import DEFAULT_COLUMNS, parse_column

        s = self.pd.Series(TEXTS, index=[10, 11, 12, 13, 14], name="address")
        out = parse_column(s)
        assert list(out.columns) == [f"addr_{c}" for c in DEFAULT_COLUMNS]
        assert list(out.index) == [10, 11, 12, 13, 14]
        assert all(str(t) == "string" for t in out.dtypes)
        assert out.loc[10, "addr_street_name"] == "MAIN"
        assert out.loc[10, "addr_zip5"] == "33101"
        assert out.loc[12, "addr_address_type"] == "po_box"
        for i in (11, 13):
            assert out.loc[i].isna().all()

    def test_null_and_non_string_rows(self):
        from doorplate.pandas import parse_column

        s = self.pd.Series(["1 Main St, Miami FL 33101", None, float("nan"), 5])
        out = parse_column(s, columns=["street_name"])
        assert out["addr_street_name"].tolist()[0] == "MAIN"
        assert out["addr_street_name"].isna().tolist() == [False, True, True, True]

    def test_options_and_join(self):
        from doorplate.pandas import parse_column

        df = self.pd.DataFrame({"address": ["281 EVELYN DR DOTHAN"], "id": [7]})
        out = df.join(parse_column(df["address"], columns=["city"], prefix="p_", default_state="AL"))
        assert out.loc[0, "p_city"] == "DOTHAN"
        assert list(out.columns) == ["address", "id", "p_city"]
        with pytest.raises(ValueError):
            parse_column(df["address"], default_state="ZZ")

    def test_same_as_a_loop_of_parse(self):
        from doorplate.pandas import parse_column

        out = parse_column(self.pd.Series(TEXTS), columns=["street_number", "key", "usps_line1"])
        for i, t in enumerate(TEXTS):
            if i in BLANK_ROWS:
                continue
            a = uap.parse(t)
            assert out.loc[i, "addr_street_number"] == a.street_number or (
                a.street_number is None and out["addr_street_number"].isna()[i]
            )
            assert out.loc[i, "addr_key"] == a.key()


class TestPolars:
    pl = pytest.importorskip("polars")

    def test_columns_and_values(self):
        from doorplate.polars import DEFAULT_COLUMNS, parse_column

        out = parse_column(self.pl.Series("address", TEXTS))
        assert out.columns == [f"addr_{c}" for c in DEFAULT_COLUMNS]
        assert out.height == len(TEXTS)
        assert set(out.dtypes) == {self.pl.String}
        assert out["addr_street_name"][0] == "MAIN"
        assert out["addr_address_type"][2] == "po_box"
        for i in BLANK_ROWS:
            assert all(v is None for v in out.row(i))

    def test_nulls_and_all_null_column(self):
        from doorplate.polars import parse_column

        out = parse_column(self.pl.Series("a", [None, ""], dtype=self.pl.String))
        assert out.height == 2
        assert set(out.dtypes) == {self.pl.String}
        assert out["addr_key"].null_count() == 2

    def test_options_and_hstack(self):
        from doorplate.polars import parse_column

        df = self.pl.DataFrame({"address": ["281 EVELYN DR DOTHAN"]})
        out = df.hstack(parse_column(df["address"], columns=["city"], prefix="p_", default_state="AL"))
        assert out.columns == ["address", "p_city"]
        assert out["p_city"][0] == "DOTHAN"
        with pytest.raises(ValueError):
            parse_column(df["address"], columns=["bogus"])
