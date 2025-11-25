use bf_common::errors::BlockfrostError;
use pallas_hardano::display::haskell_error::as_cbor_decode_failure;
use pallas_primitives::{alonzo::Value, babbage::GenTransactionOutput, conway::Tx};
use tracing::info;

/// Checks if the given transaction is a valid CBOR-encoded transaction, by trying to decode it.
/// This function is used to validate the transaction before submitting it to the node.
/// If the transaction is invalid, returns the decoding error.
/// If the transaction is valid, returns Ok(()).
pub(crate) fn validate_tx_cbor(tx: &[u8]) -> Result<(), BlockfrostError> {
    match pallas_codec::minicbor::decode::<Tx>(tx) {
        Ok(decoded) => {
            if _check_multiasset_zero(decoded) {
                Err(BlockfrostError::custom_400(as_cbor_decode_failure(
                    "MultiAsset cannot contain zeros".to_string(),
                    0,
                )))
            } else {
                Ok(())
            }
        },
        Err(e) => {
            info!("Invalid TX CBOR: {:?}, CBOR: {}", e, hex::encode(tx));
            Err(BlockfrostError::custom_400(as_cbor_decode_failure(
                e.to_string(),
                e.position().unwrap_or(0) as u64,
            )))
        },
    }
}

/// A workaround to match the the ledger behaviour.
/// Checks if the transaction contains any multiasset outputs with zero amounts.
/// pallas decoding will not fail, but we need to fail for this case.
/// Initially we implemented this workaround in the pallas codabase.
/// See https://github.com/IntersectMBO/cardano-ledger/blob/49623962445143680dd725ebbf812c37e099b65c/eras/mary/impl/src/Cardano/Ledger/Mary/Value.hs#L328
fn _check_multiasset_zero(tx: Tx) -> bool {
    for output in tx.transaction_body.outputs.iter() {
        if let GenTransactionOutput::Legacy(o) = output {
            if let Value::Multiasset(_, kv) = &o.amount {
                for (_, assets) in kv.iter() {
                    for (_, amount) in assets.iter() {
                        if *amount == 0 {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_tx_cbor() {
        // Invalid CBOR bytes
        let invalid_tx = vec![0xFF, 0xFF];
        assert!(validate_tx_cbor(&invalid_tx).is_err());

        let invalid_tx = hex::decode("aaaaaa").unwrap();
        assert!(validate_tx_cbor(&invalid_tx).is_err());

        let invalid_tx = hex::decode("88").unwrap();
        assert!(validate_tx_cbor(&invalid_tx).is_err());

        let empty_tx = vec![];
        assert!(validate_tx_cbor(&empty_tx).is_err());

        let invalid_tx = hex::decode("").unwrap();
        assert!(validate_tx_cbor(&invalid_tx).is_err());

        let invalid_tx = hex::decode("84a800848258204c16d304e6d531c59afd87a9199b7bb4175bc131b3d6746917901046b662963c00825820893c3f630c0b2db16d041c388aa0d58746ccbbc44133b2d7a3127a72c79722f1018258201e8f017df70bda0b4d129a66ab5297557920928c0261ed78fce16cf347430657028258208380ce7240ba59187f6450911f74a70cf3d2749228badb2e7cd10fb6499355f503018482581d61e15900a9a62a8fb01f936a25bf54af209c7ed1248c4e5abd05ec4e76821a0023ba63a1581ca0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c235a145484f534b5900a300581d71cba5c6770fe7b30ebc1fa32f01938c150513211360ded23ac76e36b301821a006336d5a3581c239075b83c03c2333eacd0b0beac6b8314f11ce3dc0c047012b0cad4a144706f6f6c01581c3547b4325e495d529619335603ababde10025dceafa9ed34b1fb6611a158208b284793d3bd4967244a2ddd68410d56d06d36ac8d201429b937096a2e8234bc1b7ffffffffffade6b581ca0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c235a145484f534b59195e99028201d818583ad8799fd8799f4040ffd8799f581ca0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c23545484f534b59ff1a006336d5195e99ff825839016d06090559d8ed2988aa5b2fff265d668cf552f4f62278c0128f816c0a48432e080280d0d9b15edb65563995f97ce236035afea568e660d1821a00118f32a1581c2f8b2d1f384485896f38406173fa11df2a4ce53b4b0886138b76597aa1476261746368657201825839016d06090559d8ed2988aa5b2fff265d668cf552f4f62278c0128f816c0a48432e080280d0d9b15edb65563995f97ce236035afea568e660d11a06d9f713021a000ab9e00b582027f17979d848d6472896266dd8bf39f7251ca23798713464bc407bf637286c230d81825820cf5de9189b958f8ad64c1f1837c2fa4711d073494598467a1c1a59589393eae20310825839016d06090559d8ed2988aa5b2fff265d668cf552f4f62278c0128f816c0a48432e080280d0d9b15edb65563995f97ce236035afea568e660d11a08666c75111a001016d01282825820bf93dc59c10c19c35210c2414779d7391ca19128cc7b13794ea85af5ff835f59008258201c37df764f8261edce8678b197767668a91d544b2b203fb5d0cf9acc10366e7600a200818258200eabfa083d7969681d2fc8e825a5f79e1c40f03aeac46ecd94bf5c5790db1bc05840a84fa0a9dd9547776502e4f54ab02e549277bc37332d3b9bbf4afa5afd8e33f042cdb92a0044ba9dbb8814f99960944ffa3c2c68fef9c738855ec1f67bf675010582840001d8799fd8799f011a006336d5195e991b7ffffffffffade6bd8799f1a000539e7ff01ffff821a000b46e41a0a7f3ca4840003d87d80821a002dccfe1a28868be8f5f6").unwrap();
        assert!(
            validate_tx_cbor(&invalid_tx).is_err(),
            "multiasset with zeros should fail"
        );

        // A very basic transaction encoded as CBOR
        let valid_tx = hex::decode("84a300d90102818258205176274bef11d575edd6aa72392aaf993a07f736e70239c1fb22d4b1426b22bc01018282583900ddf1eb9ce2a1561e8f156991486b97873fb6969190cbc99ddcb3816621dcb03574152623414ed354d2d8f50e310f3f2e7d167cb20e5754271a003d09008258390099a5cb0fa8f19aba38cacf8a243d632149129f882df3a8e67f6bd512bcb0cde66a545e9fbc7ca4492f39bca1f4f265cc1503b4f7d6ff205c1b000000024f127a7c021a0002a2ada100d90102818258208b83e59abc9d7a66a77be5e0825525546a595174f8b929f164fcf5052d7aab7b5840709c64556c946abf267edd90b8027343d065193ef816529d8fa7aa2243f1fd2ec27036a677974199e2264cb582d01925134b9a20997d5a734da298df957eb002f5f6").unwrap();
        assert!(validate_tx_cbor(&valid_tx).is_ok());
    }
}
