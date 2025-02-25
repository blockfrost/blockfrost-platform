use serde_json::Value;

/// This workaround exists because the final error is a Haskell string
/// which we don't want to bother de-serializing from string.
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

    fn get_response_struct(response: &[u8]) -> TestResponse {
        let as_value = serde_json::from_slice::<serde_json::Value>(response).unwrap();

        TestResponse {
            message: serde_json::from_str(as_value.get("message").unwrap().as_str().unwrap())
                .unwrap(),
            error: as_value.get("error").unwrap().to_string(),
            status_code: as_value.get("status_code").unwrap().as_u64().unwrap(),
        }
    }

    assert_eq!(
        get_response_struct(bf_response),
        get_response_struct(local_response)
    );
}
