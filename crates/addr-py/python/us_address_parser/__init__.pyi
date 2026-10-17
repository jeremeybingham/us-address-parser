"""Type stubs for us_address_parser (the compiled module is `_core`)."""
from collections.abc import Sequence
from typing import TypedDict

__version__: str

class ParseError(ValueError):
    """The input has no address text at all (empty or only punctuation)."""

class Highway(TypedDict):
    """A street named after a highway (`KY STATE HIGHWAY 625`)."""

    state: str | None
    prefix: str
    route: str
    name: str

class RuralRoute(TypedDict):
    """`RR 2 BOX 15` or `HC 3 BOX 4`."""

    kind: str
    route: str
    box: str | None

class Military(TypedDict):
    """`PSC 1650 BOX 10`."""

    kind: str
    number: str
    box: str | None

class Place(TypedDict):
    """The place the city stands for (`city` itself is never changed)."""

    name: str
    state: str
    county: str
    kind: str  # "municipality" | "place" | "subdivision"
    source: str  # "name" | "variant" | "completed" | "zip"

class Usps(TypedDict):
    line1: str | None
    line2: str | None
    city: str | None
    state: str | None
    zip: str | None
    last_line: str

class ParsedAddressDict(TypedDict):
    input: str
    street_number: str | None
    street_dir_prefix: str | None
    street_name: str | None
    street_suffix: str | None
    street_dir_suffix: str | None
    highway: Highway | None
    unit_type: str | None
    unit_number: str | None
    second_unit_type: str | None
    second_unit_number: str | None
    po_box: str | None
    rural_route: RuralRoute | None
    general_delivery: bool
    military: Military | None
    military_post_office: str | None
    attention: str | None
    building: str | None
    city: str | None
    state: str | None
    zip5: str | None
    zip4: str | None
    place: Place | None
    warnings: list[str]
    unparsed: str | None
    confidence: float | None
    address_type: str
    key: str | None
    key_unit: str | None
    slug: str | None
    usps: Usps
    reso: dict[str, str]

class ParsedAddress:
    """A parsed address. Read-only. Formats and parses; it does not validate
    that the address exists or can receive mail."""

    @property
    def input(self) -> str: ...
    @property
    def street_number(self) -> str | None: ...
    @property
    def street_dir_prefix(self) -> str | None: ...
    @property
    def street_name(self) -> str | None: ...
    @property
    def street_suffix(self) -> str | None: ...
    @property
    def street_dir_suffix(self) -> str | None: ...
    @property
    def highway(self) -> Highway | None: ...
    @property
    def unit_type(self) -> str | None: ...
    @property
    def unit_number(self) -> str | None: ...
    @property
    def second_unit_type(self) -> str | None: ...
    @property
    def second_unit_number(self) -> str | None: ...
    @property
    def po_box(self) -> str | None: ...
    @property
    def rural_route(self) -> RuralRoute | None: ...
    @property
    def general_delivery(self) -> bool: ...
    @property
    def military(self) -> Military | None: ...
    @property
    def military_post_office(self) -> str | None: ...
    @property
    def attention(self) -> str | None: ...
    @property
    def building(self) -> str | None: ...
    @property
    def city(self) -> str | None: ...
    @property
    def state(self) -> str | None: ...
    @property
    def zip5(self) -> str | None: ...
    @property
    def zip4(self) -> str | None: ...
    @property
    def place(self) -> Place | None: ...
    @property
    def warnings(self) -> list[str]: ...
    @property
    def unparsed(self) -> str | None: ...
    @property
    def confidence(self) -> float | None: ...
    @property
    def library_version(self) -> str: ...
    def address_type(self) -> str: ...
    def key(self, unit: bool = False) -> str | None: ...
    def slug(self) -> str | None: ...
    def to_usps(self, max_line: int | None = None) -> Usps: ...
    def to_reso(self) -> dict[str, str]: ...
    def to_usaddress_tags(self) -> list[tuple[str, str | None]]: ...
    def tag(self) -> tuple[dict[str, str], str]: ...
    def repeated_labels(self) -> list[str]: ...
    def to_dict(self) -> ParsedAddressDict: ...

class Parser:
    def __init__(self, default_state: str | None = None) -> None: ...
    def parse(self, text: str) -> ParsedAddress: ...
    def parse_fields(
        self,
        line1: str | None = None,
        line2: str | None = None,
        city: str | None = None,
        state: str | None = None,
        zip: str | None = None,
    ) -> ParsedAddress: ...
    def parse_many(self, texts: Sequence[str]) -> list[ParsedAddress | None]: ...

def parse(text: str, default_state: str | None = None) -> ParsedAddress: ...
def parse_fields(
    line1: str | None = None,
    line2: str | None = None,
    city: str | None = None,
    state: str | None = None,
    zip: str | None = None,
    default_state: str | None = None,
) -> ParsedAddress: ...
def parse_many(
    texts: Sequence[str], default_state: str | None = None
) -> list[ParsedAddress | None]: ...
def tag(text: str, default_state: str | None = None) -> tuple[dict[str, str], str]: ...
def parse_tokens(
    text: str, default_state: str | None = None
) -> list[tuple[str, str | None]]: ...
def address_type(text: str, default_state: str | None = None) -> str: ...
