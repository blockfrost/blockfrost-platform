use serde_json::{Map, Value, json};

use crate::model::api::OgmiosError;

pub fn to_ogmios_error_v5(oe: &OgmiosError, reflection: &Value) -> Value {
    let code = oe.code;
    let name = get_error_name(code);

    use crate::model::api::EvaluationError::*;
    match &oe.data {
        Evaluation(value) => {
            let result = convert_error(value, code);

            json!({
                "type": "jsonwsp/response",
                "version": "1.0",
                "servicename": "ogmios",
                "methodname": "EvaluateTx",
                "result": result,
                "reflection": reflection
            })
        },
        Deserialization(data) => {
            if name == "DeserializationError" {
                json!({
                    "type": "jsonwsp/fault",
                    "version": "1.0",
                    "servicename": "ogmios",
                    "fault": {
                        "code": "client",
                        "string": data.conway,
                    },
                    "reflection": reflection
                })
            } else if name == "InvalidRequest" {
                invalid_request_v5(reflection)
            } else {
                json!({
                    "type": "jsonwsp/fault",
                    "version": "1.0",
                    "servicename": "ogmios",
                    "fault": {
                        "code": "client",
                        "string": oe.message,
                    },
                    "reflection": reflection
                })
            }
        },
    }
}

pub fn invalid_request_v5(reflection: &Value) -> Value {
    json!({
        "type": "jsonwsp/fault",
        "version": "1.0",
        "servicename": "ogmios",
        "fault": {
            "code": "client",
            "string": "Invalid request: failed to decode payload from base64 or base16.",
        },
        "reflection": reflection
    })
}

fn transform_utxo_in_error(utxos: Option<Vec<Value>>) -> Option<Vec<Value>> {
    utxos.map(|utxos| {
        utxos
            .into_iter()
            .map(|utxo| {
                let tx_id = utxo
                    .get("transaction")
                    .and_then(|transaction| transaction.get("id"))
                    .unwrap_or(&Value::Null);
                let index = utxo.get("index").unwrap_or(&Value::Null);

                json!({
                    "txId": tx_id,
                    "index": index,
                })
            })
            .collect()
    })
}

fn convert_error(error: &Value, code: i64) -> Value {
    // Incompatible era
    // Capitalize era name
    //
    if code == 3000
        && let Some(era_name) = error
            .get("data")
            .and_then(|d| d.get("incompatibleEra"))
            .and_then(|ie| ie.as_str())
    {
        return json!({
            "EvaluationFailure": {
                "IncompatibleEra": capitalize_words(era_name)
            }
        });
    }

    // v6 NodeTipTooOld -> v5 NotEnoughSynced
    // Capitalize era names
    //
    if code == 3003
        && let Some(era_data) = error.get("data").and_then(|ie| ie.as_object())
    {
        return json!({
            "EvaluationFailure": {
                "NotEnoughSynced": {
                    "minimumRequiredEra": capitalize_words(era_data["minimumRequiredEra"].as_str().unwrap_or("Unknown")),
                    "currentNodeEra": capitalize_words(era_data["currentNodeEra"].as_str().unwrap_or("Unknown")),
                }
            }
        });
    }

    if code == 3002
        && let Some(data_error) = error
            .get("overlappingOutputReferences")
            .and_then(|ie| ie.as_array())
    {
        let transformed_utxos = transform_utxo_in_error(Some(data_error.to_vec()));

        return json!({
            "EvaluationFailure": {
                "AdditionalUtxoOverlap": transformed_utxos
            }
        });
    }

    if let Some(errors) = error["data"].as_array() {
        return process_error_array(errors);
    }

    process_generic_error(error)
}

pub fn process_error_array(errors: &[Value]) -> Value {
    let mut script_failures: Map<String, Value> = Map::new();

    for err_obj in errors {
        let (key, error_code) = extract_key_and_error_code(err_obj);
        let err = &err_obj["error"]["message"]["data"];

        match error_code {
            3004 => handle_cannot_create_evaluation_context(err, &mut script_failures, &key),
            3011 => handle_missing_scripts(err, &mut script_failures, &key),
            3117 => handle_unknown_utxos(err, &mut script_failures, &key),
            3110 => handle_extraneous_redeemers(err, &mut script_failures, &key),
            3115 => handle_no_cost_models(err, &mut script_failures, &key),
            3111 => handle_missing_datums(err, &mut script_failures, &key),
            3013 => handle_unknown_input_referenced_by_redeemer(err, &mut script_failures, &key),
            _ => {},
        }
    }

    json!({
        "EvaluationFailure": {
            "ScriptFailures": script_failures
        }
    })
}

fn insert_script_failure(
    script_failures: &mut Map<String, Value>,
    key: &str,
    field: &str,
    value: Value,
) {
    let entry = script_failures
        .entry(key.to_string())
        .or_insert_with(|| json!({}));

    if let Some(obj) = entry.as_object_mut() {
        obj.insert(field.to_string(), value);
    }
}

fn extract_key_and_error_code(err: &Value) -> (String, i64) {
    let validator = &err["validator"];
    let error_code = err["error"]["message"]["code"].as_i64().unwrap_or_default();
    let validator_index = validator["index"].as_i64().unwrap_or_default();
    let validator_purpose = validator["purpose"].as_str().unwrap_or_default();
    let key = format!("{}:{}", validator_purpose, validator_index);

    (key, error_code)
}

fn convert_purpose(purpose: &str) -> &str {
    if purpose == "withdraw" {
        return "withdrawal";
    }

    if purpose == "publish" {
        return "certificate";
    }

    purpose
}

fn handle_cannot_create_evaluation_context(
    err: &Value,
    script_failures: &mut Map<String, Value>,
    key: &str,
) {
    if let Some(reason) = err["reason"].as_str() {
        insert_script_failure(
            script_failures,
            key,
            "CannotCreateEvaluationContext",
            json!({ "reason": reason }),
        );
    }
}

fn handle_unknown_input_referenced_by_redeemer(
    err: &Value,
    script_failures: &mut Map<String, Value>,
    key: &str,
) {
    if let Some(unsuitable_output_reference) = err["unsuitableOutputReference"].as_object() {
        let tx_id = unsuitable_output_reference
            .get("transaction")
            .and_then(|t| t["id"].as_str())
            .unwrap_or_default();

        let index = unsuitable_output_reference
            .get("index")
            .and_then(|i| i.as_i64())
            .unwrap_or_default();

        insert_script_failure(
            script_failures,
            key,
            "unknownInputReferencedByRedeemer",
            json!({ "txId": tx_id, "index": index }),
        );
    }
}

fn handle_missing_scripts(err: &Value, script_failures: &mut Map<String, Value>, key: &str) {
    if let Some(missing_scripts) = err["missingScripts"].as_array() {
        let missing_scripts_result: Vec<String> = missing_scripts
            .iter()
            .map(|script| {
                let index = script["index"].to_string();
                let mut purpose = script["purpose"].as_str().unwrap_or_default();
                purpose = convert_purpose(purpose);
                format!("{}:{}", purpose, index)
            })
            .collect();

        let entry = script_failures
            .entry(key.to_string())
            .or_insert_with(|| json!({}));

        if let Some(obj) = entry.as_object_mut() {
            let missing_entry = obj
                .entry("missingRequiredScripts")
                .or_insert_with(|| json!({"missing": []}));

            if let Some(missing_obj) = missing_entry.as_object_mut() {
                let missing_array = missing_obj.entry("missing").or_insert_with(|| json!([]));
                if let Some(array) = missing_array.as_array_mut() {
                    for script in missing_scripts_result {
                        array.push(json!(script));
                    }
                }
            }
        }
    }
}

fn handle_missing_datums(err: &Value, script_failures: &mut Map<String, Value>, key: &str) {
    if let Some(missing_datums) = err["missingDatums"].as_array() {
        let missing_datums_result: Vec<String> = missing_datums
            .iter()
            .filter_map(|datum| datum.as_str().map(String::from))
            .collect();

        let entry = script_failures
            .entry(key.to_string())
            .or_insert_with(|| json!({}));

        if let Some(obj) = entry.as_object_mut() {
            let missing_entry = obj
                .entry("missingRequiredDatums")
                .or_insert_with(|| json!({"missing": []}));

            if let Some(missing_obj) = missing_entry.as_object_mut() {
                let missing_array = missing_obj.entry("missing").or_insert_with(|| json!([]));

                if let Some(array) = missing_array.as_array_mut() {
                    for datum_hash in missing_datums_result {
                        array.push(json!(datum_hash));
                    }
                }
            }
        }
    }
}

fn handle_extraneous_redeemers(err: &Value, script_failures: &mut Map<String, Value>, key: &str) {
    if let Some(extraneous_redeemers) = err["extraneousRedeemers"].as_array() {
        let mut extra_redeemers: Vec<String> = Vec::new();

        for redeemer in extraneous_redeemers {
            let index = redeemer["index"].to_string();
            let purpose = redeemer["purpose"].as_str().unwrap_or_default();
            extra_redeemers.push(format!("{}:{}", purpose, index));
        }

        insert_script_failure(
            script_failures,
            key,
            "extraRedeemers",
            json!(extra_redeemers),
        );
    }
}

fn handle_no_cost_models(err: &Value, script_failures: &mut Map<String, Value>, key: &str) {
    if let Some(missing_cost_models) = err["missingCostModels"].as_array() {
        let missing_models: Vec<&str> = missing_cost_models
            .iter()
            .filter_map(|model| model.as_str())
            .collect();

        if !missing_models.is_empty() {
            insert_script_failure(
                script_failures,
                key,
                "noCostModelForLanguage",
                // Ogmios 5 schema expects a scalar language string for
                // "noCostModelForLanguage" not an array.
                // Currently pallas returns only one.
                json!(missing_models[0]),
            );
        }
    }
}

fn handle_unknown_utxos(err: &Value, script_failures: &mut Map<String, Value>, key: &str) {
    if let Some(unknown_refs) = err["unknownOutputReferences"].as_array() {
        let unknown_output_references: Vec<Value> = unknown_refs
            .iter()
            .map(|uref| {
                let tx_id = uref["transaction"]["id"].as_str().unwrap_or_default();
                let index = uref["index"].as_i64().unwrap_or_default();
                json!({
                    "txId": tx_id,
                    "index": index,
                })
            })
            .collect();

        let entry = script_failures
            .entry(key.to_string())
            .or_insert_with(|| json!({}));

        if let Some(obj) = entry.as_object_mut() {
            let existing_refs = obj
                .entry("UnknownOutputReference")
                .or_insert_with(|| json!([]));

            if let Some(array) = existing_refs.as_array_mut() {
                for ref_entry in unknown_output_references {
                    array.push(ref_entry);
                }
            }
        }
    }
}

fn process_generic_error(error: &Value) -> Value {
    let code = error["code"].as_i64().unwrap_or_default();
    let name = get_error_name(code);
    let message = error["message"].as_str().unwrap_or("Unknown");
    let reason = capitalize_words(
        error
            .get("data")
            .and_then(|d| d["reason"].as_str())
            .unwrap_or(message),
    );

    json!({
        "EvaluationFailure": {
            name.to_owned(): {
                "reason": reason
            }
        }
    })
}

fn get_error_name(code: i64) -> &'static str {
    match code {
        -32600 => "InvalidRequest",
        -32602 => "DeserializationError",
        3000 => "IncompatibleEra",
        3001 => "UnsupportedEra",
        3002 => "OverlappingAdditionalUtxo",
        3003 => "NodeTipTooOld",
        3004 => "CannotCreateEvaluationContext",
        3005 => "EraMismatch",
        3010 => "ScriptExecutionFailure",
        3011 => "InvalidRedeemerPointers",
        3012 => "ValidationFailure",
        3013 => "UnsuitableOutputReference",
        3100 => "InvalidSignatories",
        3101 => "MissingSignatories",
        3102 => "MissingScripts",
        3103 => "FailingNativeScript",
        3104 => "ExtraneousScripts",
        3105 => "MissingMetadataHash",
        3106 => "MissingMetadata",
        3107 => "MetadataHashMismatch",
        3108 => "InvalidMetadata",
        3109 => "MissingRedeemers",
        3110 => "ExtraneousRedeemers",
        3111 => "MissingDatums",
        3112 => "ExtraneousDatums",
        3113 => "ScriptIntegrityHashMismatch",
        3114 => "OrphanScriptInputs",
        3115 => "MissingCostModels",
        3116 => "MalformedScripts",
        3117 => "UnknownOutputReference",
        3118 => "OutsideOfValidityInterval",
        3119 => "TransactionTooLarge",
        3120 => "ValueTooLarge",
        3121 => "EmptyInputSet",
        3122 => "FeeTooSmall",
        3123 => "ValueNotConserved",
        3124 => "NetworkMismatch",
        3125 => "InsufficientlyFundedOutputs",
        3126 => "BootstrapAttributesTooLarge",
        3127 => "MintingOrBurningAda",
        3128 => "InsufficientCollateral",
        3129 => "CollateralLockedByScript",
        3130 => "UnforeseeableSlot",
        3131 => "TooManyCollateralInputs",
        3132 => "MissingCollateralInputs",
        3133 => "NonAdaCollateral",
        3134 => "ExecutionUnitsTooLarge",
        3135 => "TotalCollateralMismatch",
        3136 => "SpendsMismatch",
        3137 => "UnauthorizedVotes",
        3138 => "UnknownGovernanceProposals",
        3139 => "InvalidProtocolParametersUpdate",
        3140 => "UnknownStakePool",
        3141 => "IncompleteWithdrawals",
        3142 => "RetirementTooLate",
        3143 => "StakePoolCostTooLow",
        3144 => "MetadataHashTooLarge",
        3145 => "CredentialAlreadyRegistered",
        3146 => "UnknownCredential",
        3147 => "NonEmptyRewardAccount",
        3148 => "InvalidGenesisDelegation",
        3149 => "InvalidMIRTransfer",
        3150 => "ForbiddenWithdrawal",
        3151 => "CredentialDepositMismatch",
        3152 => "DRepAlreadyRegistered",
        3153 => "DRepNotRegistered",
        3154 => "UnknownConstitutionalCommitteeMember",
        3155 => "GovernanceProposalDepositMismatch",
        3156 => "ConflictingCommitteeUpdate",
        3157 => "InvalidCommitteeUpdate",
        3158 => "TreasuryWithdrawalMismatch",
        3159 => "InvalidOrMissingPreviousProposals",
        3160 => "VotingOnExpiredActions",
        3998 => "UnrecognizedCertificateType",
        3999 => "InternalLedgerTypeError",
        _ => "UnknownError",
    }
}

fn capitalize_words(message: &str) -> String {
    let words_to_capitalize = [
        "byron", "shelley", "allegra", "mary", "alonzo", "babbage", "conway",
    ];
    let mut result = Vec::new();

    for word in message.split_whitespace() {
        if words_to_capitalize.contains(&word.to_lowercase().as_str()) {
            let capitalized_word = word
                .char_indices()
                .map(|(i, c)| {
                    if i == 0 {
                        c.to_uppercase().to_string()
                    } else {
                        c.to_string()
                    }
                })
                .collect::<String>();
            result.push(capitalized_word);
        } else {
            result.push(word.to_string());
        }
    }

    result.join(" ")
}
