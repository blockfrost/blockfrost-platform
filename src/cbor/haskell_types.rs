#![allow(dead_code)]

use std::fmt;

use pallas::ledger::addresses::StakeKeyHash;
use pallas_codec::minicbor;
use pallas_codec::minicbor::Decode;
use pallas_codec::utils::Bytes;
use pallas_primitives::{
    byron::{TxIn, TxOut},
    conway::{DatumHash, ExUnits, RewardAccount, ScriptHash, VKeyWitness, Value},
    Coin,
};
use serde::Serialize;
use serde_with::SerializeDisplay;
use std::fmt::Display;

/// This file contains the types that are mapped from the Haskell codebase.
/// The main reason these mappings exist is to mimick the error responses from the cardano-submit-api
/// and generate identical responses to the Blockfrost.io /tx/submit API (https://docs.blockfrost.io/#tag/cardano--transactions/POST/tx/submit)
///
/// To mimick, we need to:
/// - Decode the CBOR error reasons (pallas doesn't do it) from the cardano-node
/// - Generate the same JSON response structure as the cardano-submit-api
///
/// So you can expect two kind of types here:
/// - Types that are used to decode the CBOR error reasons
/// - Types that are used to generate the JSON response structure
///
/// Here is an example response from the cardano-submit-api:
/// curl --header "Content-Type: application/cbor" -X POST http://localhost:8090/api/submit/tx --data-binary @tx.bin
/// {
///     "contents": {
///       "contents": {
///         "contents": {
///          "era": "ShelleyBasedEraConway",
///          "error": [
///             "ConwayUtxowFailure (UtxoFailure (ValueNotConservedUTxO (MaryValue (Coin 9498687280) (MultiAsset (fromList []))) (MaryValue (Coin 9994617117) (MultiAsset (fromList [])))))",
///             "ConwayUtxowFailure (UtxoFailure (FeeTooSmallUTxO (Coin 166909) (Coin 173)))"
///           ],
///           "kind": "ShelleyTxValidationError"
///         },
///         "tag": "TxValidationErrorInCardanoMode"
///       },
///       "tag": "TxCmdTxSubmitValidationError"
///     },
///     "tag": "TxSubmitFail"
///   }
///
/// Here is an example CBOR error reason from the cardano-node:
/// [2,
///     [
///         [6,
///             [
///                 [1,
///                     [0,
///                         [6, 9498687280, 9994617117]
///                     ]
///                 ],
///                 [1,
///                     [0,
///                         [5, 166909, 173]
///                     ]
///                 ]
///             ]
///         ]
///     ]
/// ]
///
/// TxValidationError is the most outer type that is decoded from the CBOR error reason.
/// Than, it is wrapped in TxValidationErrorInCardanoMode and TxCmdTxSubmitValidationError to generate the JSON response.
///
/// Type examples:
/// https://github.com/IntersectMBO/ouroboros-consensus/blob/82c5ebf7c9f902b7250144445f45083c1c13929e/ouroboros-consensus-cardano/src/shelley/Ouroboros/Consensus/Shelley/Eras.hs#L334
/// https://github.com/IntersectMBO/cardano-node-emulator/blob/ba5c4910a958bbccb38399f6a871459e46701a93/cardano-node-emulator/src/Cardano/Node/Emulator/Internal/Node/Validation.hs#L255
/// https://github.com/IntersectMBO/cardano-node/blob/master/cardano-testnet/test/cardano-testnet-test/files/golden/tx.failed.response.json.golden
///
/// Haskell references to the types are commented next to them.
/// Here are some more type references:
/// https://github.com/IntersectMBO/cardano-ledger/blob/78b20b6301b2703aa1fe1806ae3c129846708a10/libs/cardano-ledger-core/src/Cardano/Ledger/BaseTypes.hs#L737
/// https://github.com/IntersectMBO/cardano-ledger/blob/master/eras/mary/impl/src/Cardano/Ledger/Mary/Value.hs
/// https://github.com/IntersectMBO/cardano-ledger/blob/master/libs/cardano-ledger-core/src/Cardano/Ledger/Coin.hs

/*
** cardano-node CBOR types
** These types are used to decode the CBOR error reasons from the cardano-node.
** Some of them are decoded in codec.rs and some of them using Derive(Decode) macro.
*/
// https://github.com/IntersectMBO/cardano-api/blob/a0df586e3a14b98ae4771a192c09391dacb44564/cardano-api/internal/Cardano/Api/InMode.hs#L289
// https://github.com/IntersectMBO/cardano-api/blob/a0df586e3a14b98ae4771a192c09391dacb44564/cardano-api/internal/Cardano/Api/InMode.hs#L204
// toJson https://github.com/IntersectMBO/cardano-api/blob/a0df586e3a14b98ae4771a192c09391dacb44564/cardano-api/internal/Cardano/Api/InMode.hs#L233
#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
pub enum TxValidationError {
    ByronTxValidationError {
        error: ApplyTxErr,
    },
    ShelleyTxValidationError {
        error: ApplyTxErr,
        era: ShelleyBasedEra,
    },
}

// https://github.com/IntersectMBO/cardano-api/blob/a0df586e3a14b98ae4771a192c09391dacb44564/cardano-api/internal/Cardano/Api/Eon/ShelleyBasedEra.hs#L271
#[derive(Debug, Serialize, PartialEq)]
pub enum ShelleyBasedEra {
    ShelleyBasedEraShelley,
    ShelleyBasedEraAllegra,
    ShelleyBasedEraMary,
    ShelleyBasedEraAlonzo,
    ShelleyBasedEraBabbage,
    ShelleyBasedEraConway,
}

#[derive(Debug, Serialize)]
pub struct ApplyTxErr(pub Vec<ApplyConwayTxPredError>);

// https://github.com/IntersectMBO/cardano-ledger/blob/aed1dc28b98c25ea73bc692e7e6c6d3a22381ff5/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Ledger.hs#L146
#[derive(Debug, SerializeDisplay)]
pub enum ApplyConwayTxPredError {
    ConwayUtxowFailure(ConwayUtxoWPredFailure),
    CertsFailure(ConwayUtxoWPredFailure),
    GovFailure(ConwayUtxoWPredFailure),
    WdrlNotDelegatedToDRep(StakeKeyHash),
    ConwayTreasuryValueMismatch(DisplayCoin, DisplayCoin),
    TxRefScriptsSizeTooBig(u64),
    MempoolFailure(String),
}

impl fmt::Display for ApplyConwayTxPredError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ApplyConwayTxPredError::*;

        match self {
            ConwayUtxowFailure(e) => write!(f, "ConwayUtxowFailure ({})", e),
            CertsFailure(e) => write!(f, "CertsFailure ({})", e),
            GovFailure(e) => write!(f, "GovFailure ({})", e),
            WdrlNotDelegatedToDRep(e) => write!(f, "WdrlNotDelegatedToDRep ({})", e),
            ConwayTreasuryValueMismatch(c1, c2) => {
                write!(f, "ConwayTreasuryValueMismatch ({}) ({})", c1, c2)
            }
            TxRefScriptsSizeTooBig(e) => write!(f, "TxRefScriptsSizeTooBig ({})", e),
            MempoolFailure(e) => write!(f, "MempoolFailure ({})", e),
        }
    }
}

// https://github.com/IntersectMBO/cardano-ledger/blob/f54489071f4faa4b6209e1ba5288507c824cca50/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxow.hs
#[derive(Debug, SerializeDisplay)]
pub enum ConwayUtxoWPredFailure {
    UtxoFailure(ConwayUtxoPredFailure),
    InvalidWitnessesUTXOW(DisplayVKeyWitness),
    MissingVKeyWitnessesUTXOW(DisplayVKeyWitness),
    MissingScriptWitnessesUTXOW(DisplayScriptHash),
    ScriptWitnessNotValidatingUTXOW(DisplayScriptHash),
    MissingTxBodyMetadataHash(Bytes),      // auxDataHash
    MissingTxMetadata(Bytes),              // auxDataHash
    ConflictingMetadataHash(Bytes, Bytes), // Mismatch auxDataHash
    InvalidMetadata(),                     // empty
    ExtraneousScriptWitnessesUTXOW(DisplayScriptHash),
    MissingRedeemers(Vec<(PlutusPurpose, DisplayScriptHash)>),
    MissingRequiredDatums(Vec<DatumHash>, Vec<DatumHash>), // set of missing data hashes, set of recieved data hashes
    NotAllowedSupplementalDatums(Vec<DatumHash>, Vec<DatumHash>), // set of unallowed data hashes, set of acceptable data hashes
    PPViewHashesDontMatch(Option<ScriptIntegrityHash>),
    UnspendableUTxONoDatumHash(Vec<SerializableTxIn>), //  Set of transaction inputs that are TwoPhase scripts, and should have a DataHash but don't
    ExtraRedeemers(Vec<PlutusPurpose>),                // List of redeemers not needed
    MalformedScriptWitnesses(Vec<DisplayScriptHash>),
    MalformedReferenceScripts(Vec<DisplayScriptHash>),
}

impl fmt::Display for ConwayUtxoWPredFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ConwayUtxoWPredFailure::*;

        match self {
            UtxoFailure(e) => write!(f, "UtxoFailure ({})", e),
            InvalidWitnessesUTXOW(e) => write!(f, "InvalidWitnessesUTXOW ({})", e),
            MissingVKeyWitnessesUTXOW(e) => write!(f, "MissingVKeyWitnessesUTXOW ({})", e),
            MissingScriptWitnessesUTXOW(e) => write!(f, "MissingScriptWitnessesUTXOW ({})", e),
            ScriptWitnessNotValidatingUTXOW(e) => {
                write!(f, "ScriptWitnessNotValidatingUTXOW ({})", e)
            }
            MissingTxBodyMetadataHash(e) => write!(f, "MissingTxBodyMetadataHash ({})", e),
            MissingTxMetadata(e) => write!(f, "MissingTxMetadata ({})", e),
            ConflictingMetadataHash(e1, e2) => {
                write!(f, "ConflictingMetadataHash ({}, {})", e1, e2)
            }
            InvalidMetadata() => write!(f, "InvalidMetadata"),
            ExtraneousScriptWitnessesUTXOW(e) => {
                write!(f, "ExtraneousScriptWitnessesUTXOW ({})", e)
            }
            MissingRedeemers(e) => write!(f, "MissingRedeemers ({})", display_tuple_vec(e)),
            MissingRequiredDatums(e1, e2) => write!(
                f,
                "MissingRequiredDatums ({}, {})",
                display_vec(e1),
                display_vec(e2)
            ),
            NotAllowedSupplementalDatums(e1, e2) => write!(
                f,
                "NotAllowedSupplementalDatums ({}, {})",
                display_vec(e1),
                display_vec(e2)
            ),
            PPViewHashesDontMatch(e) => write!(f, "PPViewHashesDontMatch ({})", display_option(e)),
            UnspendableUTxONoDatumHash(e) => {
                write!(f, "UnspendableUTxONoDatumHash ({})", display_vec(e))
            }
            ExtraRedeemers(e) => write!(f, "ExtraRedeemers ({})", display_vec(e)),
            MalformedScriptWitnesses(e) => {
                write!(f, "MalformedScriptWitnesses ({})", display_vec(e))
            }
            MalformedReferenceScripts(e) => {
                write!(f, "MalformedReferenceScripts ({})", display_vec(e))
            }
        }
    }
}

// https://github.com/IntersectMBO/cardano-ledger/blob/f54489071f4faa4b6209e1ba5288507c824cca50/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxo.hs#L315
#[derive(Debug)]
pub enum ConwayUtxoPredFailure {
    UtxosFailure(Box<ConwayUtxoPredFailure>),
    BadInputsUTxO(Vec<SerializableTxIn>),
    OutsideValidityIntervalUTxO(ValidityInterval, SlotNo), // validity interval, current slot
    MaxTxSizeUTxO(u64),                                    // less than or equal
    InputSetEmptyUTxO(),                                   // empty
    FeeTooSmallUTxO(DisplayCoin, DisplayCoin),             // Mismatch expected, supplied
    ValueNotConservedUTxO(DisplayValue, DisplayValue),
    WrongNetwork(Network, Vec<Addr>), // the expected network id,  the set of addresses with incorrect network IDs
    WrongNetworkWithdrawal(Network, Vec<RewardAccount>), // the expected network id ,  the set of reward addresses with incorrect network IDs
    OutputTooSmallUTxO(Vec<SerializableTxOut>),
    OutputBootAddrAttrsTooBig(Vec<SerializableTxOut>),
    OutputTooBigUTxO(Vec<(u64, u64, SerializableTxOut)>), //  list of supplied bad transaction output triples (actualSize,PParameterMaxValue,TxOut)
    InsufficientCollateral(DisplayCoin, DisplayCoin), // balance computed, the required collateral for the given fee
    ScriptsNotPaidUTxO(Utxo), // The UTxO entries which have the wrong kind of script
    ExUnitsTooBigUTxO(DisplayExUnits), // check: The values are serialised in reverse order
    CollateralContainsNonADA(DisplayValue),
    WrongNetworkInTxBody(), // take in Network, https://github.com/IntersectMBO/cardano-ledger/blob/78b20b6301b2703aa1fe1806ae3c129846708a10/libs/cardano-ledger-core/src/Cardano/Ledger/BaseTypes.hs#L779
    OutsideForecast(SlotNo),
    TooManyCollateralInputs(u64), // this is Haskell Natural, how many bit is it?
    NoCollateralInputs(),         // empty
    IncorrectTotalCollateralField(DisplayCoin, DisplayCoin), // collateral provided, collateral amount declared in transaction body
    BabbageOutputTooSmallUTxO(Vec<(SerializableTxOut, DisplayCoin)>), // list of supplied transaction outputs that are too small, together with the minimum value for the given output
    BabbageNonDisjointRefInputs(Vec<SerializableTxIn>), // TxIns that appear in both inputs and reference inputs
}

impl fmt::Display for ConwayUtxoPredFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ConwayUtxoPredFailure::*;

        match self {
            UtxosFailure(e) => write!(f, "UtxosFailure ({})", e),
            BadInputsUTxO(e) => write!(f, "BadInputsUTxO ({})", display_vec(e)),
            OutsideValidityIntervalUTxO(vi, slot) => {
                write!(f, "OutsideValidityIntervalUTxO ({}, {})", vi, slot)
            }
            MaxTxSizeUTxO(size) => write!(f, "MaxTxSizeUTxO ({})", size),
            InputSetEmptyUTxO() => write!(f, "InputSetEmptyUTxO"),
            FeeTooSmallUTxO(expected, supplied) => {
                write!(f, "FeeTooSmallUTxO ({}) ({})", expected, supplied)
            }
            ValueNotConservedUTxO(expected, supplied) => {
                write!(f, "ValueNotConservedUTxO ({}) ({})", expected, supplied)
            }
            WrongNetwork(network, addrs) => {
                write!(f, "WrongNetwork ({}) ({})", network, display_vec(addrs))
            }
            WrongNetworkWithdrawal(network, accounts) => write!(
                f,
                "WrongNetworkWithdrawal ({}) ({})",
                network,
                display_vec(accounts)
            ),
            OutputTooSmallUTxO(outputs) => {
                write!(f, "OutputTooSmallUTxO ({})", display_vec(outputs))
            }
            OutputBootAddrAttrsTooBig(outputs) => {
                write!(f, "OutputBootAddrAttrsTooBig ({})", display_vec(outputs))
            }
            OutputTooBigUTxO(outputs) => {
                write!(f, "OutputTooBigUTxO ({})", display_triple_vec(outputs))
            }
            InsufficientCollateral(balance, required) => {
                write!(f, "InsufficientCollateral ({}) ({})", balance, required)
            }
            ScriptsNotPaidUTxO(utxo) => write!(f, "ScriptsNotPaidUTxO ({})", utxo),
            ExUnitsTooBigUTxO(units) => write!(f, "ExUnitsTooBigUTxO ({})", units),
            CollateralContainsNonADA(value) => write!(f, "CollateralContainsNonADA ({})", value),
            WrongNetworkInTxBody() => write!(f, "WrongNetworkInTxBody"),
            OutsideForecast(slot) => write!(f, "OutsideForecast ({})", slot),
            TooManyCollateralInputs(inputs) => write!(f, "TooManyCollateralInputs ({})", inputs),
            NoCollateralInputs() => write!(f, "NoCollateralInputs"),
            IncorrectTotalCollateralField(provided, declared) => write!(
                f,
                "IncorrectTotalCollateralField ({}, {})",
                provided, declared
            ),
            BabbageOutputTooSmallUTxO(outputs) => {
                write!(
                    f,
                    "BabbageOutputTooSmallUTxO ({})",
                    display_tuple_vec(outputs)
                )
            }
            BabbageNonDisjointRefInputs(inputs) => {
                write!(f, "BabbageNonDisjointRefInputs ({})", display_vec(inputs))
            }
        }
    }
}

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct DisplayScriptHash(#[n(0)] pub ScriptHash);

impl fmt::Display for DisplayScriptHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VKeyWitness {{ {} ", (self.0))
    }
}

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct DisplayVKeyWitness(#[n(0)] pub VKeyWitness);

impl fmt::Display for DisplayVKeyWitness {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "VKeyWitness {{ vkey: {}, signature: {} }}",
            (self.0).vkey,
            (self.0).signature
        )
    }
}

// https://github.com/IntersectMBO/cardano-ledger/blob/f54489071f4faa4b6209e1ba5288507c824cca50/libs/cardano-ledger-core/src/Cardano/Ledger/Address.hs
// the bytes are not decoded
pub type Addr = Bytes;

// https://github.com/IntersectMBO/cardano-ledger/blob/78b20b6301b2703aa1fe1806ae3c129846708a10/eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Scripts.hs#L497
// not tested yet
#[derive(Debug, Serialize)]
pub enum PlutusPurpose {
    Spending,   // 0
    Minting,    // 1
    Certifying, // 2
    Rewarding,  // 3
}

impl fmt::Display for PlutusPurpose {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use PlutusPurpose::*;

        match self {
            Spending => write!(f, "Spending"),
            Minting => write!(f, "Minting"),
            Certifying => write!(f, "Certifying"),
            Rewarding => write!(f, "Rewarding"),
        }
    }
}

// https://github.com/IntersectMBO/cardano-ledger/blob/78b20b6301b2703aa1fe1806ae3c129846708a10/libs/cardano-ledger-core/src/Cardano/Ledger/BaseTypes.hs#L779
#[derive(Debug, Decode, Serialize)]
pub enum Network {
    #[n(0)]
    Mainnet,
    #[n(1)]
    Testnet,
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Network::*;

        match self {
            Mainnet => write!(f, "Mainnet"),
            Testnet => write!(f, "Testnet"),
        }
    }
}
// https://github.com/IntersectMBO/cardano-ledger/blob/aed1dc28b98c25ea73bc692e7e6c6d3a22381ff5/eras/alonzo/impl/src/Cardano/Ledger/Alonzo/TxBody/Internal.hs#L162
// not tested yet
#[derive(Debug, Decode)]
pub struct ScriptIntegrityHash(#[n(0)] ScriptHash);

impl fmt::Display for ScriptIntegrityHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ScriptIntegrityHash({:?})", self.0)
    }
}

// https://github.com/IntersectMBO/cardano-ledger/blob/aed1dc28b98c25ea73bc692e7e6c6d3a22381ff5/eras/allegra/impl/src/Cardano/Ledger/Allegra/Scripts.hs#L109
#[derive(Debug, Decode, Serialize)]

pub struct ValidityInterval {
    #[n(0)]
    pub invalid_before: Option<SlotNo>, // SlotNo
    #[n(1)]
    pub invalid_hereafter: Option<SlotNo>, // SlotNo
}

impl fmt::Display for ValidityInterval {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ValidityInterval {{ invalid_before: {}, invalid_hereafter: {} }}",
            display_option(&self.invalid_before),
            display_option(&self.invalid_hereafter)
        )
    }
}

// https://github.com/IntersectMBO/cardano-ledger/blob/aed1dc28b98c25ea73bc692e7e6c6d3a22381ff5/libs/cardano-ledger-core/src/Cardano/Ledger/UTxO.hs#L83
#[derive(Debug)]
pub struct Utxo(pub Vec<(SerializableTxIn, SerializableTxOut)>);

impl fmt::Display for Utxo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Utxo({})", display_tuple_vec(&self.0))
    }
}

#[derive(Debug, Decode)]
pub struct SerializableTxIn(#[n(0)] pub TxIn);

impl fmt::Display for SerializableTxIn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Debug, Decode)]
pub struct SerializableTxOut(#[n(0)] pub TxOut);

impl fmt::Display for SerializableTxOut {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

type SlotNo = u64;

// https://github.com/IntersectMBO/ouroboros-consensus/blob/e86b921443bd6e8ea25e7190eb7cb5788e28f4cc/ouroboros-consensus/src/ouroboros-consensus/Ouroboros/Consensus/HardFork/Combinator/AcrossEras.hs#L208
#[derive(Serialize)]
pub struct EraMismatch {
    ledger: String, //  Name of the era of the ledger ("Byron" or "Shelley").
    other: String,  // Era of the block, header, transaction, or query.
}

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct DisplayCoin(#[n(0)] Coin);

impl fmt::Display for DisplayCoin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Coin {}", self.0)
    }
}

/*
** cardano-submit-api types
** These types are used to mimick cardano-submit-api error responses.
*/

// https://github.com/IntersectMBO/cardano-node/blob/9dbf0b141e67ec2dfd677c77c63b1673cf9c5f3e/cardano-submit-api/src/Cardano/TxSubmit/Types.hs#L54
#[derive(Serialize)]
#[serde(tag = "tag", content = "contents")]
pub enum TxSubmitFail {
    TxSubmitDecodeHex,
    TxSubmitEmpty,
    TxSubmitDecodeFail(DecoderError),
    TxSubmitBadTx(String),
    TxSubmitFail(TxCmdError),
}

// https://github.com/IntersectMBO/cardano-node/blob/9dbf0b141e67ec2dfd677c77c63b1673cf9c5f3e/cardano-submit-api/src/Cardano/TxSubmit/Types.hs#L92
#[derive(Serialize)]
#[serde(tag = "tag", content = "contents")]
pub enum TxCmdError {
    SocketEnvError(String),
    TxReadError(Vec<DecoderError>),
    TxCmdTxSubmitValidationError(TxValidationErrorInCardanoMode),
}

// TODO: Implement DecoderError errors from the Haskell codebase.
// Lots of errors, skipping for now. https://github.com/IntersectMBO/cardano-base/blob/391a2c5cfd30d2234097e000dbd8d9db21ef94d7/cardano-binary/src/Cardano/Binary/FromCBOR.hs#L90
type DecoderError = String;

// https://github.com/IntersectMBO/cardano-api/blob/d7c62a04ebf18d194a6ea70e6765eb7691d57668/cardano-api/internal/Cardano/Api/InMode.hs#L259
#[derive(Serialize)]
#[serde(tag = "tag", content = "contents")]
pub enum TxValidationErrorInCardanoMode {
    TxValidationErrorInCardanoMode(TxValidationError),
    EraMismatch(EraMismatch),
}

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct DisplayExUnits(#[n(0)] pub ExUnits);

impl fmt::Display for DisplayExUnits {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ExUnits {{ mem: {}, steps: {} }}",
            self.0.mem, self.0.steps
        )
    }
}

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct DisplayValue(#[n(0)] pub Value);

impl fmt::Display for DisplayValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Value {{ {:?} }}", self.0)
    }
}

/*
**Helper functions for Display'ing the types.
*/
fn display_tuple<T: Display, U: Display>(t: &(T, U)) -> String {
    format!("({} {})", t.0, t.1)
}

fn display_tuple_vec<T: Display, U: Display>(vec: &[(T, U)]) -> String {
    vec.iter()
        .map(|x| display_tuple(x))
        .collect::<Vec<String>>()
        .join(" ")
}

fn display_triple<T: Display, U: Display, V: Display>(t: &(T, U, V)) -> String {
    format!("({} {} {})", t.0, t.1, t.2)
}
fn display_triple_vec<T: Display, U: Display, V: Display>(vec: &[(T, U, V)]) -> String {
    vec.iter()
        .map(|x| display_triple(x))
        .collect::<Vec<String>>()
        .join(" ")
}
fn display_vec<T: Display>(vec: &[T]) -> String {
    vec.iter()
        .map(|x| format!("{}", x))
        .collect::<Vec<String>>()
        .join(" ")
}

fn display_option<T: Display>(opt: &Option<T>) -> String {
    match opt {
        Some(x) => format!("{}", x),
        None => "None".to_string(),
    }
}
