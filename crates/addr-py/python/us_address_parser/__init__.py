"""Parse US addresses into components.

USPS Publication 28 and RESO output, a match key and a slug, with
diagnostics. This library formats and parses; it does not validate that an
address exists or can receive mail.
"""
from ._core import (
    ParsedAddress,
    ParseError,
    Parser,
    __version__,
    address_type,
    parse,
    parse_fields,
    parse_many,
    parse_tokens,
    tag,
)

__all__ = [
    "ParseError",
    "ParsedAddress",
    "Parser",
    "__version__",
    "address_type",
    "parse",
    "parse_fields",
    "parse_many",
    "parse_tokens",
    "tag",
]
