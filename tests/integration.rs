use std::path::PathBuf;

use dasl::drisl::Value as DrislValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct TestResult {
    pass: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TestCase {
    #[serde(rename = "type")]
    test_type: String,
    data: String,
    name: String,
}

const FIXTURE_PATH: &str = "./3rd-party/dasl-testing/fixtures/cbor/";

#[test]
fn test_cid() {
    let results = run_test_group("cid.json");

    process_results(results, &[]);
}

#[test]
fn test_concat() {
    let results = run_test_group("concat.json");

    process_results(results, &[]);
}
#[test]
fn test_floats() {
    let results = run_test_group("floats.json");

    process_results(results, &["float reduction"]);
}

#[test]
fn test_indefinite() {
    let results = run_test_group("indefinite.json");

    process_results(results, &[]);
}

#[test]
fn test_integer_range() {
    let results = run_test_group("integer_range.json");

    process_results(results, &[]);
}

#[test]
fn test_map_keys() {
    let results = run_test_group("map_keys.json");

    process_results(results, &[]);
}

#[test]
fn test_numeric_reduction() {
    let results = run_test_group("numeric_reduction.json");

    process_results(results, &[]);
}

#[test]
fn test_recursion() {
    let results = run_test_group("recursion.json");

    process_results(results, &[]);
}

#[test]
fn test_short_form() {
    let results = run_test_group("short_form.json");

    process_results(results, &[]);
}

#[test]
fn test_simple() {
    let results = run_test_group("simple.json");

    process_results(results, &[]);
}

#[test]
fn test_tags() {
    let results = run_test_group("tags.json");

    process_results(results, &[]);
}

#[test]
fn test_utf8() {
    let results = run_test_group("utf8.json");

    process_results(results, &[]);
}

fn process_results(results: Vec<(TestResult, TestCase)>, skip_list: &[&str]) {
    let mut num_passed = 0;
    let mut num_total = 0;
    for (result, case) in results {
        let to_skip = skip_list.contains(&case.name.as_str());
        let emoji = if result.pass {
            "✅"
        } else if to_skip {
            "⛔️"
        } else {
            "❌"
        };
        println!("{}  {} ({})", emoji, case.name, case.test_type);

        if to_skip {
            continue;
        }
        num_total += 1;

        if result.pass {
            num_passed += 1;
        } else {
            if let Some(output) = result.output {
                println!("  Output: {output}");
            }
            if let Some(err) = result.error {
                println!("  Error: {err}");
            }
        }
        // assert!(
        //     result.pass,
        //     "{group_name}:\n  {:?}\n  {:?}",
        //     result.output, result.error
        // );
    }
    assert_eq!(num_passed, num_total, "not all cases passed");
}

fn run_test_group(name: &str) -> Vec<(TestResult, TestCase)> {
    let path = PathBuf::from(FIXTURE_PATH).join(name);
    let file = std::fs::read(path).expect("invalid file");
    let tests: Vec<TestCase> = serde_json::from_slice(&file).expect("invalid json");
    run_tests(tests)
}

fn run_tests(tests: Vec<TestCase>) -> Vec<(TestResult, TestCase)> {
    let mut results = Vec::with_capacity(tests.len());

    for test in tests {
        println!("{} ({})", test.name, test.test_type);
        let result = run_test(&test);
        results.push((result, test));
    }
    results
}

fn run_test(test: &TestCase) -> TestResult {
    let test_data = hex::decode(&test.data).expect("invalid test data");

    match test.test_type.as_str() {
        "roundtrip" => match roundtrip(&test_data) {
            Ok(output) => {
                if test_data == output {
                    TestResult {
                        pass: true,
                        output: None,
                        error: None,
                    }
                } else {
                    TestResult {
                        pass: false,
                        output: Some(hex::encode(output)),
                        error: Some(format!("expected: {}", hex::encode(test_data))),
                    }
                }
            }
            Err(err) => TestResult {
                pass: false,
                output: None,
                error: Some(err),
            },
        },
        "invalid_in" => {
            let (failed, info) = invalid_decode(&test_data);
            if failed {
                TestResult {
                    pass: true,
                    output: None,
                    error: Some(info),
                }
            } else {
                TestResult {
                    pass: false,
                    output: None,
                    error: None,
                }
            }
        }
        "invalid_out" => {
            let (failed, info) = invalid_encode(&test_data);
            if failed {
                TestResult {
                    pass: true,
                    output: None,
                    error: Some(info),
                }
            } else {
                TestResult {
                    pass: false,
                    output: None,
                    error: None,
                }
            }
        }
        _ => panic!("unknown test type '{}'", test.test_type),
    }
}

fn roundtrip(b: &[u8]) -> Result<Vec<u8>, String> {
    let obj: DrislValue =
        dasl::drisl::from_slice(b).map_err(|e| format!("DRISL decode error: {e}"))?;

    let output = dasl::drisl::to_vec(&obj).map_err(|e| format!("DRISL encode error: {e}"))?;

    Ok(output)
}

fn invalid_decode(b: &[u8]) -> (bool, String) {
    let result: Result<DrislValue, _> =
        dasl::drisl::from_slice(b).map_err(|e| format!("DRISL decode error: {e}"));

    match dbg!(result) {
        Ok(_) => (false, String::new()),
        Err(e) => (true, e.to_string()),
    }
}

fn invalid_encode(b: &[u8]) -> (bool, String) {
    let obj: ciborium::Value = ciborium::from_reader(std::io::Cursor::new(b))
        .expect("general CBOR library failed to decode test input");

    let drisl_obj = match cbor_value_to_drisl(obj) {
        Ok(obj) => obj,
        Err(e) => return (true, e),
    };
    match dasl::drisl::to_vec(&drisl_obj) {
        Ok(_) => (false, String::new()),
        Err(e) => (true, e.to_string()),
    }
}

fn cbor_value_to_drisl(value: ciborium::Value) -> Result<DrislValue, String> {
    match value {
        ciborium::Value::Integer(i) => Ok(DrislValue::Integer(i.into())),
        ciborium::Value::Bytes(b) => Ok(DrislValue::Bytes(b)),
        ciborium::Value::Float(f) => Ok(DrislValue::Float(f)),
        ciborium::Value::Text(s) => Ok(DrislValue::Text(s)),
        ciborium::Value::Bool(b) => Ok(DrislValue::Bool(b)),
        ciborium::Value::Null => Ok(DrislValue::Null),
        ciborium::Value::Array(arr) => {
            let mut drisl_list = Vec::new();
            for item in arr {
                drisl_list.push(cbor_value_to_drisl(item)?);
            }
            Ok(DrislValue::Array(drisl_list))
        }
        ciborium::Value::Map(map) => {
            let mut drisl_map = std::collections::BTreeMap::new();
            for (k, v) in map {
                if let ciborium::Value::Text(key) = k {
                    drisl_map.insert(key, cbor_value_to_drisl(v)?);
                } else {
                    return Err(format!("Map keys must be strings, found: {k:?}"));
                }
            }
            Ok(DrislValue::Map(drisl_map))
        }
        _ => Err(format!("Unsupported CBOR type: {value:?}")),
    }
}
