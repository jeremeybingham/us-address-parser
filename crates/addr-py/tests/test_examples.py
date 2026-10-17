"""The examples in `examples/` run and give sensible output, so they stay
current. Each test skips when its library (pandas, Polars, nbclient) is not
installed."""
import runpy
from pathlib import Path

import pytest

EXAMPLES = Path(__file__).resolve().parents[3] / "examples"


def run_script(name):
    return runpy.run_path(str(EXAMPLES / name), run_name="example")["main"]()


def test_pandas_pipeline(capsys):
    pytest.importorskip("pandas")
    df = run_script("pandas_pipeline.py")
    assert len(df) == 12
    assert df.loc[0, "addr_street_name"] == "HARBOR"
    assert df["addr_key"].isna().sum() == 1  # the row with no address text
    assert "row(s) with warnings" in capsys.readouterr().out


def test_polars_pipeline(capsys):
    pytest.importorskip("polars")
    df = run_script("polars_pipeline.py")
    assert df.height == 12
    assert df["addr_street_name"][0] == "HARBOR"
    assert df["addr_key"].null_count() == 1
    assert "row(s) with warnings" in capsys.readouterr().out


def test_notebook_executes():
    pytest.importorskip("pandas")
    nbformat = pytest.importorskip("nbformat")
    nbclient = pytest.importorskip("nbclient")
    pytest.importorskip("ipykernel")
    nb = nbformat.read(EXAMPLES / "quickstart.ipynb", as_version=4)
    nbclient.NotebookClient(
        nb, timeout=120, resources={"metadata": {"path": str(EXAMPLES)}}
    ).execute()
    text = "".join(
        o.get("text", "") for c in nb.cells if c.cell_type == "code" for o in c.outputs
    )
    assert "k1:S|123|N|MAIN|ST||FL|33101" in text
    assert "intersection_detected" in text


@pytest.mark.parametrize("name", ["addresses_oneline.csv", "addresses_split.csv", "addresses_messy.txt"])
def test_data_files_exist(name):
    assert (EXAMPLES / "data" / name).stat().st_size > 0
