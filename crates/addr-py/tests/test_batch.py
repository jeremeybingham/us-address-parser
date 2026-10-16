"""`parse_many`: the batch API. It must give exactly what a loop of `parse`
gives, keep order, report unusable input as `None`, and release the GIL while
Rust parses so other Python threads keep running."""
import threading
import time

import pytest

import us_address_parser as uap

SAMPLE = [
    "123 N Main St Apt 4, Miami FL 33101",
    "PO Box 12, Tampa FL 33601",
    "RR 2 Box 15, Loami IL 62661",
    "PSC 1234 Box 56, APO AE 09012",
    "General Delivery, Tampa FL 33601",
    "22200 SR 60, Lake Wales FL 33898",
    "9418 CUBA RD, PO BOX 100, HAYES VA 23072",
    "NO SITUS",
    "281 EVELYN DR DOTHAN",
    "5 OAK ST UNIT-4, TAMPA FL 33601",
]


def test_same_as_a_loop_of_parse():
    batch = uap.parse_many(SAMPLE)
    assert len(batch) == len(SAMPLE)
    for text, got in zip(SAMPLE, batch):
        assert got.to_dict() == uap.parse(text).to_dict(), text


def test_default_state_applies_to_every_element():
    texts = ["281 EVELYN DR DOTHAN", "100 MAIN ST DOTHAN 36301", "500 OAK ST, ATLANTA, GA 30301"]
    plain = uap.parse_many(texts)
    hinted = uap.parse_many(texts, default_state="AL")
    assert plain[0].city is None and hinted[0].city == "DOTHAN"
    assert [h.to_dict() for h in hinted[1:]] == [p.to_dict() for p in plain[1:]]
    assert [a.to_dict() for a in uap.Parser("AL").parse_many(texts)] == [
        h.to_dict() for h in hinted
    ]
    with pytest.raises(ValueError):
        uap.parse_many(texts, default_state="ZZ")


def test_unusable_elements_are_none_and_do_not_stop_the_batch():
    out = uap.parse_many(["1 Main St, Miami FL", "", "   ", " , ", "2 Oak Ave, Tampa FL"])
    assert [o is None for o in out] == [False, True, True, True, False]
    assert out[4].street_name == "OAK"


def test_accepts_a_list_or_tuple_of_strings_only():
    assert len(uap.parse_many(tuple(SAMPLE))) == len(SAMPLE)
    for bad in ("1 Main St", ["1 Main St", 5], [None], (s for s in SAMPLE)):
        with pytest.raises(TypeError):
            uap.parse_many(bad)


def test_large_batch_keeps_order():
    texts = ["%d Main St, Miami FL 33101" % i for i in range(1, 20001)]
    nums = [a.street_number for a in uap.parse_many(texts)]
    assert nums == [str(i) for i in range(1, 20001)]


def test_other_threads_run_while_rust_parses():
    """The GIL is released during the batch: a counting thread keeps going
    while one big `parse_many` runs."""
    texts = ["%d Main St Apt %d, Miami FL 33101" % (i, i % 9) for i in range(1, 150001)]
    count = [0]
    stop = threading.Event()

    def spin():
        while not stop.is_set():
            count[0] += 1

    t = threading.Thread(target=spin)
    t.start()
    time.sleep(0.05)  # let it start
    before = count[0]
    started = time.perf_counter()
    uap.parse_many(texts)
    elapsed = time.perf_counter() - started
    during = count[0] - before
    stop.set()
    t.join()
    # With the GIL held for the whole call the counter would not move at all.
    assert elapsed > 0.01
    assert during > 1000, (during, elapsed)


def test_pandas_recipe_with_chosen_columns():
    """The README's short recipe: a few attributes, no `to_dict`."""
    pd = pytest.importorskip("pandas")
    df = pd.DataFrame({"id": [1, 2], "address": [SAMPLE[0], ""]})
    parsed = uap.parse_many(df["address"].fillna("").tolist())
    cols = ["street_number", "street_name", "city", "state", "zip5"]
    flat = pd.DataFrame([{c: getattr(a, c) for c in cols} if a else {} for a in parsed])
    flat["key"] = [a.key() if a else None for a in parsed]
    flat["warnings"] = [",".join(a.warnings) if a else None for a in parsed]
    out = pd.concat([df, flat.add_prefix("addr_")], axis=1)
    assert out.loc[0, "addr_street_name"] == "MAIN"
    assert out.loc[0, "addr_key"] == "k1:S|123|N|MAIN|ST||FL|33101"
    assert pd.isna(out.loc[1, "addr_street_name"]) and pd.isna(out.loc[1, "addr_key"])


def test_pandas_recipe():
    pd = pytest.importorskip("pandas")
    df = pd.DataFrame({"id": [1, 2, 3], "address": [SAMPLE[0], None, SAMPLE[1]]})
    parsed = uap.parse_many(df["address"].fillna("").tolist())
    flat = pd.json_normalize([a.to_dict() if a else {} for a in parsed])
    out = pd.concat([df, flat.add_prefix("addr_")], axis=1)
    assert list(out["id"]) == [1, 2, 3]
    assert out.loc[0, "addr_street_name"] == "MAIN"
    assert pd.isna(out.loc[1, "addr_street_name"])
    assert out.loc[2, "addr_po_box"] == "12"
    assert out.loc[0, "addr_place.name"] == "MIAMI"
