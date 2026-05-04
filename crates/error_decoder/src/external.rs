use bf_common::errors::AppError;
use bf_testgen::testgen::{Testgen, TestgenResponse, Variant};

#[derive(Clone)]
pub struct ExternalDecoder {
    testgen: Testgen,
}
impl ExternalDecoder {
    pub fn spawn() -> Result<Self, AppError> {
        let testgen = Testgen::spawn(Variant::DeserializeStream)
            .map_err(|err| AppError::Server(format!("Failed to spawn ExternalDecoder: {err}")))?;

        Ok(Self { testgen })
    }

    pub async fn decode(&self, input: &[u8]) -> Result<serde_json::Value, String> {
        match self.testgen.decode(input).await {
            Ok(resp) => match resp {
                TestgenResponse::Ok(value) => Ok(value),
                TestgenResponse::Err(err) => Err(err.to_string()),
            },
            Err(err) => Err(err),
        }
    }

    /// This function is called at startup, so that we make sure that the worker is reasonable.
    pub async fn startup_sanity_test(&self) -> Result<(), String> {
        let input = hex::decode("8182068182028200a0").map_err(|err| err.to_string())?;
        let result = self.decode(&input).await;
        let expected = serde_json::json!({
          "contents": {
            "contents": {
              "contents": {
                "era": "ShelleyBasedEraConway",
                "error": [
                  "ConwayCertsFailure (WithdrawalsNotInRewardsCERTS (Withdrawals {unWithdrawals = fromList []}))"
                ],
                "kind": "ShelleyTxValidationError"
              },
              "tag": "TxValidationErrorInCardanoMode"
            },
            "tag": "TxCmdTxSubmitValidationError"
          },
          "tag": "TxSubmitFail"
        });

        if result == Ok(expected) {
            Ok(())
        } else {
            Err(format!(
                "ExternalDecoder: startup_sanity_test failed: {result:?}"
            ))
        }
    }

    /// A single global [`ExternalDecoder`] that you can cheaply use in tests.
    #[cfg(all(test, not(feature = "tarpaulin")))]
    pub fn instance() -> Self {
        GLOBAL_INSTANCE.clone()
    }
}

#[cfg(all(test, not(feature = "tarpaulin")))]
static GLOBAL_INSTANCE: std::sync::LazyLock<ExternalDecoder> =
    std::sync::LazyLock::new(|| ExternalDecoder::spawn().expect("Failed to spawn ExternalDecoder"));

#[cfg(test)]
mod tests {
    // The CBOR test cases are covered by `crates/error_decoder/src/tests/specific.rs`,
    // which already cross-validates the Rust (pallas-hardano) implementation against the
    // Haskell (testgen-hs) external decoder. This module only contains tests that are
    // unique to the external decoder itself (e.g. crash-recovery behaviour).
    #[cfg(not(feature = "tarpaulin"))]
    use super::*;

    #[tokio::test]
    //#[tracing_test::traced_test]
    #[cfg(not(feature = "tarpaulin"))]
    async fn test_sanity() {
        let decoder = ExternalDecoder::spawn().unwrap();

        // Wait for it to come up:
        decoder.startup_sanity_test().await.unwrap();

        // Now, kill our child to test the restart logic:
        sysinfo::System::new_all()
            .process(sysinfo::Pid::from_u32(decoder.testgen.child_pid().unwrap()))
            .unwrap()
            .kill();

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let input = hex::decode("8182068183051a000c275b1a000b35ec").unwrap();
        let result = decoder.decode(&input).await;

        assert_eq!(
            result,
            Ok(serde_json::json!({"contents":
                 {"contents":
                  {"contents":
                   {"era": "ShelleyBasedEraConway", "error":
                    ["ConwayTreasuryValueMismatch (Mismatch {mismatchSupplied = Coin 734700, mismatchExpected = Coin 796507})"],
                    "kind": "ShelleyTxValidationError"
                    },
                    "tag": "TxValidationErrorInCardanoMode"
                }, "tag": "TxCmdTxSubmitValidationError"
            },
                "tag": "TxSubmitFail"
                }
            ))
        );
    }
}
