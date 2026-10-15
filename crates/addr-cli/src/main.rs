//! `addr` command-line tool.
//!
//! ```text
//! addr parse "<address>" [--pretty] [--usps]
//! addr parse --line1 X [--line2 X] [--city X] [--state X] [--zip X]
//! addr parse            (no argument or "-": one address per stdin line, JSONL out)
//! addr parse-file IN.csv --column NAME [-o OUT.csv]
//! ```
//!
//! This tool parses and formats; it does not validate deliverability.

mod json;

use addr_core::{Fields, ParseError, ParsedAddress, Parser};
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

const USAGE: &str = "usage:
  addr parse \"<address>\" [--pretty] [--usps]
  addr parse --line1 X [--line2 X] [--city X] [--state X] [--zip X] [--pretty] [--usps]
  addr parse [-]                 read one address per line from stdin
  addr parse-file IN.csv --column NAME [--line2 NAME] [--city NAME] [--state NAME]
                  [--zip NAME] [-o OUT.csv]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("parse") => parse_cmd(&args[1..]),
        Some("parse-file") => parse_file_cmd(&args[1..]),
        _ => Err(CliError::Usage(String::new())),
    };
    match result {
        Ok(code) => code,
        Err(CliError::Usage(msg)) => {
            if !msg.is_empty() {
                eprintln!("error: {msg}");
            }
            eprintln!("{USAGE}");
            ExitCode::from(1)
        }
        Err(CliError::Failed(msg)) => {
            eprintln!("error: {msg}");
            ExitCode::from(2)
        }
    }
}

enum CliError {
    /// Bad command line: exit 1, with the usage text.
    Usage(String),
    /// The input could not be used: exit 2.
    Failed(String),
}

impl From<io::Error> for CliError {
    fn from(e: io::Error) -> Self {
        CliError::Failed(e.to_string())
    }
}

/// `--flag value` options, plus positional arguments.
struct Opts {
    positional: Vec<String>,
    values: Vec<(String, String)>,
    switches: Vec<String>,
}

impl Opts {
    fn parse(args: &[String], with_value: &[&str], switches: &[&str]) -> Result<Self, CliError> {
        let mut o = Opts {
            positional: Vec::new(),
            values: Vec::new(),
            switches: Vec::new(),
        };
        let mut it = args.iter();
        while let Some(a) = it.next() {
            if with_value.contains(&a.as_str()) {
                let v = it
                    .next()
                    .ok_or_else(|| CliError::Usage(format!("{a} needs a value")))?;
                o.values.push((a.clone(), v.clone()));
            } else if switches.contains(&a.as_str()) {
                o.switches.push(a.clone());
            } else if a.starts_with("--") || (a.starts_with('-') && a.len() > 1) {
                return Err(CliError::Usage(format!("unknown option {a}")));
            } else {
                o.positional.push(a.clone());
            }
        }
        Ok(o)
    }

    fn value(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    fn has(&self, name: &str) -> bool {
        self.switches.iter().any(|s| s == name)
    }
}

fn parse_cmd(args: &[String]) -> Result<ExitCode, CliError> {
    let o = Opts::parse(
        args,
        &["--line1", "--line2", "--city", "--state", "--zip"],
        &["--pretty", "--usps"],
    )?;
    let parser = Parser::new();
    let fields = Fields {
        line1: o.value("--line1").map(str::to_string),
        line2: o.value("--line2").map(str::to_string),
        city: o.value("--city").map(str::to_string),
        state: o.value("--state").map(str::to_string),
        zip: o.value("--zip").map(str::to_string),
    };
    let have_fields = !o.values.is_empty();
    let out = io::stdout();
    let mut out = out.lock();
    if have_fields {
        if !o.positional.is_empty() {
            return Err(CliError::Usage(
                "give an address or field options, not both".into(),
            ));
        }
        return print_one(&mut out, parser.parse_fields(&fields), &o);
    }
    match o.positional.as_slice() {
        [one] if one != "-" => print_one(&mut out, parser.parse(one), &o),
        [] | [_] => {
            // Stdin, one address per line. Blank lines are reported on
            // stderr and skipped, so the output lines stay in step with
            // the non-blank input lines.
            let mut worst = ExitCode::SUCCESS;
            for (n, line) in io::stdin().lock().lines().enumerate() {
                let line = line?;
                match parser.parse(&line) {
                    Ok(a) => print_parsed(&mut out, &a, &o)?,
                    Err(e) => {
                        eprintln!("line {}: {e}", n + 1);
                        worst = ExitCode::from(2);
                    }
                }
            }
            Ok(worst)
        }
        _ => Err(CliError::Usage("give one address (quote it)".into())),
    }
}

fn print_one(
    out: &mut impl Write,
    r: Result<ParsedAddress, ParseError>,
    o: &Opts,
) -> Result<ExitCode, CliError> {
    let a = r.map_err(|e| CliError::Failed(e.to_string()))?;
    print_parsed(out, &a, o)?;
    Ok(ExitCode::SUCCESS)
}

fn print_parsed(out: &mut impl Write, a: &ParsedAddress, o: &Opts) -> io::Result<()> {
    if o.has("--usps") {
        for line in a.to_usps().lines() {
            writeln!(out, "{line}")?;
        }
        return Ok(());
    }
    let v = json::record(a);
    let text = if o.has("--pretty") {
        serde_json::to_string_pretty(&v)
    } else {
        serde_json::to_string(&v)
    };
    writeln!(out, "{}", text.map_err(io::Error::other)?)
}

fn parse_file_cmd(args: &[String]) -> Result<ExitCode, CliError> {
    let o = Opts::parse(
        args,
        &["--column", "--line2", "--city", "--state", "--zip", "-o"],
        &[],
    )?;
    let [input] = o.positional.as_slice() else {
        return Err(CliError::Usage("give one input CSV file".into()));
    };
    let column = o
        .value("--column")
        .ok_or_else(|| CliError::Usage("--column NAME is required".into()))?;
    let mut reader = csv::Reader::from_path(input).map_err(|e| CliError::Failed(e.to_string()))?;
    let headers = reader
        .headers()
        .map_err(|e| CliError::Failed(e.to_string()))?
        .clone();
    let index = |opt: &str, name: &str| -> Result<usize, CliError> {
        headers
            .iter()
            .position(|h| h == name)
            .ok_or_else(|| CliError::Failed(format!("{opt}: no column named {name:?} in {input}")))
    };
    let line1 = index("--column", column)?;
    let extra = |opt: &str| o.value(opt).map(|n| index(opt, n)).transpose();
    let (line2, city, state, zip) = (
        extra("--line2")?,
        extra("--city")?,
        extra("--state")?,
        extra("--zip")?,
    );
    let multi = line2.is_some() || city.is_some() || state.is_some() || zip.is_some();

    let sink: Box<dyn Write> = match o.value("-o") {
        Some(path) => Box::new(File::create(path)?),
        None => Box::new(io::stdout()),
    };
    let mut writer = csv::Writer::from_writer(sink);
    let mut head: Vec<&str> = headers.iter().collect();
    // Prefixed so a new column never shares a name with one of the input's.
    let added: Vec<String> = json::COLUMNS.iter().map(|c| format!("addr_{c}")).collect();
    head.extend(added.iter().map(String::as_str));
    writer
        .write_record(&head)
        .map_err(|e| CliError::Failed(e.to_string()))?;

    let parser = Parser::new();
    let mut empty_rows = 0usize;
    for rec in reader.records() {
        let rec = rec.map_err(|e| CliError::Failed(e.to_string()))?;
        let cell = |i: Option<usize>| i.and_then(|i| rec.get(i)).map(str::to_string);
        let parsed = if multi {
            parser.parse_fields(&Fields {
                line1: cell(Some(line1)),
                line2: cell(line2),
                city: cell(city),
                state: cell(state),
                zip: cell(zip),
            })
        } else {
            parser.parse(rec.get(line1).unwrap_or(""))
        };
        let mut row: Vec<String> = rec.iter().map(str::to_string).collect();
        match parsed {
            Ok(a) => row.extend(json::row(&a)),
            Err(_) => {
                // Unusable input: keep the row, leave the new columns empty.
                empty_rows += 1;
                row.extend(json::COLUMNS.iter().map(|_| String::new()));
            }
        }
        writer
            .write_record(&row)
            .map_err(|e| CliError::Failed(e.to_string()))?;
    }
    writer.flush()?;
    if empty_rows > 0 {
        eprintln!("{empty_rows} row(s) had no address text; their new columns are empty");
    }
    Ok(ExitCode::SUCCESS)
}
