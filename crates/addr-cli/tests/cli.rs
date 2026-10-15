//! Runs the built `addr` binary.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::process::{Command, Stdio};

fn addr(args: &[&str], stdin: &str) -> (String, String, i32) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_addr"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn addr");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn parse_prints_one_json_line() {
    let (out, _, code) = addr(&["parse", "123 N Main St Apt 4, Miami, FL 33101"], "");
    assert_eq!(code, 0);
    assert_eq!(out.lines().count(), 1);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["components"]["street_name"], "MAIN");
    assert_eq!(v["components"]["unit_type"], "APT");
    assert_eq!(v["usps"]["line1"], "123 N MAIN ST APT 4");
    assert_eq!(v["usps"]["last_line"], "MIAMI FL 33101");
    assert_eq!(v["key"], "k1:S|123|N|MAIN|ST||FL|33101");
    assert_eq!(v["reso"]["City"], "MIAMI");
    assert_eq!(v["warnings"], serde_json::json!([]));
}

#[test]
fn fields_and_usps_output() {
    let (out, _, code) = addr(
        &[
            "parse",
            "--line1",
            "413 OLD CORRY FIELD RD B 32506",
            "--line2",
            "PENSACOLA, FL- 32506",
            "--usps",
        ],
        "",
    );
    assert_eq!(code, 0);
    assert_eq!(out, "413 OLD CORRY FIELD RD # B\nPENSACOLA FL 32506\n");
}

#[test]
fn stdin_gives_one_json_line_per_address_and_reports_blank_lines() {
    let (out, err, code) = addr(&["parse"], "1 MAIN ST\n\n2 ELM AVE\n");
    assert_eq!(out.lines().count(), 2);
    assert!(err.contains("line 2"), "{err}");
    assert_eq!(code, 2);
}

#[test]
fn empty_input_and_bad_usage_use_distinct_exit_codes() {
    assert_eq!(addr(&["parse", "  "], "").2, 2);
    assert_eq!(addr(&["parse", "--bogus"], "").2, 1);
    assert_eq!(addr(&[], "").2, 1);
}

#[test]
fn parse_file_appends_prefixed_columns() {
    let dir = std::env::temp_dir().join(format!("addr-cli-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let input = dir.join("in.csv");
    std::fs::write(&input, "id,address,city\n1,12 OAK ST #4,TAMPA\n2,,X\n").expect("write");
    let input = input.to_str().expect("utf-8 path");
    let (out, _, code) = addr(
        &["parse-file", input, "--column", "address", "--city", "city"],
        "",
    );
    assert_eq!(code, 0);
    let mut lines = out.lines();
    let header = lines.next().expect("header");
    assert!(
        header.starts_with("id,address,city,addr_street_number,"),
        "{header}"
    );
    let row = lines.next().expect("row");
    assert!(row.starts_with("1,12 OAK ST #4,TAMPA,12,"), "{row}");
    assert_eq!(lines.count(), 1);
    std::fs::remove_dir_all(&dir).ok();
}
