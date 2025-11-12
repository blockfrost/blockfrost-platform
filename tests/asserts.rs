use serde_json::Value;

/// This workaround exists because the final error is a Haskell string
/// which we don't want to bother de-serializing from string.
#[allow(dead_code)]
pub fn assert_submit_error_responses(bf_response: &[u8], local_response: &[u8]) {
    #[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Eq)]
    #[serde(untagged)]
    pub enum TestData {
        Wrapper {
            contents: Box<TestData>,
            tag: String,
        },
        Data {
            contents: Value,
        },
    }

    #[derive(Debug, PartialEq, Eq)]
    pub struct TestResponse {
        message: TestData,
        error: String,
        status_code: u64,
    }

    fn sort_all_error_arrays(value: &mut Value) {
        match value {
            Value::Object(map) => {
                for (key, val) in map.iter_mut() {
                    if key == "error" {
                        if let Value::Array(arr) = val {
                            arr.sort_by_key(|v| v.to_string());
                        }
                    }
                    sort_all_error_arrays(val);
                }
            },
            Value::Array(arr) => {
                for val in arr {
                    sort_all_error_arrays(val);
                }
            },
            _ => {},
        }
    }

    fn sort_error_array_in_message_json(raw: &str) -> String {
        let mut message_value: Value = serde_json::from_str(raw).unwrap();
        sort_all_error_arrays(&mut message_value);
        serde_json::to_string(&message_value).unwrap()
    }

    fn get_response_struct(response: &[u8]) -> TestResponse {
        let as_value = serde_json::from_slice::<Value>(response).unwrap();

        let raw_message = as_value.get("message").unwrap().as_str().unwrap();
        let sorted_message_json = sort_error_array_in_message_json(raw_message);

        TestResponse {
            message: serde_json::from_str(&sorted_message_json).unwrap(),
            error: as_value.get("error").unwrap().to_string(),
            status_code: as_value.get("status_code").unwrap().as_u64().unwrap(),
        }
    }

    assert_eq!(
        get_response_struct(bf_response),
        get_response_struct(local_response)
    );
}
