//! Assembly (pipeline step 5, item 14): tagged tokens to a
//! [`ParsedAddress`] with spans and warnings.
//!
//! Runs of tokens with the same tag form one component; lexicon values are
//! looked up here from the joined text, so the tagger only decides *what*
//! each token is. Spelling choices that Pub 28 does not list
//! (`MatchKind::Observed`) raise `NonStandardAbbreviation`.

use crate::classify::{is_ordinal, join, zip_parts};
use crate::lexicon::{Lexicons, MatchKind, Special};
use crate::tagger::{Tag, Tagged};
use crate::tokenize::Token;
use crate::types::{
    Component, ComponentSpan, Highway, MilitaryBox, ParsedAddress, RouteKind, RuralRoute, Span,
    Warning,
};

/// The state of the address before it is assembled: the tagged state, else
/// the main place's state for the tagged ZIP. A state pack reads some
/// spellings differently (`SR` is STATE ROAD in Florida).
fn address_state(runs: &[(Tag, &[Token])], lex: &Lexicons) -> Option<&'static str> {
    let text_of = |tag: Tag| {
        runs.iter()
            .find(|(t, _)| *t == tag)
            .map(|(_, run)| join(run))
    };
    if let Some(l) = text_of(Tag::State).and_then(|t| lex.state(&t)) {
        return Some(l.value.code());
    }
    let text = text_of(Tag::Zip)?;
    let (z5, _) = zip_parts(&text)?;
    Some(lex.zip_places(z5)?.main().state())
}

/// Original text for a span (single-line input or one field).
pub(crate) type SourceText<'a> = dyn Fn(&Span) -> Option<&'a str> + 'a;

pub(crate) fn assemble(
    addr: &mut ParsedAddress,
    toks: &[Token],
    tagged: Tagged,
    lex: &Lexicons,
    source: &SourceText<'_>,
) {
    let Tagged { tags, warnings } = tagged;
    for w in warnings {
        addr.diagnostics.warn(w);
    }

    // Runs of equal tags.
    let mut runs: Vec<(Tag, &[Token])> = Vec::new();
    let mut start = 0;
    for k in 1..=toks.len() {
        if k == toks.len() || tags[k] != tags[start] {
            if let Some(run) = toks.get(start..k) {
                runs.push((tags[start], run));
            }
            start = k;
        }
    }

    let state = address_state(&runs, lex);
    let mut unparsed: Vec<String> = Vec::new();
    let mut highway_state = None;
    let mut highway_prefix = None;
    let mut route_kind = None;
    let mut military_kind = None;
    // `BOX 2877 PSC 103`: a box number before its military line.
    let mut military_box = None;
    let mut suffix_text = None;
    for (tag, run) in runs {
        let text = join(run);
        let span = run_span(run);
        let record = |addr: &mut ParsedAddress, c: Component| {
            if let Some(s) = span {
                addr.source.spans.push(ComponentSpan {
                    component: c,
                    span: s,
                });
            }
        };
        match tag {
            Tag::Untagged | Tag::Unparsed => {
                for t in run {
                    unparsed.push(source(&t.span).map_or_else(|| t.text.clone(), str::to_string));
                    addr.source.spans.push(ComponentSpan {
                        component: Component::Unparsed,
                        span: t.span,
                    });
                }
            }
            Tag::Skip => {}
            Tag::StreetNumber => {
                set_once(&mut addr.street_number, text);
                record(addr, Component::StreetNumber);
            }
            Tag::PreDir => {
                if let Some(l) = lex.directional(&text) {
                    addr.street_dir_prefix.get_or_insert(l.value);
                    record(addr, Component::StreetDirPrefix);
                }
            }
            Tag::Name => {
                set_once(&mut addr.street_name, text);
                record(addr, Component::StreetName);
            }
            Tag::Suffix => {
                if let Some(l) = lex.suffix(&text) {
                    observed(addr, l.kind);
                    if addr.street_suffix.is_none() {
                        addr.street_suffix = Some(l.value);
                        suffix_text = Some(text);
                    }
                    record(addr, Component::StreetSuffix);
                }
            }
            Tag::PostDir => {
                if let Some(l) = lex.directional(&text) {
                    addr.street_dir_suffix.get_or_insert(l.value);
                    record(addr, Component::StreetDirSuffix);
                }
            }
            Tag::HighwayState => {
                highway_state = lex.state(&text).map(|l| l.value);
                record(addr, Component::Highway);
            }
            Tag::HighwayPrefix => {
                // A state pack may add prefix spellings (`SR` in Florida).
                let l = match state {
                    Some(st) => lex.highway_in(st, &text),
                    None => lex.highway(&text),
                };
                if let Some(l) = &l {
                    observed(addr, l.kind);
                }
                highway_prefix = l.map(|l| l.value);
                record(addr, Component::Highway);
            }
            Tag::HighwayRoute => {
                let state = highway_state.take();
                // Appendix F: a state with only a route is a HIGHWAY
                // (`KENTUCKY 440` is `KY HIGHWAY 440`).
                let prefix = highway_prefix
                    .take()
                    .or_else(|| state.and_then(|_| lex.highway("HIGHWAY")).map(|l| l.value));
                if let Some(prefix) = prefix {
                    addr.highway.get_or_insert(Highway {
                        state,
                        prefix,
                        route: text,
                    });
                    record(addr, Component::Highway);
                }
            }
            Tag::UnitType(n) => {
                if let Some(l) = lex.unit(&text) {
                    observed(addr, l.kind);
                    if n == 0 {
                        addr.unit_type.get_or_insert(l.value);
                        record(addr, Component::UnitType);
                    } else {
                        addr.second_unit_type.get_or_insert(l.value);
                        record(addr, Component::SecondUnitType);
                    }
                }
            }
            Tag::UnitNumber(n) => {
                // A penthouse after a designator is one value: `APT PH 5213`
                // is `APT PH5213` (design.md decision 31).
                let text = match run {
                    [first, rest @ ..]
                        if !rest.is_empty()
                            && lex
                                .unit(&first.text)
                                .is_some_and(|u| u.value.abbr() == "PH") =>
                    {
                        run.iter().map(|t| t.text.as_str()).collect()
                    }
                    _ => text,
                };
                if n == 0 {
                    set_once(&mut addr.unit_number, text);
                    record(addr, Component::UnitNumber);
                } else {
                    set_once(&mut addr.second_unit_number, text);
                    record(addr, Component::SecondUnitNumber);
                }
            }
            Tag::PoBox => record(addr, Component::PoBox),
            Tag::Attention => {
                set_once(&mut addr.attention, text);
                record(addr, Component::Attention);
            }
            Tag::Building => {
                set_once(&mut addr.building, text);
                record(addr, Component::Building);
            }
            Tag::PoBoxNumber => {
                set_once(&mut addr.po_box, text);
                record(addr, Component::PoBox);
            }
            Tag::Route => {
                route_kind = lex.special(&text).and_then(|l| match l.value {
                    Special::RuralRoute => Some(RouteKind::Rural),
                    Special::HighwayContract => Some(RouteKind::HighwayContract),
                    _ => None,
                });
                record(addr, Component::RuralRoute);
            }
            Tag::RouteNumber => {
                if let Some(kind) = route_kind.take() {
                    addr.rural_route.get_or_insert(RuralRoute {
                        kind,
                        route: text,
                        box_number: None,
                    });
                    record(addr, Component::RuralRoute);
                }
            }
            Tag::BoxNumber => {
                if let Some(r) = &mut addr.rural_route {
                    r.box_number.get_or_insert(text);
                    record(addr, Component::RuralRoute);
                }
            }
            Tag::GeneralDelivery => {
                addr.general_delivery = true;
                record(addr, Component::GeneralDelivery);
            }
            Tag::MilitaryKind => {
                military_kind = lex.military(&text).map(|l| l.value);
                record(addr, Component::Military);
            }
            Tag::MilitaryNumber => {
                if let Some(kind) = military_kind.take() {
                    addr.military.get_or_insert(MilitaryBox {
                        kind,
                        number: text,
                        box_number: military_box.take(),
                    });
                    record(addr, Component::Military);
                }
            }
            Tag::MilitaryBoxNumber => {
                if let Some(m) = &mut addr.military {
                    m.box_number.get_or_insert(text);
                } else {
                    military_box = Some(text);
                }
                record(addr, Component::Military);
            }
            Tag::MilitaryPostOffice => {
                if let Some(l) = lex.military_post_office(&text) {
                    addr.military_post_office.get_or_insert(l.value);
                    record(addr, Component::MilitaryPostOffice);
                }
            }
            Tag::City => {
                set_once(&mut addr.city, text);
                record(addr, Component::City);
            }
            Tag::State => {
                if let Some(l) = lex.state(&text) {
                    addr.state.get_or_insert(l.value);
                    record(addr, Component::State);
                }
            }
            Tag::Zip => {
                if let Some((z5, z4)) = zip_parts(&text) {
                    addr.zip5.get_or_insert_with(|| z5.to_string());
                    record(addr, Component::Zip5);
                    if let Some(z4) = z4 {
                        addr.zip4.get_or_insert_with(|| z4.to_string());
                        record(addr, Component::Zip4);
                    }
                }
            }
        }
    }

    if !unparsed.is_empty() {
        addr.diagnostics.unparsed = Some(unparsed.join(" "));
    }
    // A state pack may read the suffix spelling differently (`CR` is
    // CIRCLE in Florida). The state written on the address wins; with none,
    // the ZIP's.
    if let Some(text) = suffix_text {
        let state = addr.state.map(|s| s.code()).or_else(|| {
            let z = lex.zip_places(addr.zip5.as_deref()?)?;
            Some(z.main().state())
        });
        if let Some(l) = state.and_then(|st| lex.suffix_in(st, &text)) {
            addr.street_suffix = Some(l.value);
        }
    }
    // A floor is a number (Appendix C2: FL takes a range): `5TH FLOOR` is
    // FL 5.
    for (t, n) in [
        (addr.unit_type, &mut addr.unit_number),
        (addr.second_unit_type, &mut addr.second_unit_number),
    ] {
        if let (Some(t), Some(v)) = (t, n.as_mut()) {
            if t.abbr() == "FL" && is_ordinal(v) {
                v.truncate(v.trim_end_matches(|c: char| c.is_ascii_alphabetic()).len());
            }
        }
    }
    if addr.city.is_none() && addr.military_post_office.is_none() {
        addr.diagnostics.warn(Warning::MissingCity);
    }
    if addr.state.is_none() && !addr.diagnostics.has(&Warning::UnknownState) {
        addr.diagnostics.warn(Warning::MissingState);
    }
    if addr.zip5.is_none() && !addr.diagnostics.has(&Warning::InvalidZip) {
        addr.diagnostics.warn(Warning::MissingZip);
    }
}

fn set_once(slot: &mut Option<String>, value: String) {
    if slot.is_none() {
        *slot = Some(value);
    }
}

fn observed(addr: &mut ParsedAddress, kind: MatchKind) {
    if kind == MatchKind::Observed {
        addr.diagnostics.warn(Warning::NonStandardAbbreviation);
    }
}

/// One span over a run of tokens from the same field.
fn run_span(run: &[Token]) -> Option<Span> {
    let first = run.first()?;
    let last = run.last()?;
    (first.span.field == last.span.field).then_some(Span {
        field: first.span.field,
        start: first.span.start,
        end: last.span.end,
    })
}
