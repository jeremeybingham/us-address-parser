"""`tag` and `parse_tokens` against usaddress itself, when it is installed
(`pip install usaddress`). The rates are the ones recorded in
docs/usaddress-comparison.md; the point is to notice a regression, not to
require identical output."""
import pathlib
import xml.etree.ElementTree as ET

import pytest

import us_address_parser as uap

usaddress = pytest.importorskip("usaddress")

FILES = sorted(
    (pathlib.Path(__file__).resolve().parents[3] / "tests" / "external" / "usaddress").glob("*.xml")
)

# Labels this library does not model; a string that uses one is skipped.
OUT_OF_SCOPE = {
    "Recipient", "LandmarkName", "CornerOf", "IntersectionSeparator", "NotAddress",
    "CountryName", "SubaddressType", "SubaddressIdentifier", "StreetNamePreModifier",
    "AddressNumberPrefix",
}


def strings():
    seen = []
    for path in FILES:
        for el in ET.parse(path).getroot().iter("AddressString"):
            text, labels = el.text or "", set()
            for child in el:
                text += (child.text or "") + (child.tail or "")
                labels.add(child.tag)
            if not labels & OUT_OF_SCOPE:
                seen.append(" ".join(text.split()))
    return list(dict.fromkeys(seen))


@pytest.mark.skipif(not FILES, reason="the vendored usaddress test files are not here")
def test_tag_agrees_with_usaddress_on_its_test_files():
    same = types = compared = 0
    for s in strings():
        try:
            theirs, their_type = usaddress.tag(s)
        except usaddress.RepeatedLabelError:
            ours, _ = uap.tag(s)  # never raises
            assert isinstance(ours, dict)
            continue
        ours, our_type = uap.tag(s)
        compared += 1
        same += ours == theirs
        types += our_type == their_type
    assert compared > 5000
    # Recorded: 99.3% of dicts and 99.8% of types are identical.
    assert same / compared >= 0.98, same / compared
    assert types / compared >= 0.99, types / compared


@pytest.mark.skipif(not FILES, reason="the vendored usaddress test files are not here")
def test_parse_tokens_split_the_input_like_usaddress():
    for s in strings()[:300]:
        assert [t for t, _ in uap.parse_tokens(s)] == [t for t, _ in usaddress.parse(s)], s


def test_a_repeated_label_is_joined_not_raised():
    s = "123 Main St Apt 4 Ste 5, Miami FL 33101"
    ours, kind = uap.tag(s)
    assert kind == "Street Address"
    assert ours["OccupancyType"] == "Apt Ste"
    assert uap.parse(s).repeated_labels() == ["OccupancyType", "OccupancyIdentifier"]
