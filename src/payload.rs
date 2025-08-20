use crate::errors::APIError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Debug, Deserialize)]
pub struct Payload {
    pub mode: String,
    pub port: i32,
    pub secret: String,
    pub reward_address: String,
    pub api_prefix: Uuid,
}

impl Payload {
    pub fn validate(&self) -> Result<(), APIError> {
        // Validate mode
        if self.mode.is_empty() {
            return Err(APIError::Validation("Mode cannot be empty".to_string()));
        }

        if !["compact", "light", "full"].contains(&self.mode.as_str()) {
            return Err(APIError::Validation(
                "Mode must be one of 'compact', 'light', or 'full'".to_string(),
            ));
        }

        // Validate port
        if self.port <= 0 || self.port > 65535 {
            return Err(APIError::Validation(
                "Port must be between 1 and 65535".to_string(),
            ));
        }

        // Validate secret
        if self.secret.len() < 8 {
            return Err(APIError::Validation(
                "Secret must be at least 8 characters long".to_string(),
            ));
        }

        // Validate reward_address
        if self.reward_address.is_empty() {
            return Err(APIError::Validation("reward_address is empty".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};
    use uuid::Uuid;

    #[fixture]
    fn valid_payload() -> Payload {
        Payload {
            mode: "compact".to_string(),
            port: 3000,
            secret: "123456789".to_string(),
            reward_address: "addr_test1qq....".to_string(),
            api_prefix: Uuid::new_v4(),
        }
    }

    fn assert_validation_err_contains(res: Result<(), APIError>, expected: &str) {
        match res {
            Err(APIError::Validation(msg)) => assert!(
                msg.contains(expected),
                "Expected error to contain: `{expected}`, got: `{msg}`"
            ),
            Ok(()) => panic!("Expected APIError::Validation, got Ok(())"),
            _ => panic!("Expected APIError::Validation variant"),
        }
    }

    #[rstest]
    fn valid_payload_passes(valid_payload: Payload) {
        assert!(valid_payload.validate().is_ok());
    }

    #[rstest]
    fn empty_mode_fails(mut valid_payload: Payload) {
        valid_payload.mode.clear();
        assert_validation_err_contains(valid_payload.validate(), "Mode cannot be empty");
    }

    #[rstest]
    #[case("COMPACT")]
    #[case("fast")]
    #[case("slow")]
    #[case("lightweight")]
    fn invalid_mode_fails(mut valid_payload: Payload, #[case] mode: &str) {
        valid_payload.mode = mode.into();

        assert_validation_err_contains(
            valid_payload.validate(),
            "Mode must be one of 'compact', 'light', or 'full'",
        );
    }

    #[rstest]
    #[case(1)]
    #[case(65535)]
    fn port_bounds_ok(mut valid_payload: Payload, #[case] port: i32) {
        valid_payload.port = port;

        assert!(valid_payload.validate().is_ok());
    }

    #[rstest]
    #[case(0)]
    #[case(65_536)]
    #[case(-1)]
    fn port_out_of_range_fails(mut valid_payload: Payload, #[case] port: i32) {
        valid_payload.port = port;

        assert_validation_err_contains(valid_payload.validate(), "Port must be between 1 and 65535");
    }

    #[rstest]
    #[case("1234567", true)] // 7 -> error
    #[case("12345678", false)] // 8 -> ok
    fn secret_min_length(mut valid_payload: Payload, #[case] secret: &str, #[case] expect_err: bool) {
        valid_payload.secret = secret.into();

        let res = valid_payload.validate();

        if expect_err {
            assert_validation_err_contains(res, "Secret must be at least 8 characters long");
        } else {
            assert!(res.is_ok());
        }
    }

    #[rstest]
    fn reward_address_cannot_be_empty(mut valid_payload: Payload) {
        valid_payload.reward_address.clear();

        assert_validation_err_contains(valid_payload.validate(), "reward_address is empty");
    }

    #[rstest]
    #[case("compact")]
    #[case("light")]
    #[case("full")]
    fn accepts_all_allowed_modes(mut valid_payload: Payload, #[case] mode: &str) {
        valid_payload.mode = mode.into();

        assert!(
            valid_payload.validate().is_ok(),
            "Mode '{mode}' should be accepted"
        );
    }
}
