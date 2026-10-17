//! Python bindings for `doorplate`, built with PyO3 and maturin.
//!
//! A thin layer: every value comes from `doorplate`, nothing is parsed or
//! formatted here. `ParsedAddress` wraps the core type and reads it; the
//! emitters are the core's. This library formats and parses; it does not
//! validate deliverability.
//!
//! Built with maturin as the extension module `doorplate._core`; the
//! package (`python/doorplate/`) re-exports it.

use std::sync::OnceLock;

use doorplate::{
    Fields, Highway, KeyOptions, OptionsError, ParseError as CoreParseError, ParseOptions,
    ParsedAddress, Parser, PlaceKind,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

pyo3::create_exception!(
    _core,
    ParseError,
    PyValueError,
    "The input has no address text at all (empty or only punctuation)."
);

fn parse_err(e: &CoreParseError) -> PyErr {
    ParseError::new_err(e.to_string())
}

/// The parser for a `default_state` hint (`None` is the shared default one).
fn parser_for(default_state: Option<&str>) -> PyResult<Parser> {
    static DEFAULT: OnceLock<Parser> = OnceLock::new();
    match default_state {
        None => Ok(DEFAULT.get_or_init(Parser::new).clone()),
        Some(s) => Parser::with_options(ParseOptions {
            default_state: Some(s.to_string()),
        })
        .map_err(|e: OptionsError| PyValueError::new_err(e.to_string())),
    }
}

/// A parsed address. Read-only; the values are the core's.
#[pyclass(name = "ParsedAddress", module = "doorplate", frozen)]
struct PyParsedAddress {
    inner: ParsedAddress,
}

fn dict_or_none<'py>(
    py: Python<'py>,
    items: Option<Vec<(&'static str, Option<String>)>>,
) -> PyResult<Option<Bound<'py, PyDict>>> {
    let Some(items) = items else { return Ok(None) };
    let d = PyDict::new(py);
    for (k, v) in items {
        d.set_item(k, v)?;
    }
    Ok(Some(d))
}

#[pymethods]
impl PyParsedAddress {
    /// The input as given (for fields, the non-empty fields joined with ", ").
    #[getter]
    fn input(&self) -> &str {
        &self.inner.source.original
    }

    #[getter]
    fn street_number(&self) -> Option<&str> {
        self.inner.street_number.as_deref()
    }

    /// Directional before the name, as its USPS abbreviation (`N`, `SW`).
    #[getter]
    fn street_dir_prefix(&self) -> Option<&'static str> {
        self.inner.street_dir_prefix.map(|d| d.abbr())
    }

    #[getter]
    fn street_name(&self) -> Option<&str> {
        self.inner.street_name.as_deref()
    }

    /// Street suffix, as its USPS abbreviation (`ST`, `AVE`).
    #[getter]
    fn street_suffix(&self) -> Option<&'static str> {
        self.inner.street_suffix.map(|s| s.abbr())
    }

    #[getter]
    fn street_dir_suffix(&self) -> Option<&'static str> {
        self.inner.street_dir_suffix.map(|d| d.abbr())
    }

    /// A street named after a highway: `{"state", "prefix", "route",
    /// "name"}`, or `None`.
    #[getter]
    fn highway<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyDict>>> {
        dict_or_none(
            py,
            self.inner.highway.as_ref().map(|h: &Highway| {
                vec![
                    ("state", h.state.map(|s| s.code().to_string())),
                    ("prefix", Some(h.prefix.standard().to_string())),
                    ("route", Some(h.route.clone())),
                    ("name", Some(h.name())),
                ]
            }),
        )
    }

    #[getter]
    fn unit_type(&self) -> Option<&'static str> {
        self.inner.unit_type.map(|u| u.abbr())
    }

    #[getter]
    fn unit_number(&self) -> Option<&str> {
        self.inner.unit_number.as_deref()
    }

    #[getter]
    fn second_unit_type(&self) -> Option<&'static str> {
        self.inner.second_unit_type.map(|u| u.abbr())
    }

    #[getter]
    fn second_unit_number(&self) -> Option<&str> {
        self.inner.second_unit_number.as_deref()
    }

    #[getter]
    fn po_box(&self) -> Option<&str> {
        self.inner.po_box.as_deref()
    }

    /// `{"kind": "RR" | "HC", "route", "box"}`, or `None`.
    #[getter]
    fn rural_route<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyDict>>> {
        dict_or_none(
            py,
            self.inner.rural_route.as_ref().map(|r| {
                vec![
                    ("kind", Some(r.kind.abbr().to_string())),
                    ("route", Some(r.route.clone())),
                    ("box", r.box_number.clone()),
                ]
            }),
        )
    }

    #[getter]
    fn general_delivery(&self) -> bool {
        self.inner.general_delivery
    }

    /// `{"kind": "PSC" | "CMR" | "UNIT" | ..., "number", "box"}`, or `None`.
    #[getter]
    fn military<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyDict>>> {
        dict_or_none(
            py,
            self.inner.military.as_ref().map(|m| {
                vec![
                    ("kind", Some(m.kind.abbr().to_string())),
                    ("number", Some(m.number.clone())),
                    ("box", m.box_number.clone()),
                ]
            }),
        )
    }

    /// `APO`, `FPO` or `DPO` where the city goes.
    #[getter]
    fn military_post_office(&self) -> Option<&'static str> {
        self.inner.military_post_office.map(|m| m.abbr())
    }

    #[getter]
    fn attention(&self) -> Option<&str> {
        self.inner.attention.as_deref()
    }

    #[getter]
    fn building(&self) -> Option<&str> {
        self.inner.building.as_deref()
    }

    /// The city as the input wrote it.
    #[getter]
    fn city(&self) -> Option<&str> {
        self.inner.city.as_deref()
    }

    /// Two-letter USPS state code.
    #[getter]
    fn state(&self) -> Option<&'static str> {
        self.inner.state.map(|s| s.code())
    }

    #[getter]
    fn zip5(&self) -> Option<&str> {
        self.inner.zip5.as_deref()
    }

    #[getter]
    fn zip4(&self) -> Option<&str> {
        self.inner.zip4.as_deref()
    }

    /// The place the city stands for: `{"name", "state", "county", "kind",
    /// "source"}`, or `None`. `city` is never changed.
    #[getter]
    fn place<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyDict>>> {
        dict_or_none(
            py,
            self.inner.place.map(|m| {
                vec![
                    ("name", Some(m.place.name().to_string())),
                    ("state", Some(m.place.state().to_string())),
                    ("county", Some(m.place.county().to_string())),
                    (
                        "kind",
                        Some(
                            match m.place.kind() {
                                PlaceKind::Municipality => "municipality",
                                PlaceKind::Place => "place",
                                PlaceKind::Subdivision => "subdivision",
                            }
                            .to_string(),
                        ),
                    ),
                    ("source", Some(m.source.code().to_string())),
                ]
            }),
        )
    }

    /// Warning codes (`"missing_zip"`, ...), in the order they were raised.
    #[getter]
    fn warnings(&self) -> Vec<&'static str> {
        self.inner
            .diagnostics
            .warnings
            .iter()
            .map(|w| w.code())
            .collect()
    }

    /// Text the parse could not place, or `None`.
    #[getter]
    fn unparsed(&self) -> Option<&str> {
        self.inner.diagnostics.unparsed.as_deref()
    }

    /// Reserved for a calibrated probability that the parse is right. Always
    /// `None`: this version does not compute a score.
    #[getter]
    fn confidence(&self) -> Option<f32> {
        self.inner.diagnostics.confidence
    }

    #[getter]
    fn library_version(&self) -> &'static str {
        self.inner.diagnostics.library_version
    }

    /// `street`, `po_box`, `rural_route`, `military`, `general_delivery`,
    /// `dual` or `unknown`.
    fn address_type(&self) -> &'static str {
        self.inner.address_type().code()
    }

    /// The match key (`k1:...`), or `None` when there is nothing to key on.
    /// With `unit=True` the unit numbers are part of it.
    #[pyo3(signature = (unit=false))]
    fn key(&self, unit: bool) -> Option<String> {
        self.inner.key(if unit {
            KeyOptions::unit()
        } else {
            KeyOptions::property()
        })
    }

    /// A URL-safe label, for display only. Use `key()` for identity.
    fn slug(&self) -> Option<String> {
        self.inner.slug()
    }

    /// The USPS Publication 28 form: `{"line1", "line2", "city", "state",
    /// "zip", "last_line"}`. With `max_line`, a unit that does not fit on
    /// `line1` moves to `line2`.
    #[pyo3(signature = (max_line=None))]
    fn to_usps<'py>(
        &self,
        py: Python<'py>,
        max_line: Option<usize>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let u = match max_line {
            Some(n) => self.inner.to_usps_wrapped(n),
            None => self.inner.to_usps(),
        };
        let d = PyDict::new(py);
        d.set_item("line1", u.line1.clone())?;
        d.set_item("line2", u.line2.clone())?;
        d.set_item("city", u.city.clone())?;
        d.set_item("state", u.state.clone())?;
        d.set_item("zip", u.zip.clone())?;
        d.set_item("last_line", u.last_line())?;
        Ok(d)
    }

    /// RESO Data Dictionary 2.0 fields by name; a field with no value is left
    /// out.
    fn to_reso<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        for (k, v) in self.inner.to_reso().fields() {
            d.set_item(k, v)?;
        }
        Ok(d)
    }

    /// Every input token with the label usaddress would give it:
    /// `[(token, label)]`; `label` is `None` for a token the parse did not
    /// place.
    fn to_usaddress_tags(&self) -> Vec<(String, Option<&'static str>)> {
        self.inner
            .to_usaddress_tags()
            .into_iter()
            .map(|t| (t.text, t.label))
            .collect()
    }

    /// The counterpart of `usaddress.tag()`: `(dict, address_type)` with
    /// usaddress's label names and its three type names. It never raises
    /// `RepeatedLabelError`: the runs are joined and `repeated_labels()`
    /// lists the labels.
    fn tag<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyDict>, &'static str)> {
        let t = self.inner.tag();
        let d = PyDict::new(py);
        for (label, value) in t.components() {
            d.set_item(label, value)?;
        }
        Ok((d, t.usaddress_type()))
    }

    /// The labels `tag()` joined from separate runs (two units give
    /// `OccupancyType`); empty when none.
    fn repeated_labels(&self) -> Vec<&'static str> {
        self.inner.tag().repeated_labels().to_vec()
    }

    /// Everything as plain Python values: the components (absent ones are
    /// `None`), `warnings`, `unparsed`, `place`, `address_type`, `key`,
    /// `key_unit`, `slug`, `usps` and `reso`.
    fn to_dict<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyDict>> {
        let py = slf.py();
        let a = slf.get();
        let d = PyDict::new(py);
        d.set_item("input", a.input())?;
        for name in [
            "street_number",
            "street_dir_prefix",
            "street_name",
            "street_suffix",
            "street_dir_suffix",
            "highway",
            "unit_type",
            "unit_number",
            "second_unit_type",
            "second_unit_number",
            "po_box",
            "rural_route",
            "general_delivery",
            "military",
            "military_post_office",
            "attention",
            "building",
            "city",
            "state",
            "zip5",
            "zip4",
            "place",
            "warnings",
            "unparsed",
            "confidence",
        ] {
            d.set_item(name, slf.getattr(name)?)?;
        }
        d.set_item("address_type", a.address_type())?;
        d.set_item("key", a.key(false))?;
        d.set_item("key_unit", a.key(true))?;
        d.set_item("slug", a.slug())?;
        d.set_item("usps", a.to_usps(py, None)?)?;
        d.set_item("reso", a.to_reso(py)?)?;
        Ok(d)
    }

    fn __repr__(&self) -> String {
        format!("ParsedAddress({:?})", self.inner.source.original)
    }
}

/// An address parser with settings. Cheap to construct and thread-safe.
#[pyclass(name = "Parser", module = "doorplate", frozen)]
struct PyParser {
    inner: Parser,
}

fn wrap(a: ParsedAddress) -> PyParsedAddress {
    PyParsedAddress { inner: a }
}

#[pymethods]
impl PyParser {
    /// `default_state` (a USPS code or a state name) is a hint for lookups
    /// when the input names no state: it never fills `state`. Raises
    /// `ValueError` for anything that is not a US state, DC or territory.
    #[new]
    #[pyo3(signature = (default_state=None))]
    fn new(default_state: Option<&str>) -> PyResult<Self> {
        Ok(Self {
            inner: parser_for(default_state)?,
        })
    }

    /// Parse one address string. Raises `ParseError` only when there is no
    /// address text at all; anything else comes back with warnings.
    fn parse(&self, text: &str) -> PyResult<PyParsedAddress> {
        self.inner.parse(text).map(wrap).map_err(|e| parse_err(&e))
    }

    /// Parse pre-split fields.
    #[pyo3(signature = (line1=None, line2=None, city=None, state=None, zip=None))]
    fn parse_fields(
        &self,
        line1: Option<String>,
        line2: Option<String>,
        city: Option<String>,
        state: Option<String>,
        zip: Option<String>,
    ) -> PyResult<PyParsedAddress> {
        let fields = Fields {
            line1,
            line2,
            city,
            state,
            zip,
        };
        self.inner
            .parse_fields(&fields)
            .map(wrap)
            .map_err(|e| parse_err(&e))
    }

    /// Parse a batch. The GIL is released while Rust parses (in parallel).
    /// An element with no address text gives `None`.
    fn parse_many(&self, py: Python<'_>, texts: Vec<String>) -> Vec<Option<PyParsedAddress>> {
        let parser = &self.inner;
        let results = py.detach(|| {
            let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
            parser.parse_many(&refs)
        });
        results.into_iter().map(|r| r.ok().map(wrap)).collect()
    }

    fn __repr__(&self) -> &'static str {
        "Parser()"
    }
}

/// Parse one address string (see `Parser.parse`).
#[pyfunction]
#[pyo3(signature = (text, default_state=None))]
fn parse(text: &str, default_state: Option<&str>) -> PyResult<PyParsedAddress> {
    parser_for(default_state)?
        .parse(text)
        .map(wrap)
        .map_err(|e| parse_err(&e))
}

/// Parse pre-split fields (see `Parser.parse_fields`).
#[pyfunction]
#[pyo3(signature = (line1=None, line2=None, city=None, state=None, zip=None, default_state=None))]
fn parse_fields(
    line1: Option<String>,
    line2: Option<String>,
    city: Option<String>,
    state: Option<String>,
    zip: Option<String>,
    default_state: Option<&str>,
) -> PyResult<PyParsedAddress> {
    parser_for(default_state)?
        .parse_fields(&Fields {
            line1,
            line2,
            city,
            state,
            zip,
        })
        .map(wrap)
        .map_err(|e| parse_err(&e))
}

/// Parse a batch (see `Parser.parse_many`).
#[pyfunction]
#[pyo3(signature = (texts, default_state=None))]
fn parse_many(
    py: Python<'_>,
    texts: Vec<String>,
    default_state: Option<&str>,
) -> PyResult<Vec<Option<PyParsedAddress>>> {
    let parser = PyParser {
        inner: parser_for(default_state)?,
    };
    Ok(parser.parse_many(py, texts))
}

/// The counterpart of `usaddress.tag()`: `(dict, address_type)`.
#[pyfunction]
#[pyo3(signature = (text, default_state=None))]
fn tag<'py>(
    py: Python<'py>,
    text: &str,
    default_state: Option<&str>,
) -> PyResult<(Bound<'py, PyDict>, &'static str)> {
    parse(text, default_state)?.tag(py)
}

/// The counterpart of `usaddress.parse()`: `[(token, label)]`.
#[pyfunction]
#[pyo3(signature = (text, default_state=None))]
fn parse_tokens(
    text: &str,
    default_state: Option<&str>,
) -> PyResult<Vec<(String, Option<&'static str>)>> {
    Ok(parse(text, default_state)?.to_usaddress_tags())
}

/// The kind of address in `text`: `street`, `po_box`, `rural_route`,
/// `military`, `general_delivery`, `dual` or `unknown`.
#[pyfunction]
#[pyo3(signature = (text, default_state=None))]
fn address_type(text: &str, default_state: Option<&str>) -> PyResult<&'static str> {
    Ok(parse(text, default_state)?.address_type())
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", doorplate::VERSION)?;
    m.add("ParseError", m.py().get_type::<ParseError>())?;
    m.add_class::<PyParser>()?;
    m.add_class::<PyParsedAddress>()?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(parse_fields, m)?)?;
    m.add_function(wrap_pyfunction!(parse_many, m)?)?;
    m.add_function(wrap_pyfunction!(tag, m)?)?;
    m.add_function(wrap_pyfunction!(parse_tokens, m)?)?;
    m.add_function(wrap_pyfunction!(address_type, m)?)?;
    Ok(())
}
