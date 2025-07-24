use crate::AppError;
use crate::cbor::external::testgen::{self, Testgen};

#[derive(Clone)]
pub struct FallbackDecoder {
    testgen: Testgen,
}
impl FallbackDecoder {
    pub fn spawn() -> Result<Self, AppError> {
        let testgen = testgen::Testgen::spawn("deserialize-stream")
            .map_err(|err| AppError::Server(format!("Failed to spawn FallbackDecoder: {err}")))?;

        Ok(Self { testgen })
    }

    pub async fn decode(&self, input: &[u8]) -> Result<serde_json::Value, String> {
        self.testgen.decode(input).await
    }

    /// This function is called at startup, so that we make sure that the worker is reasonable.
    pub async fn startup_sanity_test(&self) -> Result<(), String> {
        let input = hex::decode("8182068182028200a0").map_err(|err| err.to_string())?;
        let result = self.testgen.decode(&input).await;
        let expected = serde_json::json!({
          "contents": {
            "contents": {
              "contents": {
                "era": "ShelleyBasedEraConway",
                "error": [
                  "ConwayCertsFailure (WithdrawalsNotInRewardsCERTS (fromList []))"
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
                "FallbackDecoder: startup_sanity_test failed: {result:?}"
            ))
        }
    }

    #[cfg(test)]
    /// A single global [`FallbackDecoder`] that you can cheaply use in tests.
    pub fn instance() -> Self {
        GLOBAL_INSTANCE.clone()
    }
}

#[cfg(test)]
static GLOBAL_INSTANCE: std::sync::LazyLock<FallbackDecoder> =
    std::sync::LazyLock::new(|| FallbackDecoder::spawn().expect("Failed to spawn FallbackDecoder"));

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "tarpaulin"))]
    use super::*;
    #[tokio::test]
    //#[tracing_test::traced_test]
    #[cfg(not(feature = "tarpaulin"))]
    async fn test_fallback_decoder() {
        let decoder = FallbackDecoder::spawn().unwrap();

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
