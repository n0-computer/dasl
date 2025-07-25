use std::collections::HashMap;

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
}

#[test]
fn test_dasl_testing() -> Result<(), Box<dyn std::error::Error>> {
    let mut results = HashMap::new();

    let fixtures_path = "./3rd-party/dasl-testing/fixtures/cbor/";
    let entries = std::fs::read_dir(fixtures_path)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if let Some(extension) = path.extension() {
            if extension == "json" {
                let content = std::fs::read_to_string(&path)?;
                let tests: Vec<TestCase> = serde_json::from_str(&content)?;

                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    let res = run_tests(tests);
                    results.insert(file_name.to_string(), res);
                }
            }
        }
    }

    for (group_name, cases) in results {
        println!("# {group_name}");
        for (result, case) in cases {
            println!("  {}", case.test_type);
            assert!(
                result.pass,
                "{group_name}:\n  {:?}\n  {:?}",
                result.output, result.error
            );
        }
    }

    Ok(())
}

fn run_tests(tests: Vec<TestCase>) -> Vec<(TestResult, TestCase)> {
    let mut results = Vec::with_capacity(tests.len());

    for test in tests {
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
                        error: None,
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

    match result {
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
            let mut ipld_list = Vec::new();
            for item in arr {
                ipld_list.push(cbor_value_to_drisl(item)?);
            }
            Ok(DrislValue::Array(ipld_list))
        }
        ciborium::Value::Map(map) => {
            let mut ipld_map = std::collections::BTreeMap::new();
            for (k, v) in map {
                if let ciborium::Value::Text(key) = k {
                    ipld_map.insert(key, cbor_value_to_drisl(v)?);
                } else {
                    return Err(format!("Map keys must be strings, found: {k:?}"));
                }
            }
            Ok(DrislValue::Map(ipld_map))
        }
        _ => Err(format!("Unsupported CBOR type: {value:?}")),
    }
}
