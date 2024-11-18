#![allow(dead_code)]

use std::fmt;

use pallas::ledger::addresses::StakeKeyHash;
use pallas_codec::minicbor;
use pallas_codec::minicbor::Decode;
use pallas_codec::utils::Bytes;
use pallas_primitives::{
    byron::{TxIn, TxOut},
    conway::{Coin, DatumHash, ExUnits, RewardAccount, ScriptHash, VKeyWitness, Value},
};

/// This file contains the types that are mapped from the Haskell codebase.
/// Type examples:
/// https://github.com/IntersectMBO/ouroboros-consensus/blob/82c5ebf7c9f902b7250144445f45083c1c13929e/ouroboros-consensus-cardano/src/shelley/Ouroboros/Consensus/Shelley/Eras.hs#L334
/// https://github.com/IntersectMBO/cardano-node-emulator/blob/ba5c4910a958bbccb38399f6a871459e46701a93/cardano-node-emulator/src/Cardano/Node/Emulator/Internal/Node/Validation.hs#L255
/// https://github.com/IntersectMBO/cardano-node/blob/master/cardano-testnet/test/cardano-testnet-test/files/golden/tx.failed.response.json.golden
///
/// Haskell references to the types are commented next to them.
/// Here are some more type referernces:
/// https://github.com/IntersectMBO/cardano-ledger/blob/78b20b6301b2703aa1fe1806ae3c129846708a10/libs/cardano-ledger-core/src/Cardano/Ledger/BaseTypes.hs#L737
/// https://github.com/IntersectMBO/cardano-ledger/blob/master/eras/mary/impl/src/Cardano/Ledger/Mary/Value.hs
/// https://github.com/IntersectMBO/cardano-ledger/blob/master/libs/cardano-ledger-core/src/Cardano/Ledger/Coin.hs

// https://github.com/IntersectMBO/cardano-api/blob/a0df586e3a14b98ae4771a192c09391dacb44564/cardano-api/internal/Cardano/Api/Eon/ShelleyBasedEra.hs#L271
#[derive(Debug, serde::Serialize)]
pub enum ShelleyBasedEra {
    ShelleyBasedEraShelley,
    ShelleyBasedEraAllegra,
    ShelleyBasedEraMary,
    ShelleyBasedEraAlonzo,
    ShelleyBasedEraBabbage,
    ShelleyBasedEraConway,
}

#[derive(Debug, serde::Serialize)]
pub struct ApplyTxErr(pub Vec<ApplyConwayTxPredError>);

// https://github.com/IntersectMBO/cardano-ledger/blob/aed1dc28b98c25ea73bc692e7e6c6d3a22381ff5/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Ledger.hs#L146
#[derive(Debug, serde::Serialize)]
pub enum ApplyConwayTxPredError {
    UtxowFailure(ConwayUtxoWPredFailure),
    CertsFailure(ConwayUtxoWPredFailure),
    GovFailure(ConwayUtxoWPredFailure),
    WdrlNotDelegatedToDRep(StakeKeyHash),
    TreasuryValueMismatch(Coin),
    TxRefScriptsSizeTooBig(u64),
    MempoolFailure(String),
}

impl fmt::Display for ApplyConwayTxPredError {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ApplyConwayTxPredError::*;

        match self {
            UtxowFailure(e) => write!(f, "UtxowFailure ({})", e),
            CertsFailure(e) => write!(f, "CertsFailure ({})", e),
            GovFailure(e) => write!(f, "GovFailure ({})", e),
            WdrlNotDelegatedToDRep(e) => write!(f, "WdrlNotDelegatedToDRep ({})", e),
            TreasuryValueMismatch(e) => write!(f, "TreasuryValueMismatch ({})", e),
            TxRefScriptsSizeTooBig(e) => write!(f, "TxRefScriptsSizeTooBig ({})", e),
            MempoolFailure(e) => write!(f, "MempoolFailure ({})", e),
        }
    }
}

// https://github.com/IntersectMBO/cardano-ledger/blob/f54489071f4faa4b6209e1ba5288507c824cca50/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxow.hs
#[derive(Debug, serde::Serialize)]
pub enum ConwayUtxoWPredFailure {
    UtxoFailure(ConwayUtxoPredFailure),
    InvalidWitnessesUTXOW(VKeyWitness),
    MissingVKeyWitnessesUTXOW(VKeyWitness),
    MissingScriptWitnessesUTXOW(ScriptHash),
    ScriptWitnessNotValidatingUTXOW(ScriptHash),
    MissingTxBodyMetadataHash(Bytes),      // auxDataHash
    MissingTxMetadata(Bytes),              // auxDataHash
    ConflictingMetadataHash(Bytes, Bytes), // Mismatch auxDataHash
    InvalidMetadata(),                     // empty
    ExtraneousScriptWitnessesUTXOW(ScriptHash),
    MissingRedeemers(Vec<(PlutusPurpose, ScriptHash)>),
    MissingRequiredDatums(Vec<DatumHash>, Vec<DatumHash>), // set of missing data hashes, set of recieved data hashes
    NotAllowedSupplementalDatums(Vec<DatumHash>, Vec<DatumHash>), // set of unallowed data hashes, set of acceptable data hashes
    PPViewHashesDontMatch(Option<ScriptIntegrityHash>),
    UnspendableUTxONoDatumHash(Vec<SerializableTxIn>), //  Set of transaction inputs that are TwoPhase scripts, and should have a DataHash but don't
    ExtraRedeemers(Vec<PlutusPurpose>),                // List of redeemers not needed
    MalformedScriptWitnesses(Vec<ScriptHash>),
    MalformedReferenceScripts(Vec<ScriptHash>),
}

impl fmt::Display for ConwayUtxoWPredFailure {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ConwayUtxoWPredFailure::*;

        match self { 
            UtxoFailure(e) => write!(f, "UtxoFailure ({})", e),
            InvalidWitnessesUTXOW(e) => write!(f, "InvalidWitnessesUTXOW ({:?})", e),
            MissingVKeyWitnessesUTXOW(e) => write!(f, "MissingVKeyWitnessesUTXOW ({:?})", e),
            MissingScriptWitnessesUTXOW(e) => write!(f, "MissingScriptWitnessesUTXOW ({:?})", e),
            ScriptWitnessNotValidatingUTXOW(e) => write!(f, "ScriptWitnessNotValidatingUTXOW ({:?})", e),
            MissingTxBodyMetadataHash(e) => write!(f, "MissingTxBodyMetadataHash ({:?})", e),
            MissingTxMetadata(e) => write!(f, "MissingTxMetadata ({:?})", e),
            ConflictingMetadataHash(e1, e2) => write!(f, "ConflictingMetadataHash ({:?}, {:?})", e1, e2),
            InvalidMetadata() => write!(f, "InvalidMetadata"),
            ExtraneousScriptWitnessesUTXOW(e) => write!(f, "ExtraneousScriptWitnessesUTXOW ({:?})", e),
            MissingRedeemers(e) => write!(f, "MissingRedeemers ({:?})", e),
            MissingRequiredDatums(e1, e2) => write!(f, "MissingRequiredDatums ({:?}, {:?})", e1, e2),
            NotAllowedSupplementalDatums(e1, e2) => write!(f, "NotAllowedSupplementalDatums ({:?}, {:?})", e1, e2),
            PPViewHashesDontMatch(e) => write!(f, "PPViewHashesDontMatch ({:?})", e),
            UnspendableUTxONoDatumHash(e) => write!(f, "UnspendableUTxONoDatumHash ({:?})", e),
            ExtraRedeemers(e) => write!(f, "ExtraRedeemers ({:?})", e),
            MalformedScriptWitnesses(e) => write!(f, "MalformedScriptWitnesses ({:?})", e),
            MalformedReferenceScripts(e) => write!(f, "MalformedReferenceScripts ({:?})", e),
        }
    }
}

// https://github.com/IntersectMBO/cardano-ledger/blob/f54489071f4faa4b6209e1ba5288507c824cca50/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxo.hs#L315
#[derive(Debug, serde::Serialize)]
pub enum ConwayUtxoPredFailure {
    UtxosFailure(Box<ConwayUtxoPredFailure>),
    BadInputsUTxO(Vec<SerializableTxIn>),
    OutsideValidityIntervalUTxO(ValidityInterval, SlotNo), // validity interval, current slot
    MaxTxSizeUTxO(u64),                                    // less than or equal
    InputSetEmptyUTxO(),                                   // empty
    FeeTooSmallUTxO(Coin, Coin),                           // Mismatch expected, supplied
    ValueNotConservedUTxO(Value, Value),
    WrongNetwork(Network, Vec<Addr>), // the expected network id,  the set of addresses with incorrect network IDs
    WrongNetworkWithdrawal(Network, Vec<RewardAccount>), // the expected network id ,  the set of reward addresses with incorrect network IDs
    OutputTooSmallUTxO(Vec<SerializableTxOut>),
    OutputBootAddrAttrsTooBig(Vec<SerializableTxOut>),
    OutputTooBigUTxO(Vec<(u64, u64, SerializableTxOut)>), //  list of supplied bad transaction output triples (actualSize,PParameterMaxValue,TxOut)
    InsufficientCollateral(Coin, Coin), // balance computed, the required collateral for the given fee
    ScriptsNotPaidUTxO(Utxo),           // The UTxO entries which have the wrong kind of script
    ExUnitsTooBigUTxO(ExUnits),         // check: The values are serialised in reverse order
    CollateralContainsNonADA(Value),
    WrongNetworkInTxBody(), // take in Network, https://github.com/IntersectMBO/cardano-ledger/blob/78b20b6301b2703aa1fe1806ae3c129846708a10/libs/cardano-ledger-core/src/Cardano/Ledger/BaseTypes.hs#L779
    OutsideForecast(SlotNo),
    TooManyCollateralInputs(u64), // this is Haskell Natural, how many bit is it?
    NoCollateralInputs(),         // empty
    IncorrectTotalCollateralField(Coin, Coin), // collateral provided, collateral amount declared in transaction body
    BabbageOutputTooSmallUTxO(Vec<(SerializableTxOut, Coin)>), // list of supplied transaction outputs that are too small, together with the minimum value for the given output
    BabbageNonDisjointRefInputs(Vec<SerializableTxIn>), // TxIns that appear in both inputs and reference inputs
}

impl fmt::Display for ConwayUtxoPredFailure {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ConwayUtxoPredFailure::*;

        match self {
            UtxosFailure(e) => write!(f, "UtxosFailure ({})", e),
            BadInputsUTxO(e) => write!(f, "BadInputsUTxO ({:?})", e),
            OutsideValidityIntervalUTxO(vi, slot) => write!(f, "OutsideValidityIntervalUTxO ({:?}, {:?})", vi, slot),
            MaxTxSizeUTxO(size) => write!(f, "MaxTxSizeUTxO ({})", size),
            InputSetEmptyUTxO() => write!(f, "InputSetEmptyUTxO"),
            FeeTooSmallUTxO(expected, supplied) => write!(f, "FeeTooSmallUTxO ({}, {})", expected, supplied),
            ValueNotConservedUTxO(expected, supplied) => write!(f, "ValueNotConservedUTxO ({:?}, {:?})", expected, supplied),
            WrongNetwork(network, addrs) => write!(f, "WrongNetwork ({:?}, {:?})", network, addrs),
            WrongNetworkWithdrawal(network, accounts) => write!(f, "WrongNetworkWithdrawal ({:?}, {:?})", network, accounts),
            OutputTooSmallUTxO(outputs) => write!(f, "OutputTooSmallUTxO ({:?})", outputs),
            OutputBootAddrAttrsTooBig(outputs) => write!(f, "OutputBootAddrAttrsTooBig ({:?})", outputs),
            OutputTooBigUTxO(outputs) => write!(f, "OutputTooBigUTxO ({:?})", outputs),
            InsufficientCollateral(balance, required) => write!(f, "InsufficientCollateral ({}, {})", balance, required),
            ScriptsNotPaidUTxO(utxo) => write!(f, "ScriptsNotPaidUTxO ({:?})", utxo),
            ExUnitsTooBigUTxO(units) => write!(f, "ExUnitsTooBigUTxO ({:?})", units),
            CollateralContainsNonADA(value) => write!(f, "CollateralContainsNonADA ({:?})", value),
            WrongNetworkInTxBody() => write!(f, "WrongNetworkInTxBody"),
            OutsideForecast(slot) => write!(f, "OutsideForecast ({})", slot),
            TooManyCollateralInputs(inputs) => write!(f, "TooManyCollateralInputs ({})", inputs),
            NoCollateralInputs() => write!(f, "NoCollateralInputs"),
            IncorrectTotalCollateralField(provided, declared) => write!(f, "IncorrectTotalCollateralField ({}, {})", provided, declared),
            BabbageOutputTooSmallUTxO(outputs) => write!(f, "BabbageOutputTooSmallUTxO ({:?})", outputs),
            BabbageNonDisjointRefInputs(inputs) => write!(f, "BabbageNonDisjointRefInputs ({:?})", inputs),
        }
    }
}

// wrapping  TxValidationError (ShelleyTxValidationError or ByronTxValidationError) in TxValidationErrorInCardanoMode
// https://github.com/IntersectMBO/cardano-api/blob/a0df586e3a14b98ae4771a192c09391dacb44564/cardano-api/internal/Cardano/Api/InMode.hs#L289
// https://github.com/IntersectMBO/cardano-api/blob/a0df586e3a14b98ae4771a192c09391dacb44564/cardano-api/internal/Cardano/Api/InMode.hs#L204
// toJson https://github.com/IntersectMBO/cardano-api/blob/a0df586e3a14b98ae4771a192c09391dacb44564/cardano-api/internal/Cardano/Api/InMode.hs#L233
#[derive(Debug, serde::Serialize)]
#[serde(tag = "kind")]
pub enum TxValidationError {
    
    ByronTxValidationError{error: ApplyTxErr},
    ShelleyTxValidationError{error: ApplyTxErr, era: ShelleyBasedEra},
}

// https://github.com/IntersectMBO/cardano-ledger/blob/f54489071f4faa4b6209e1ba5288507c824cca50/libs/cardano-ledger-core/src/Cardano/Ledger/Address.hs
// the bytes are not decoded
pub type Addr = Bytes;

// https://github.com/IntersectMBO/cardano-ledger/blob/78b20b6301b2703aa1fe1806ae3c129846708a10/eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Scripts.hs#L497
// not tested yet
#[derive(Debug, serde::Serialize)]
pub enum PlutusPurpose {
    Spending,   // 0
    Minting,    // 1
    Certifying, // 2
    Rewarding,  // 3
}
// https://github.com/IntersectMBO/cardano-ledger/blob/78b20b6301b2703aa1fe1806ae3c129846708a10/libs/cardano-ledger-core/src/Cardano/Ledger/BaseTypes.hs#L779
#[derive(Debug, Decode, serde::Serialize)]
pub enum Network {
    #[n(0)]
    Mainnet,
    #[n(1)]
    Testnet,
}
// https://github.com/IntersectMBO/cardano-ledger/blob/aed1dc28b98c25ea73bc692e7e6c6d3a22381ff5/eras/alonzo/impl/src/Cardano/Ledger/Alonzo/TxBody/Internal.hs#L162
// not tested yet
type ScriptIntegrityHash = ScriptHash;

// https://github.com/IntersectMBO/cardano-ledger/blob/aed1dc28b98c25ea73bc692e7e6c6d3a22381ff5/eras/allegra/impl/src/Cardano/Ledger/Allegra/Scripts.hs#L109
#[derive(Debug, Decode, serde::Serialize)]

pub struct ValidityInterval {
    #[n(0)]
    pub invalid_before: Option<SlotNo>, // SlotNo
    #[n(1)]
    pub invalid_hereafter: Option<SlotNo>, // SlotNo
}

// https://github.com/IntersectMBO/cardano-ledger/blob/aed1dc28b98c25ea73bc692e7e6c6d3a22381ff5/libs/cardano-ledger-core/src/Cardano/Ledger/UTxO.hs#L83
#[derive(Debug, serde::Serialize)]
pub struct Utxo(pub Vec<(SerializableTxIn, SerializableTxOut)>);

#[derive(Debug, Decode)]
pub struct SerializableTxIn(#[n(0)] pub TxIn);

impl serde::Serialize for SerializableTxIn {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!()
    }
}

#[derive(Debug, Decode)]
pub struct SerializableTxOut(#[n(0)] pub TxOut);

impl serde::Serialize for SerializableTxOut {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!()
    }
}

type SlotNo = u64;

// https://github.com/IntersectMBO/ouroboros-consensus/blob/e86b921443bd6e8ea25e7190eb7cb5788e28f4cc/ouroboros-consensus/src/ouroboros-consensus/Ouroboros/Consensus/HardFork/Combinator/AcrossEras.hs#L208
#[derive(Debug, serde::Serialize)]
pub struct EraMismatch {
    ledger: String, //  Name of the era of the ledger ("Byron" or "Shelley").
    other: String,  // Era of the block, header, transaction, or query.
}

/*
** cardano-submit-api types
** These types are used to mimick cardano-submit-api error responses.
*/

// https://github.com/IntersectMBO/cardano-node/blob/9dbf0b141e67ec2dfd677c77c63b1673cf9c5f3e/cardano-submit-api/src/Cardano/TxSubmit/Types.hs#L54
#[derive(serde::Serialize)]
#[serde(tag = "tag", content = "contents")]
pub enum TxSubmitFail {
    TxSubmitDecodeHex,
    TxSubmitEmpty,
    TxSubmitDecodeFail(DecoderError),
    TxSubmitBadTx(String),
    TxSubmitFail(TxCmdError),
}

// https://github.com/IntersectMBO/cardano-node/blob/9dbf0b141e67ec2dfd677c77c63b1673cf9c5f3e/cardano-submit-api/src/Cardano/TxSubmit/Types.hs#L92
#[derive(serde::Serialize)]
#[serde(tag = "tag", content = "contents")]
pub enum TxCmdError {
    SocketEnvError(String),
    TxReadError(Vec<DecoderError>),
    TxCmdTxSubmitValidationError(TxValidationErrorInCardanoMode),
}

// Lots of errors, skipping for now. https://github.com/IntersectMBO/cardano-base/blob/391a2c5cfd30d2234097e000dbd8d9db21ef94d7/cardano-binary/src/Cardano/Binary/FromCBOR.hs#L90
type DecoderError = String;

// https://github.com/IntersectMBO/cardano-api/blob/d7c62a04ebf18d194a6ea70e6765eb7691d57668/cardano-api/internal/Cardano/Api/InMode.hs#L259
#[derive(Debug, serde::Serialize)]
#[serde(tag = "tag", content = "contents")]
pub enum TxValidationErrorInCardanoMode {
    TxValidationErrorInCardanoMode(TxValidationError),
    EraMismatch(EraMismatch),
}


