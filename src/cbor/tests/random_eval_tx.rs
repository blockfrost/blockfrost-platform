use super::*;

// -------------------------- random evaluation tests -------------------------- //

#[test]
#[allow(non_snake_case)]
fn proptest_eval_Tx_Conway_size_001() {
    proptest_with_params(CaseType::Tx_Conway, 100, 10, None)
}

// -------------------------- random UTxO decoder tests -------------------------- //

#[test]
fn proptest_utxo_decoder() {
    check_generated_cases(CaseType::Tx_Conway, 500, 10, 5, None, |case| {
        let json: TestCaseJson = serde_json::from_value(case.json).unwrap();
        let utxo_cbor = json.utxo_set_cbor;
        let utxo_decoded = utxo_decoder::decode_utxo(&hex::decode(&utxo_cbor).unwrap());
        if utxo_decoded.is_ok() {
            Ok(())
        } else {
            Err(utxo_cbor)
        }
    })
}

// -------------------------- helper functions -------------------------- //

/// Tests the native Rust deserializer with the given params.
fn proptest_with_params(
    case_type: CaseType,
    num_cases: u32,
    generator_size: u16,
    seed: Option<u64>,
) {
    use crate::api::utils::txs::evaluate::model::AdditionalUtxoSet;

    check_generated_cases(case_type, num_cases, generator_size, 5, seed, |case| {
        let tx_cbor = case.cbor.clone();
        let json: TestCaseJson = serde_json::from_value(case.json).unwrap();
        let expected = json.execution_units;
        let utxo_cbor = json.utxo_set_cbor;
        let utxo: AdditionalUtxoSet =
            utxo_decoder::decode_utxo(&hex::decode(&utxo_cbor).unwrap()).unwrap();

        // TODO: okay, so now we have `tx_cbor`, `AdditionalUtxoSet`, and the `expected` JSON.
        // TODO: it’s time to call `crate::cbor::evaluate_tx()`, but it needs a `cardano-node` 👀

        todo!()
    })
}

#[derive(Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct TestCaseJson {
    pub execution_units: serde_json::Value,
    #[serde(rename = "utxoSetCBOR")]
    pub utxo_set_cbor: String,
}

// -------------------------- draft UTxO decoder (could be wrong!) -------------------------- //

mod utxo_decoder {
    use crate::api::utils::txs::evaluate::model::{
        AdditionalUtxoSet, Script, ScriptNative, TxIn, TxOut, Value,
    };
    use pallas_codec::minicbor;

    use anyhow::{Result, anyhow, bail};
    use bech32::{ToBase32, Variant, encode};
    use minicbor::{data::Type as CborType, decode::Decoder};
    use std::collections::HashMap;

    pub fn decode_utxo(bytes: &[u8]) -> Result<AdditionalUtxoSet> {
        let mut d = Decoder::new(bytes);

        let map_len = d
            .map()?
            .ok_or_else(|| anyhow!("UTxO map must be definite"))?;
        let mut out = Vec::with_capacity(map_len as usize);

        for _ in 0..map_len {
            let tx_in = decode_txin(&mut d)?;
            let tx_out = decode_txout(&mut d, bytes)?;
            out.push((tx_in, tx_out));
        }
        Ok(out)
    }

    fn decode_txin(d: &mut Decoder<'_>) -> Result<TxIn> {
        match d.array()? {
            Some(2) => {},
            Some(n) => bail!("TxIn array length {n} ≠ 2"),
            None => bail!("TxIn must be definite-length"),
        }

        let txid = d.bytes()?;
        if txid.len() != 32 {
            bail!("TxId len {} ≠ 32", txid.len());
        }
        let idx = d.u64()?;

        Ok(TxIn {
            tx_id: hex::encode(txid),
            index: idx,
        })
    }

    fn decode_txout<'b>(d: &mut Decoder<'b>, src: &'b [u8]) -> Result<TxOut> {
        match d.datatype()? {
            CborType::Map => decode_txout_map(d, src),
            CborType::Array => decode_txout_array(d, src),
            other => bail!("TxOut must be map or array, got {:?}", other),
        }
    }

    fn decode_txout_map<'b>(d: &mut Decoder<'b>, src: &'b [u8]) -> Result<TxOut> {
        let pairs = d
            .map()?
            .ok_or_else(|| anyhow!("TxOut map must be definite"))?;

        let mut addr_bytes = None;
        let mut value = None;
        let mut datum_hash = None;
        let mut datum = None;
        let mut script = None;

        for _ in 0..pairs {
            let key = d.u64()?;
            match key {
                0 => {
                    let bytes = d.bytes()?;
                    addr_bytes = Some(bytes.to_vec());
                },

                1 => value = Some(decode_value(d)?),

                2 => {
                    let start = d.position();
                    match d.datatype()? {
                        CborType::Bytes => {
                            let bs = d.bytes()?;
                            if bs.len() == 32 {
                                datum_hash = Some(hex::encode(bs));
                            } else {
                                datum = Some(hex::encode(bs));
                            }
                        },
                        _ => {
                            let raw = read_term(d, src, start)?;
                            datum = Some(hex::encode(&raw));
                        },
                    }
                },

                3 => {
                    let start = d.position();
                    let raw = read_term(d, src, start)?;
                    script = Some(parse_script(&raw)?);
                },

                _ => {
                    d.skip()?;
                },
            }
        }

        let addr_bytes = addr_bytes.ok_or_else(|| anyhow!("TxOut missing address"))?;
        let value = value.ok_or_else(|| anyhow!("TxOut missing value"))?;

        let hrp = if addr_bytes.first().map_or(false, |b| b & 0b0001_0000 == 0) {
            "addr"
        } else {
            "addr_test"
        };
        let address = encode(hrp, addr_bytes.to_base32(), Variant::Bech32)?;

        Ok(TxOut {
            address,
            value,
            datum_hash,
            datum,
            script,
        })
    }

    fn decode_txout_array<'b>(d: &mut Decoder<'b>, src: &'b [u8]) -> Result<TxOut> {
        let len = d
            .array()?
            .ok_or_else(|| anyhow!("TxOut must be definite"))?;
        if !(2..=4).contains(&len) {
            bail!("unexpected TxOut length {len}");
        }

        let addr_bytes = d.bytes()?;
        let hrp = if addr_bytes.first().map_or(false, |b| b & 0b0001_0000 == 0) {
            "addr"
        } else {
            "addr_test"
        };
        let address = encode(hrp, addr_bytes.to_base32(), Variant::Bech32)?;

        let value = decode_value(d)?;

        let mut datum_hash = None;
        let mut datum = None;
        let mut script = None;

        match len {
            2 => {},
            3 => {
                let dh = d.bytes()?;
                if dh.len() != 32 {
                    bail!("datumHash len {} ≠ 32", dh.len());
                }
                datum_hash = Some(hex::encode(dh));
            },
            4 => {
                let start = d.position();
                match d.datatype()? {
                    CborType::Bytes => {
                        let bs = d.bytes()?;
                        if bs.len() == 32 {
                            datum_hash = Some(hex::encode(bs));
                        } else {
                            datum = Some(hex::encode(bs));
                        }
                    },
                    _ => {
                        let bytes = read_term(d, src, start)?;
                        datum = Some(hex::encode(&bytes));
                    },
                }
                let start = d.position();
                let raw = read_term(d, src, start)?;
                script = Some(parse_script(&raw)?);
            },
            _ => unreachable!(),
        }

        Ok(TxOut {
            address,
            value,
            datum_hash,
            datum,
            script,
        })
    }

    fn decode_value(d: &mut Decoder<'_>) -> Result<Value> {
        match d.datatype()? {
            CborType::U8 | CborType::U16 | CborType::U32 | CborType::U64 => Ok(Value {
                coins: d.u64()?,
                assets: None,
            }),

            CborType::Map => {
                let outer = d
                    .map()?
                    .ok_or_else(|| anyhow!("multi-asset must be definite"))?;
                let mut assets = HashMap::<String, u64>::new();
                let mut coin = 0u64;

                for _ in 0..outer {
                    match d.datatype()? {
                        CborType::U8 | CborType::U16 | CborType::U32 | CborType::U64 => {
                            if d.u64()? != 0 {
                                bail!("unexpected integer key in value map");
                            }
                            coin = d.u64()?;
                        },
                        CborType::Bytes => {
                            let policy = d.bytes()?.to_vec();
                            let inner = d
                                .map()?
                                .ok_or_else(|| anyhow!("asset map must be definite"))?;
                            for _ in 0..inner {
                                let name = d.bytes()?.to_vec();
                                let qty = d.i64()?;
                                if qty < 0 {
                                    bail!("burning not allowed in Conway value");
                                }
                                let key =
                                    format!("{}.{}", hex::encode(&policy), hex::encode(&name));
                                assets.insert(key, qty as u64);
                            }
                        },
                        t => bail!("unexpected CBOR type {:?} in value map", t),
                    }
                }

                Ok(Value {
                    coins: coin,
                    assets: if assets.is_empty() {
                        None
                    } else {
                        Some(assets)
                    },
                })
            },

            t => bail!("unexpected CBOR type {:?} for TxOut value", t),
        }
    }

    fn peel_tags(input: &[u8]) -> Result<&[u8]> {
        let mut d = Decoder::new(input);

        while let CborType::Tag = d.datatype()? {
            let _t = d.tag()?; // ignore tag number
        }

        let start = d.position();
        Ok(&input[start..])
    }

    fn parse_script(bytes: &[u8]) -> Result<Script> {
        let payload = peel_tags(bytes)?;

        let mut d = Decoder::new(payload);

        if matches!(d.datatype()?, CborType::Bytes) {
            return parse_script(d.bytes()?);
        }

        if matches!(d.datatype()?, CborType::Array) {
            let len = d
                .array()?
                .ok_or_else(|| anyhow!("script array indefinite"))?;
            if len == 0 {
                bail!("empty script array");
            }

            let tag = d.u64()?; // first element

            if (1..=3).contains(&tag) && len == 2 {
                let start = d.position(); // 2nd element
                d.skip()?; // walk over it
                let end = d.position();
                let hex = hex::encode(&payload[start..end]);
                return Ok(match tag {
                    1 => Script::PlutusV1(hex),
                    2 => Script::PlutusV2(hex),
                    3 => Script::PlutusV3(hex),
                    _ => unreachable!(),
                });
            }

            return Ok(Script::Native(parse_native_array(payload)?));
        }

        if matches!(d.datatype()?, CborType::Map) {
            return Ok(Script::Native(parse_native_map(payload)?));
        }

        bail!("reference script must be array, map, or Tag-24-wrapped bytes")
    }

    fn parse_native_array(bytes: &[u8]) -> Result<ScriptNative> {
        let mut d = Decoder::new(bytes);
        let len = d
            .array()?
            .ok_or_else(|| anyhow!("native array indefinite"))?;

        if len == 0 {
            return Ok(ScriptNative::All(Vec::new()));
        }

        let first_ty = d.datatype()?;
        if !matches!(
            first_ty,
            CborType::U8 | CborType::U16 | CborType::U32 | CborType::U64
        ) {
            let subs = gather_subscripts(len as usize, &mut d, bytes)?;
            return Ok(ScriptNative::All(subs));
        }

        let tag = d.u64()?;
        match tag {
            0 => match d.datatype()? {
                CborType::Bytes => {
                    return Ok(ScriptNative::String(hex::encode(d.bytes()?)));
                },

                _ => {
                    let subs = gather_subscripts((len - 1) as usize, &mut d, bytes)?;
                    return Ok(ScriptNative::Any(subs));
                },
            },

            1 => {
                let subs = gather_subscripts((len - 1) as usize, &mut d, bytes)?;
                Ok(ScriptNative::All(subs))
            },

            2 => {
                let subs = gather_subscripts((len - 1) as usize, &mut d, bytes)?;
                Ok(ScriptNative::Any(subs))
            },

            3 => {
                let start_tail = d.position(); // decoder is already at 1st payload item

                for _ in 0..(len - 1) {
                    d.skip()?;
                }
                let end_tail = d.position();

                let tail = &bytes[start_tail..end_tail];

                let mut d2 = Decoder::new(tail);
                let mut subs = Vec::<ScriptNative>::new();
                let mut last_int = None;

                while d2.position() < tail.len() {
                    match d2.datatype()? {
                        CborType::U8 | CborType::U16 | CborType::U32 | CborType::U64 => {
                            last_int = Some(d2.u64()?);
                        },

                        CborType::Array => {
                            let start = d2.position();
                            d2.skip()?;
                            let end = d2.position();
                            let slice = &tail[start..end];
                            subs.push(
                                parse_native_array(slice).or_else(|_| parse_native_map(slice))?,
                            );
                        },

                        CborType::Map => {
                            let start = d2.position();
                            d2.skip()?;
                            let end = d2.position();
                            subs.push(parse_native_map(&tail[start..end])?);
                        },

                        other => bail!("unexpected CBOR type {:?} inside N-of payload", other),
                    }
                }

                let n = last_int.ok_or_else(|| anyhow!("N-of payload missing threshold"))? as u32;
                return Ok(ScriptNative::NOf(n, subs));
            },

            4 | 5 => {
                let slot = match d.datatype()? {
                    CborType::U8 | CborType::U16 | CborType::U32 | CborType::U64 => d.u64()?,
                    CborType::Array => {
                        // unwrap single-element array
                        let alen = d.array()?.ok_or_else(|| anyhow!("slot array indefinite"))?;
                        if alen != 1 {
                            bail!("slot array len {} ≠ 1", alen);
                        }
                        d.u64()?
                    },
                    other => bail!("slot must be integer or single-item array, got {:?}", other),
                };
                return Ok(if tag == 4 {
                    ScriptNative::StartsAt(slot)
                } else {
                    ScriptNative::ExpiresAt(slot)
                });
            },

            _ => bail!("unknown or malformed native script tag {tag}"),
        }
    }

    fn parse_native_map(bytes: &[u8]) -> Result<ScriptNative> {
        let mut d = Decoder::new(bytes);
        let len = d.map()?.ok_or_else(|| anyhow!("native map indefinite"))?;
        if len != 1 {
            bail!("native map must have exactly 1 pair, got {}", len);
        }

        let tag = d.u64()?;
        match tag {
            0 => Ok(ScriptNative::String(hex::encode(d.bytes()?))),

            1 => {
                let subs = gather_subscripts_from_cbor(&mut d, bytes)?;
                Ok(ScriptNative::All(subs))
            },

            2 => {
                let subs = gather_subscripts_from_cbor(&mut d, bytes)?;
                Ok(ScriptNative::Any(subs))
            },

            3 => {
                let n = d.u64()? as u32;
                let subs = gather_subscripts_from_cbor(&mut d, bytes)?;
                Ok(ScriptNative::NOf(n, subs))
            },

            4 => Ok(ScriptNative::StartsAt(d.u64()?)),
            5 => Ok(ScriptNative::ExpiresAt(d.u64()?)),
            _ => bail!("unknown native map tag {tag}"),
        }
    }

    fn gather_subscripts(
        count: usize,
        d: &mut Decoder<'_>,
        src: &[u8],
    ) -> Result<Vec<ScriptNative>> {
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            let start = d.position();
            d.skip()?;
            let end = d.position();
            let slice = &src[start..end];

            match Decoder::new(slice).datatype()? {
                CborType::Array => out.push(parse_native_array(slice)?),
                CborType::Map => out.push(parse_native_map(slice)?),
                other => bail!("unexpected CBOR type {:?} in sub-script", other),
            }
        }
        Ok(out)
    }

    fn gather_subscripts_from_cbor(d: &mut Decoder<'_>, src: &[u8]) -> Result<Vec<ScriptNative>> {
        let remaining = d
            .array()?
            .ok_or_else(|| anyhow!("sub-script list indefinite"))?;
        gather_subscripts(remaining as usize, d, src)
    }

    fn read_term<'b>(d: &mut Decoder<'b>, src: &'b [u8], start: usize) -> Result<Vec<u8>> {
        d.skip()?;
        let end = d.position();
        Ok(src[start..end].to_vec())
    }
}
