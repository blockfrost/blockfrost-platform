use crate::errors::BlockfrostError;

use bech32::{FromBase32, ToBase32, decode};
use cardano_serialization_lib::PublicKey;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Copy)]
pub enum PaymentCredPrefix {
    AddrVkh,
    AddrVk,
    Script,
    Invalid,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct PaymentCred {
    pub payload: Option<String>,
    pub prefix: PaymentCredPrefix,
}

impl PaymentCred {
    pub fn to_bytes(&self) -> Option<Vec<u8>> {
        self.payload.as_ref().and_then(|payment_cred| {
            let stripped = if payment_cred.starts_with("\\x") {
                payment_cred.strip_prefix("\\x")
            } else {
                Some(&payment_cred[..])
            };

            stripped.and_then(|s| hex::decode(s).ok())
        })
    }

    pub fn from_bech_32(address: &str) -> PaymentCred {
        let empty_result = PaymentCred {
            payload: None,
            prefix: PaymentCredPrefix::Invalid,
        };

        let decoded = decode(address);
        let (hrp, data, _) = match decoded {
            Ok(info) => info,
            Err(_) => return empty_result,
        };

        if hrp == "addr_vkh" {
            let payload = match Vec::<u8>::from_base32(&data) {
                Ok(payload) => payload,
                Err(_) => return empty_result,
            };

            let payment_cred = format!("\\x{}", hex::encode(payload));

            return PaymentCred {
                payload: Some(payment_cred),
                prefix: PaymentCredPrefix::AddrVkh,
            };
        }

        if hrp == "addr_vk" {
            let payload = match Vec::<u8>::from_base32(&data) {
                Ok(payload) => payload,
                Err(_) => return empty_result,
            };

            let pub_key = PublicKey::from_hex(&hex::encode(payload));

            match pub_key {
                Ok(pub_key) => {
                    let payment_cred = format!("\\x{}", pub_key.hash().to_hex());

                    return PaymentCred {
                        payload: Some(payment_cred),
                        prefix: PaymentCredPrefix::AddrVk,
                    };
                },
                Err(_) => return empty_result,
            }
        }

        if hrp == "script" {
            let payload = match Vec::<u8>::from_base32(&data) {
                Ok(payload) => payload,
                Err(_) => return empty_result,
            };

            let payment_cred = format!("\\x{}", hex::encode(payload));

            return PaymentCred {
                payload: Some(payment_cred),
                prefix: PaymentCredPrefix::Script,
            };
        }

        empty_result
    }

    pub fn to_bech_32(
        address: &String,
        payment_cred_prefix: PaymentCredPrefix,
    ) -> Result<String, BlockfrostError> {
        if hex::decode(address).is_err() {
            return Err(BlockfrostError::internal_server_error(
                "get_bech32_from_payment_cred cannot decode hex".to_string(),
            ));
        }

        match payment_cred_prefix {
            PaymentCredPrefix::AddrVkh | PaymentCredPrefix::Script => {
                let words = match hex::decode(address) {
                    Ok(bytes) => bytes.to_base32(),
                    err => {
                        return Err(BlockfrostError::internal_server_error(format!(
                            "get_bech32_from_payment_cred hex decode error: {err:?}"
                        )));
                    },
                };

                let prefix = match payment_cred_prefix {
                    PaymentCredPrefix::AddrVkh => "addr_vkh",
                    PaymentCredPrefix::Script => "script",
                    err => {
                        return Err(BlockfrostError::internal_server_error(format!(
                            "get_bech32_from_payment_cred hex decode error: {err:?}"
                        )));
                    },
                };

                let address = match bech32::encode(prefix, words, bech32::Variant::Bech32) {
                    Ok(address) => address,
                    err => {
                        return Err(BlockfrostError::internal_server_error(format!(
                            "get_bech32_from_payment_cred hex decode error: {err:?}"
                        )));
                    },
                };

                Ok(address)
            },
            err => Err(BlockfrostError::internal_server_error(format!(
                "get_bech32_from_payment_cred hex decode error: {err:?}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BlockfrostError, PaymentCred, PaymentCredPrefix};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case(
        "Valid paymentCred address (addr_vk)",
        "addr_vk1w0l2sr2zgfm26ztc6nl9xy8ghsk5sh6ldwemlpmp9xylzy4dtf7st80zhd",
        PaymentCred {
            payload: Some("\\x9493315cd92eb5d8c4304e67b7e16ae36d61d34502694657811a2c8e".to_string()),
            prefix: PaymentCredPrefix::AddrVk,
        }
    )]
    #[case(
        "Valid paymentCred address (addr_vkh)",
        "addr_vkh1c6xg0hkmmplr98yl08lrjpqlswmg636hnjpxvvp48th3zsq296f",
        PaymentCred {
            payload: Some("\\xc68c87dedbd87e329c9f79fe39041f83b68d47579c826630353aef11".to_string()),
            prefix: PaymentCredPrefix::AddrVkh,
        }
    )]
    #[case(
        "Valid paymentCred address",
        "addr_vkh1r3ej0ymkhacss5rge4qkw53lfp547wr92e9yjlwf9ynrsk5q93m",
        PaymentCred {
            payload: Some("\\x1c73279376bf71085068cd4167523f48695f3865564a497dc9292638".to_string()),
            prefix: PaymentCredPrefix::AddrVkh,
        }
    )]
    #[case(
        "Valid bech32 address (with script hash payment cred, type 7)",
        "addr1w8phkx6acpnf78fuvxn0mkew3l0fd058hzquvz7w36x4gtcyjy7wx",
        PaymentCred { payload: None, prefix: PaymentCredPrefix::Invalid },
    )]
    #[case(
        "Valid paymentCred address (script addr)",
        "script1cda3khwqv60360rp5m7akt50m6ttapacs8rqhn5w342z7r35m37",
        PaymentCred {
            payload: Some("\\xc37b1b5dc0669f1d3c61a6fddb2e8fde96be87b881c60bce8e8d542f".to_string()),
            prefix: PaymentCredPrefix::Script,
        }
    )]
    #[case(
        "Invalid paymentCred address",
        "addr_stonks",
        PaymentCred { payload: None, prefix: PaymentCredPrefix::Invalid },
    )]
    #[case(
        "Valid Bech32, but not paymentCred address",
        "addr1qyw8xfunw6lhzzzsdrx5ze6j8ayxjhecv4ty5jtaey5jvwquwvnexa4lwyy9q6xdg9n4y06gd90nse2kffyhmjffycuq405jv6",
        PaymentCred { payload: None, prefix: PaymentCredPrefix::Invalid },
    )]
    fn test_get_payment_cred_data(
        #[case] description: &str,
        #[case] input: &str,
        #[case] result: PaymentCred,
    ) {
        assert_eq!(PaymentCred::from_bech_32(input), result, "{}", description);
    }

    #[rstest]
    #[case(
        "Valid payment key hash address",
        "1c73279376bf71085068cd4167523f48695f3865564a497dc9292638".to_string(),
        PaymentCredPrefix::AddrVkh,
        Ok("addr_vkh1r3ej0ymkhacss5rge4qkw53lfp547wr92e9yjlwf9ynrsk5q93m".to_string())
    )]
    #[case(
        "Valid script address",
        "c37b1b5dc0669f1d3c61a6fddb2e8fde96be87b881c60bce8e8d542f".to_string(),
        PaymentCredPrefix::Script,
        Ok("script1cda3khwqv60360rp5m7akt50m6ttapacs8rqhn5w342z7r35m37".to_string())
    )]
    #[case(
    "Valid script address",
    "59a38a122f5278190f6f34230b7376eb8bebabf92f87240d2271e012".to_string(),
    PaymentCredPrefix::Script,
    Ok("script1tx3c5y302fupjrm0xs3skumkaw97h2le97rjgrfzw8spydzr5ej".to_string())
)]
    fn test_get_bech32_from_payment_cred(
        #[case] description: &str,
        #[case] address: String,
        #[case] prefix: PaymentCredPrefix,
        #[case] result: Result<String, BlockfrostError>,
    ) {
        assert_eq!(
            PaymentCred::to_bech_32(&address, prefix),
            result,
            "{}",
            description
        );
    }
}
