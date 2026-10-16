"""Tests for the wrapper: every value comes from addr-core, so these check
the Python surface (types, errors, shapes), not the parsing rules."""
import ast
import pathlib
import threading

import pytest

import us_address_parser as uap

PACKAGE = pathlib.Path(uap.__file__).parent


def test_parse_components_and_formats():
    a = uap.parse("123 north main street apt 4, Miami FL 33101")
    assert (a.street_number, a.street_dir_prefix, a.street_name, a.street_suffix) == (
        "123", "N", "MAIN", "ST",
    )
    assert (a.unit_type, a.unit_number) == ("APT", "4")
    assert (a.city, a.state, a.zip5, a.zip4) == ("MIAMI", "FL", "33101", None)
    assert a.place == {
        "name": "MIAMI", "state": "FL", "county": "DADE",
        "kind": "municipality", "source": "name",
    }
    assert a.warnings == []
    assert a.unparsed is None
    assert a.to_usps()["line1"] == "123 N MAIN ST APT 4"
    assert a.to_usps()["last_line"] == "MIAMI FL 33101"
    assert a.key() == "k1:S|123|N|MAIN|ST||FL|33101"
    assert a.key(unit=True) == "k1:S|123|N|MAIN|ST||FL|33101|4|"
    assert a.slug() == "123-n-main-st-apt-4-miami-fl-33101"
    assert a.to_reso()["StreetName"] == "MAIN"
    assert a.to_reso()["UnparsedAddress"] == "123 N MAIN ST APT 4, MIAMI, FL 33101"
    assert a.address_type() == "street"
    assert a.input == "123 north main street apt 4, Miami FL 33101"


def test_absent_components_are_none_and_confidence_is_uncalibrated():
    a = uap.parse("PO Box 12, Tampa FL 33601")
    assert a.po_box == "12" and a.street_name is None and a.highway is None
    assert a.address_type() == "po_box"
    assert a.confidence is None
    assert a.model_version is None
    assert a.library_version == uap.__version__


def test_dict_shapes():
    assert uap.parse("22200 SR 60, Lake Wales FL 33898").highway == {
        "state": None, "prefix": "STATE ROAD", "route": "60", "name": "STATE ROAD 60",
    }
    assert uap.parse("RR 2 Box 15, Loami IL 62661").rural_route == {
        "kind": "RR", "route": "2", "box": "15",
    }
    assert uap.parse("PSC 1234 Box 56, APO AE 09012").military == {
        "kind": "PSC", "number": "1234", "box": "56",
    }
    assert uap.parse("General Delivery, Tampa FL 33601").general_delivery is True


def test_parse_fields():
    a = uap.parse_fields(
        line1="THE COVE", line2="1111 ASBURY WAY", city="Orlando", state="FL", zip="32801"
    )
    assert a.attention == "THE COVE"
    assert a.street_number == "1111" and a.state == "FL"
    assert a.input == "THE COVE, 1111 ASBURY WAY, Orlando, FL, 32801"


def test_unusable_input_raises_parse_error():
    for text in ("", "   ", " , "):
        with pytest.raises(uap.ParseError):
            uap.parse(text)
    with pytest.raises(ValueError):  # ParseError is a ValueError
        uap.parse("")
    with pytest.raises(uap.ParseError):
        uap.parse_fields()


def test_parse_many_keeps_order_and_gives_none_for_unusable_input():
    texts = ["1 Main St, Miami FL 33101", "", "PO Box 5, Tampa FL 33601"]
    out = uap.parse_many(texts)
    assert [o and o.input for o in out] == [texts[0], None, texts[2]]
    assert uap.parse_many([]) == []
    big = ["%d Main St, Miami FL 33101" % i for i in range(1, 2001)]
    assert [a.street_number for a in uap.parse_many(big)] == [str(i) for i in range(1, 2001)]


def test_parse_many_matches_parse_across_threads():
    texts = ["%d Oak Ave Apt %d, Tampa FL 33601" % (i, i % 7) for i in range(1, 201)]
    expected = [uap.parse(t).to_dict() for t in texts]
    results = []

    def work():
        results.append([a.to_dict() for a in uap.parse_many(texts)])

    threads = [threading.Thread(target=work) for _ in range(4)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    assert len(results) == 4 and all(r == expected for r in results)


def test_default_state_is_a_hint_and_never_a_fill():
    plain = uap.parse("281 EVELYN DR DOTHAN")
    assert plain.city is None
    hinted = uap.parse("281 EVELYN DR DOTHAN", default_state="AL")
    assert hinted.city == "DOTHAN" and hinted.place["state"] == "AL"
    assert hinted.state is None
    assert "default_state_applied" in hinted.warnings and "missing_state" in hinted.warnings
    assert uap.Parser(default_state="florida").parse("22200 SR 60").highway["prefix"] == "STATE ROAD"
    assert uap.Parser(default_state=None).parse("1 Main St").state is None
    assert uap.Parser().parse_many(["281 EVELYN DR DOTHAN"])[0].city is None
    for bad in ("ZZ", "AE", "Narnia"):
        with pytest.raises(ValueError):
            uap.Parser(default_state=bad)
        with pytest.raises(ValueError):
            uap.parse("1 Main St", default_state=bad)


def test_tag_parse_tokens_and_address_type():
    d, kind = uap.tag("123 N. Main Street, Apt 4, Santa Fe NM 87505")
    assert kind == "Street Address"
    assert d["AddressNumber"] == "123" and d["StreetName"] == "Main"
    assert d["PlaceName"] == "Santa Fe" and d["ZipCode"] == "87505"
    assert uap.tag("PO Box 12, Tampa FL 33601")[1] == "PO Box"
    assert uap.tag("General Delivery, Tampa FL 33601")[1] == "Ambiguous"
    assert uap.parse_tokens("12 Oak St")[:2] == [("12", "AddressNumber"), ("Oak", "StreetName")]
    assert uap.parse_tokens("409742ND SQ") == [("409742ND", None), ("SQ", None)]
    assert uap.address_type("RR 2 Box 15, Loami IL") == "rural_route"
    assert uap.address_type("9418 CUBA RD, PO BOX 100, HAYES VA 23072") == "dual"
    # No RepeatedLabelError: the runs are joined and the labels reported.
    a = uap.parse("123 MAIN ST APT 4 STE 5, MIAMI FL 33101")
    d, _ = a.tag()
    assert d["OccupancyType"] == "APT STE" and d["OccupancyIdentifier"] == "4 5"
    assert a.repeated_labels() == ["OccupancyType", "OccupancyIdentifier"]
    assert uap.parse("1 Main St, Miami FL").repeated_labels() == []


def test_to_usps_wrapped():
    a = uap.parse("123 Very Long Street Name Boulevard Suite 1200, Miami FL 33101")
    assert a.to_usps()["line2"] is None
    wrapped = a.to_usps(max_line=30)
    assert wrapped["line2"] is not None and len(wrapped["line1"]) <= 30


def test_parsed_address_is_read_only():
    a = uap.parse("1 Main St, Miami FL 33101")
    with pytest.raises(AttributeError):
        a.street_name = "ELM"


def test_to_dict_matches_the_type_stub():
    stub = ast.parse((PACKAGE / "__init__.pyi").read_text())
    typed = {n.name: n for n in stub.body if isinstance(n, ast.ClassDef)}
    keys = [s.target.id for s in typed["ParsedAddressDict"].body if isinstance(s, ast.AnnAssign)]
    d = uap.parse("123 N Main St Apt 4, Miami FL 33101").to_dict()
    assert sorted(d) == sorted(keys)
    # Nested dict shapes.
    for cls, value in (
        ("Usps", d["usps"]),
        ("Place", d["place"]),
        ("Highway", uap.parse("22200 SR 60, Lake Wales FL 33898").highway),
        ("RuralRoute", uap.parse("RR 2 Box 15, Loami IL 62661").rural_route),
        ("Military", uap.parse("PSC 1234 Box 56, APO AE 09012").military),
    ):
        names = [s.target.id for s in typed[cls].body if isinstance(s, ast.AnnAssign)]
        assert sorted(value) == sorted(names), cls


def test_stub_covers_the_public_names():
    stub = ast.parse((PACKAGE / "__init__.pyi").read_text())
    defined = {n.name for n in stub.body if isinstance(n, (ast.ClassDef, ast.FunctionDef))}
    defined |= {
        t.id for n in stub.body if isinstance(n, ast.AnnAssign)
        for t in [n.target] if isinstance(t, ast.Name)
    }
    assert set(uap.__all__) <= defined
    methods = {
        n.name for c in stub.body if isinstance(c, ast.ClassDef) and c.name == "ParsedAddress"
        for n in c.body if isinstance(n, ast.FunctionDef)
    }
    public = {m for m in dir(uap.ParsedAddress) if not m.startswith("_")}
    assert public == methods, public ^ methods


def test_package_is_typed():
    assert (PACKAGE / "py.typed").exists()
