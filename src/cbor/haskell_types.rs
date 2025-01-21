#![allow(dead_code)]

use std::{
    collections::HashMap,
    fmt::{self},
};

use pallas_addresses::Address;
use pallas_codec::minicbor;
use pallas_codec::minicbor::Decode;
use pallas_codec::utils::Bytes;
use pallas_network::miniprotocols::localstate::queries_v16::Datum;
use pallas_primitives::{
    byron::{Blake2b256, TxIn, TxOut},
    conway::{
        Anchor, DatumHash, ExUnits, GovAction, GovActionId, ProposalProcedure, RewardAccount,
        ScriptHash, Value, Voter,
    },
    AddrKeyhash, AssetName, Coin, PolicyId, StakeCredential, TransactionInput,
};
use serde::Serialize;
use serde_with::SerializeDisplay;
use std::fmt::Display;

use super::haskell_display::HaskellDisplay;

/// This file contains the types that are mapped from the Haskell codebase.
/// The main reason these mappings exist is to mimick the error responses from the cardano-submit-api
/// and generate identical responses to the [Blockfrost.io `/tx/submit` API](https://docs.blockfrost.io/#tag/cardano--transactions/POST/tx/submit).
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
/// ```text
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
/// ```
///
/// Here is an example CBOR error reason from the cardano-node:
/// ```text
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
/// ```
///
/// TxValidationError is the most outer type that is decoded from the CBOR error reason.
/// Than, it is wrapped in TxValidationErrorInCardanoMode and TxCmdTxSubmitValidationError to generate the JSON response.
///
/// Type examples:
/// * <https://github.com/IntersectMBO/ouroboros-consensus/blob/82c5ebf7c9f902b7250144445f45083c1c13929e/ouroboros-consensus-cardano/src/shelley/Ouroboros/Consensus/Shelley/Eras.hs#L334>
/// * <https://github.com/IntersectMBO/cardano-node-emulator/blob/ba5c4910a958bbccb38399f6a871459e46701a93/cardano-node-emulator/src/Cardano/Node/Emulator/Internal/Node/Validation.hs#L255>
/// * <https://github.com/IntersectMBO/cardano-node/blob/master/cardano-testnet/test/cardano-testnet-test/files/golden/tx.failed.response.json.golden>
///
/// Haskell references to the types are commented next to them.
/// Here are some more type references:
/// * <https://github.com/IntersectMBO/cardano-ledger/blob/78b20b6301b2703aa1fe1806ae3c129846708a10/libs/cardano-ledger-core/src/Cardano/Ledger/BaseTypes.hs#L737>
/// * <https://github.com/IntersectMBO/cardano-ledger/blob/master/eras/mary/impl/src/Cardano/Ledger/Mary/Value.hs>
/// * <https://github.com/IntersectMBO/cardano-ledger/blob/master/libs/cardano-ledger-core/src/Cardano/Ledger/Coin.hs>

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
        error: ApplyTxError,
    },
    ShelleyTxValidationError {
        error: ApplyTxError,
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
pub struct ApplyTxError(pub Vec<ApplyConwayTxPredError>);

// https://github.com/IntersectMBO/cardano-ledger/blob/aed1dc28b98c25ea73bc692e7e6c6d3a22381ff5/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Ledger.hs#L146
#[derive(Debug, SerializeDisplay)]
pub enum ApplyConwayTxPredError {
    ConwayUtxowFailure(ConwayUtxoWPredFailure),
    ConwayCertsFailure(ConwayCertsPredFailure),
    ConwayGovFailure(ConwayGovPredFailure),
    ConwayWdrlNotDelegatedToDRep(Vec<KeyHash>),
    ConwayTreasuryValueMismatch(DisplayCoin, DisplayCoin),
    ConwayTxRefScriptsSizeTooBig(i8, i8),
    ConwayMempoolFailure(String),
}

impl fmt::Display for ApplyConwayTxPredError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ApplyConwayTxPredError::*;

        match self {
            ConwayUtxowFailure(e) => write!(f, "ConwayUtxowFailure {}", e),
            ConwayCertsFailure(e) => write!(f, "ConwayCertsFailure ({})", e),
            ConwayGovFailure(e) => write!(f, "ConwayGovFailure ({})", e),
            ConwayWdrlNotDelegatedToDRep(v) => {
                write!(f, "ConwayWdrlNotDelegatedToDRep ({})", v.to_haskell_str())
            }
            ConwayTreasuryValueMismatch(c1, c2) => {
                write!(
                    f,
                    "ConwayTreasuryValueMismatch ({}) ({})",
                    c1.to_haskell_str(),
                    c2.to_haskell_str()
                )
            }
            ConwayTxRefScriptsSizeTooBig(s1, s2) => {
                write!(
                    f,
                    "ConwayTxRefScriptsSizeTooBig {} {}",
                    s1.to_haskell_str(),
                    s2.to_haskell_str()
                )
            }
            ConwayMempoolFailure(e) => {
                write!(f, "ConwayMempoolFailure {}", e.to_haskell_str())
            }
        }
    }
}

// https://github.com/IntersectMBO/cardano-ledger/blob/f54489071f4faa4b6209e1ba5288507c824cca50/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxow.hs
#[derive(Debug, SerializeDisplay)]
pub enum ConwayUtxoWPredFailure {
    UtxoFailure(ConwayUtxoPredFailure),
    InvalidWitnessesUTXOW(Array<VKey>),
    MissingVKeyWitnessesUTXOW(CustomSet258<KeyHash>),
    MissingScriptWitnessesUTXOW(CustomSet258<ScriptHash>),
    ScriptWitnessNotValidatingUTXOW(CustomSet258<ScriptHash>),
    MissingTxBodyMetadataHash(Bytes),      // auxDataHash
    MissingTxMetadata(Bytes),              // auxDataHash
    ConflictingMetadataHash(Bytes, Bytes), // Mismatch auxDataHash
    InvalidMetadata(),                     // empty
    ExtraneousScriptWitnessesUTXOW(CustomSet258<ScriptHash>),
    MissingRedeemers(Array<(PlutusPurpose, ScriptHash)>),
    MissingRequiredDatums(Vec<DatumHash>, Vec<DatumHash>), // set of missing data hashes, set of recieved data hashes
    NotAllowedSupplementalDatums(CustomSet258<SafeHash>, CustomSet258<SafeHash>), // set of unallowed data hashes, set of acceptable data hashes
    PPViewHashesDontMatch(StrictMaybe<SafeHash>, StrictMaybe<SafeHash>),
    UnspendableUTxONoDatumHash(CustomSet258<TransactionInput>), //  Set of transaction inputs that are TwoPhase scripts, and should have a DataHash but don't
    ExtraRedeemers(Array<PlutusPurpose>),                       // List of redeemers not needed
    MalformedScriptWitnesses(CustomSet258<ScriptHash>),
    MalformedReferenceScripts(CustomSet258<ScriptHash>),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/7683b73971a800b36ca7317601552685fa0701ed/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxo.hs#L315
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
    WrongNetworkWithdrawal(Network, Vec<RewardAccountFielded>), // the expected network id ,  the set of reward addresses with incorrect network IDs
    OutputTooSmallUTxO(Array<BabbageTxOut>),
    OutputBootAddrAttrsTooBig(Array<SerializableTxOut>),
    OutputTooBigUTxO(Vec<(u64, u64, SerializableTxOut)>), //  list of supplied bad transaction output triples (actualSize,PParameterMaxValue,TxOut)
    InsufficientCollateral(DeltaCoin, DisplayCoin), // balance computed, the required collateral for the given fee
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
            BadInputsUTxO(e) => write!(f, "BadInputsUTxO ({})", e.to_haskell_str()),
            OutsideValidityIntervalUTxO(vi, slot) => {
                write!(f, "OutsideValidityIntervalUTxO ({}, {})", vi, slot)
            }
            MaxTxSizeUTxO(size) => write!(f, "MaxTxSizeUTxO ({})", size),
            InputSetEmptyUTxO() => write!(f, "InputSetEmptyUTxO"),
            FeeTooSmallUTxO(expected, supplied) => {
                write!(
                    f,
                    "FeeTooSmallUTxO ({}) ({})",
                    expected.to_haskell_str(),
                    supplied.to_haskell_str()
                )
            }
            ValueNotConservedUTxO(expected, supplied) => {
                write!(f, "ValueNotConservedUTxO ({}) ({})", expected, supplied)
            }
            WrongNetwork(network, addrs) => {
                write!(
                    f,
                    "WrongNetwork ({}) ({:?})",
                    network.to_haskell_str(),
                    addrs
                )
            }
            WrongNetworkWithdrawal(network, accounts) => write!(
                f,
                "WrongNetworkWithdrawal ({}) ({})",
                network.to_haskell_str(),
                accounts.to_haskell_str()
            ),
            OutputTooSmallUTxO(tx_outs) => {
                write!(f, "OutputTooSmallUTxO {}", tx_outs.to_haskell_str())
            }
            OutputBootAddrAttrsTooBig(outputs) => {
                write!(f, "OutputBootAddrAttrsTooBig {}", outputs.to_haskell_str())
            }
            OutputTooBigUTxO(outputs) => {
                write!(f, "OutputTooBigUTxO ({})", display_triple_vec(outputs))
            }
            InsufficientCollateral(balance, required) => {
                write!(
                    f,
                    "InsufficientCollateral ({}) ({})",
                    balance,
                    required.to_haskell_str()
                )
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
                write!(
                    f,
                    "BabbageNonDisjointRefInputs ({})",
                    inputs.to_haskell_str()
                )
            }
        }
    }
}

// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Gov.hs#L164
// the ones with string are not worked out
#[derive(Debug)]
pub enum ConwayGovPredFailure {
    GovActionsDoNotExist(Vec<GovActionId>), //  (NonEmpty (GovActionId (EraCrypto era)))
    MalformedProposal(GovAction),           // GovAction era
    ProposalProcedureNetworkIdMismatch(RewardAccountFielded, Network), // (RewardAccount (EraCrypto era)) Network
    TreasuryWithdrawalsNetworkIdMismatch(CustomSet258<RewardAccountFielded>, Network), // (Set.Set (RewardAccount (EraCrypto era))) Network
    ProposalDepositIncorrect(DisplayCoin, DisplayCoin), // !(Mismatch 'RelEQ Coin)
    DisallowedVoters(Vec<(Voter, GovActionId)>), // !(NonEmpty (Voter (EraCrypto era), GovActionId (EraCrypto era)))
    ConflictingCommitteeUpdate(CustomSet258<Credential>), // (Set.Set (Credential 'ColdCommitteeRole (EraCrypto era)))
    ExpirationEpochTooSmall(HashMap<StakeCredential, EpochNo>), // Probably wrong credintial type!, epochno
    InvalidPrevGovActionId(ProposalProcedure),                  // (ProposalProcedure era)
    VotingOnExpiredGovAction(Vec<(Voter, GovActionId)>), // (NonEmpty (Voter (EraCrypto era), GovActionId (EraCrypto era)))
    ProposalCantFollow(String), //        (StrictMaybe (GovPurposeId 'HardForkPurpose era)) |
    InvalidPolicyHash(
        StrictMaybe<DisplayScriptHash>,
        StrictMaybe<DisplayScriptHash>,
    ), //        (StrictMaybe (ScriptHash (EraCrypto era)))    (StrictMaybe (ScriptHash (EraCrypto era)))
    DisallowedProposalDuringBootstrap(ProposalProcedure), // (ProposalProcedure era)
    DisallowedVotesDuringBootstrap(Vec<(Voter, GovActionId)>), //        (NonEmpty (Voter (EraCrypto era), GovActionId (EraCrypto era)))
    VotersDoNotExist(Vec<Voter>),                              // (NonEmpty (Voter (EraCrypto era)))
    ZeroTreasuryWithdrawals(GovAction),                        // (GovAction era)
    ProposalReturnAccountDoesNotExist(RewardAccountFielded),   // (RewardAccount (EraCrypto era))
    TreasuryWithdrawalReturnAccountsDoNotExist(Vec<RewardAccountFielded>), //(NonEmpty (RewardAccount (EraCrypto era)))
}

// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Certs.hs#L113
#[derive(Debug)]
pub enum ConwayCertsPredFailure {
    WithdrawalsNotInRewardsCERTS(HashMap<RewardAccountFielded, DisplayCoin>),
    CertFailure(ConwayCertPredFailure),
}

impl fmt::Display for ConwayCertsPredFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ConwayCertsPredFailure::*;

        match self {
            WithdrawalsNotInRewardsCERTS(m) => {
                write!(f, "WithdrawalsNotInRewardsCERTS ({})", display_hashmap(m))
            }
            CertFailure(e) => write!(f, "CertFailure ({})", e),
        }
    }
}

// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Cert.hs#L102
#[derive(Debug)]
pub enum ConwayCertPredFailure {
    DelegFailure(ConwayDelegPredFailure),
    PoolFailure(ShelleyPoolPredFailure), // TODO
    GovCertFailure(ConwayGovCertPredFailure),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/7683b73971a800b36ca7317601552685fa0701ed/eras/shelley/impl/src/Cardano/Ledger/Shelley/Rules/Pool.hs#L91
#[derive(Debug)]
pub enum ShelleyPoolPredFailure {
    StakePoolNotRegisteredOnKeyPOOL(KeyHash),
    StakePoolRetirementWrongEpochPOOL(Mismatch<EpochNo>, Mismatch<EpochNo>),
    StakePoolCostTooLowPOOL(Mismatch<DisplayCoin>),
    WrongNetworkPOOL(Mismatch<Network>, KeyHash),
    PoolMedataHashTooBig(KeyHash, u64),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/GovCert.hs#L118C6-L118C30
#[derive(Debug)]
pub enum ConwayGovCertPredFailure {
    ConwayDRepAlreadyRegistered(Credential),
    ConwayDRepNotRegistered(Credential),
    ConwayDRepIncorrectDeposit(DisplayCoin, DisplayCoin),
    ConwayCommitteeHasPreviouslyResigned(Credential),
    ConwayDRepIncorrectRefund(DisplayCoin, DisplayCoin),
    ConwayCommitteeIsUnknown(Credential),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/b14ba8190e21ced6cc68c18a02dd1dbc2ff45a3c/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Deleg.hs#L104
#[derive(Debug)]
pub enum ConwayDelegPredFailure {
    IncorrectDepositDELEG(DisplayCoin),
    StakeKeyRegisteredDELEG(Credential),
    StakeKeyNotRegisteredDELEG(Credential),
    StakeKeyHasNonZeroRewardAccountBalanceDELEG(DisplayCoin),
    DelegateeDRepNotRegisteredDELEG(Credential),
    DelegateeStakePoolNotRegisteredDELEG(KeyHash),
}

// this type can be used inside a StrictMaybe
#[derive(Debug, Decode)]
#[cbor(transparent)]

pub struct DisplayScriptHash(#[n(0)] pub ScriptHash);

// https://github.com/IntersectMBO/cardano-ledger/blob/f54489071f4faa4b6209e1ba5288507c824cca50/libs/cardano-ledger-core/src/Cardano/Ledger/Address.hs
// the bytes are not decoded
pub type Addr = Bytes;

// https://github.com/IntersectMBO/cardano-ledger/blob/78b20b6301b2703aa1fe1806ae3c129846708a10/eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Scripts.hs#L497
// not tested yet
#[derive(Debug)]
pub enum PlutusPurpose {
    Spending(AsIx),   // 0
    Minting(AsIx),    // 1
    Certifying(AsIx), // 2
    Rewarding(AsIx),  // 3
    Voting(AsIx),
    Proposing(AsIx),
}
#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct AsIx(#[n(0)] pub u16);

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct Array<T>(#[n(0)] pub Vec<T>);

// https://github.com/IntersectMBO/cardano-ledger/blob/78b20b6301b2703aa1fe1806ae3c129846708a10/libs/cardano-ledger-core/src/Cardano/Ledger/BaseTypes.hs#L779
#[derive(Debug, Serialize, PartialEq, Eq)]
pub enum Network {
    Mainnet,
    Testnet,
}

impl HaskellDisplay for Network {
    fn to_haskell_str(&self) -> String {
        match self {
            Self::Mainnet => "Mainnet".to_string(),
            Self::Testnet => "Testnet".to_string(),
        }
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
        write!(f, "Utxo({})", self.0.to_haskell_str())
    }
}

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct SerializableTxIn(#[n(0)] pub TxIn);

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct SerializableTxOut(#[n(0)] pub TxOut);

// https://github.com/IntersectMBO/cardano-ledger/blob/ea1d4362226d29ce7e42f4ba83ffeecedd9f0565/libs/cardano-ledger-core/src/Cardano/Ledger/Address.hs#L383C9-L383C20
#[derive(Debug)]
pub struct CompactAddr();

#[derive(Debug)]
pub struct CompactForm();
#[derive(Debug)]
pub struct Addr28Extra(u64, u64, u64, u64);
#[derive(Debug)]

pub struct DataHash32(u64, u64, u64, u64);

// https://github.com/IntersectMBO/cardano-ledger/blob/master/eras/conway/impl/src/Cardano/Ledger/Conway/TxOut.hs
// https://github.com/IntersectMBO/cardano-ledger/blob/0d20d716fc15dc0b7648c448cbd735bebb7521b8/eras/babbage/impl/src/Cardano/Ledger/Babbage/TxOut.hs#L130
#[derive(Debug)]
pub enum BabbageTxOut {
    TxOutCompact(CompactAddr, CompactForm),
    TxOutCompactDH(CompactAddr, CompactForm, DataHash32),
    TxOutCompactDatum(CompactAddr, CompactForm, Bytes),
    TxOutCompactRefScript(
        Address,
        (MaryValue, MultiAsset),
        DatumEnum,
        StrictMaybe<EraScript>,
    ), // is DatumHash and ScriptHash correct?
    TxOutAddrHash28AdaOnly(Credential, Addr28Extra, CompactForm),
    TxOutAddrHash28AdaOnlyDataHash32(Credential, Addr28Extra, CompactForm, DataHash32),
    NotImplemented,
}
#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct AddressBytes(#[n(0)] pub Bytes);

// https://github.com/IntersectMBO/cardano-ledger/blob/ea1d4362226d29ce7e42f4ba83ffeecedd9f0565/eras/conway/impl/src/Cardano/Ledger/Conway/TxOut.hs#L34
// https://github.com/IntersectMBO/cardano-ledger/blob/ea1d4362226d29ce7e42f4ba83ffeecedd9f0565/eras/babbage/impl/src/Cardano/Ledger/Babbage/TxOut.hs#L130
pub enum ConwayTxOut {}
// https://github.com/IntersectMBO/cardano-ledger/blob/ea1d4362226d29ce7e42f4ba83ffeecedd9f0565/eras/mary/impl/src/Cardano/Ledger/Mary/Value.hs#L162C9-L162C19
#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct MultiAsset(#[n(0)] pub HashMap<DisplayPolicyId, HashMap<DisplayAssetName, u64>>);

#[derive(Debug, Decode, Hash, PartialEq, Eq)]
#[cbor(transparent)]
pub struct DisplayPolicyId(#[n(0)] pub PolicyId);

#[derive(Debug, Decode, Hash, PartialEq, Eq)]
#[cbor(transparent)]
pub struct DisplayAssetName(#[n(0)] pub AssetName);

// https://github.com/IntersectMBO/cardano-ledger/blob/ea1d4362226d29ce7e42f4ba83ffeecedd9f0565/eras/allegra/impl/src/Cardano/Ledger/Allegra/Scripts.hs#L135
// https://github.com/IntersectMBO/cardano-ledger/blob/ea1d4362226d29ce7e42f4ba83ffeecedd9f0565/eras/allegra/impl/src/Cardano/Ledger/Allegra/Scripts.hs#L210
// We can ignore MemoBytes datatype
#[derive(Debug)]
pub enum TimelockRaw {
    Signature(KeyHash),
    AllOf(Vec<Timelock>),
    AnyOf(Vec<Timelock>),
    MOfN(u8, Vec<Timelock>),
    TimeStart(SlotNo),
    TimeExpire(SlotNo),
}

#[derive(Debug)]
pub struct Timelock {
    pub raw: TimelockRaw,
    pub memo: DisplayHash,
}

#[derive(Debug)]
pub enum EraScript {
    Native(Timelock),
    PlutusV1(ScriptHash),
    PlutusV2(ScriptHash),
    PlutusV3(ScriptHash),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/7683b73971a800b36ca7317601552685fa0701ed/libs/cardano-ledger-core/src/Cardano/Ledger/Hashes.hs#L113
#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct DisplayHash(#[n(0)] pub Blake2b256); // Hashing algorithm used for hashing everything, except addresses

// https://github.com/IntersectMBO/cardano-ledger/blob/master/libs/cardano-ledger-core/src/Cardano/Ledger/MemoBytes/Internal.hs
#[derive(Debug)]
pub struct MemoBytes(Bytes);

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct StrictSeq<T>(#[n(0)] pub Vec<T>);

// https://github.com/IntersectMBO/cardano-ledger/blob/5aed6e50d9efc9443ec2c17197671cc4c0de5498/libs/cardano-ledger-core/src/Cardano/Ledger/Plutus/Data.hs#L206
#[derive(Debug)]

pub enum DatumEnum {
    DatumHash(DisplayDatumHash),
    Datum(DisplayDatum),
    NoDatum,
}
#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct DisplayDatumHash(#[n(0)] pub DatumHash);

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct DisplayDatum(#[n(0)] pub Datum);

impl fmt::Display for SerializableTxOut {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl fmt::Display for DisplayCoin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Coin {}", self.0)
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
pub struct DisplayCoin(#[n(0)] pub Coin);

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct EpochNo(#[n(0)] pub u64);

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct DeltaCoin(#[n(0)] Coin);

#[derive(Debug)]
//#[cbor(transparent)]
pub struct Mismatch<T>(pub T, pub T)
where
    T: HaskellDisplay;
// Decode<'static, ()> +

#[derive(Debug)]
pub enum StrictMaybe<T: HaskellDisplay> {
    Just(T),
    Nothing,
}

impl From<ScriptHash> for StrictMaybe<ScriptHash> {
    fn from(item: ScriptHash) -> Self {
        match item.len() {
            0 => StrictMaybe::Nothing,
            _ => StrictMaybe::Just(item),
        }
    }
}
impl From<&[u8]> for StrictMaybe<ScriptHash> {
    fn from(bytes: &[u8]) -> Self {
        match bytes.len() {
            0 => StrictMaybe::Nothing,
            _ => StrictMaybe::Just(bytes.into()),
        }
    }
}

impl fmt::Display for DeltaCoin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeltaCoin {}", self.0)
    }
}

pub struct InvalidPrevGovActionId(ProposalProcedure);

/*
// https://github.com/IntersectMBO/cardano-ledger/blob/730c811b7a0ee0301d013555091e7394c77c3b19/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Procedures.hs#L476
#[derive(Debug)]
pub struct ProposalProcedure {
    p_proc_deposit: DisplayCoin,
    p_proc_return_addr: RewardAccountFielded,
    p_proc_gov_action: GovAction,
    pProcAnchor: Anchor

}
 */

// RewardAcount is serialized into bytes: https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/libs/cardano-ledger-core/src/Cardano/Ledger/Address.hs#L135
#[derive(Debug, PartialEq, Eq)]
pub struct RewardAccountFielded {
    pub ra_network: Network,
    pub ra_credential: StakeCredential,
}

impl RewardAccountFielded {
    pub fn new(hex: String) -> Self {
        /*  let ra_network = if hex.starts_with("e0") {
                    Network::Testnet
                } else {
                    Network::Mainnet
                };
        */
        let bytes = hex::decode(&hex).expect("Invalid hex string");

        let (ra_network, ra_credential) = get_network_and_credentials(&bytes);
        Self {
            ra_network,
            ra_credential,
        }
    }
}

impl std::hash::Hash for RewardAccountFielded {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ra_credential.hash(state);
    }
}

#[derive(Debug, Decode, Hash, Eq, PartialEq)]
#[cbor(transparent)]
pub struct DisplayStakeCredential(#[n(0)] pub StakeCredential);

impl fmt::Display for DisplayStakeCredential {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "StakeCredential({:?})", self.0)
    }
}

// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/libs/cardano-ledger-core/src/Cardano/Ledger/Credential.hs#L82
#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Credential {
    ScriptHashObj(ScriptHash),
    KeyHashObj(AddrKeyhash),
}

#[derive(Debug, Decode, Hash, PartialEq, Eq)]
#[cbor(transparent)]
pub struct KeyHash(#[n(0)] pub Bytes);

#[derive(Debug, Decode, Hash, PartialEq, Eq)]
#[cbor(transparent)]
pub struct VKey(#[n(0)] pub Bytes);

#[derive(Debug, Decode, Hash, PartialEq, Eq, Clone)]
#[cbor(transparent)]
pub struct SafeHash(#[n(0)] pub Bytes);

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

#[derive(Debug, Decode)]
#[cbor(array)]

pub struct AAAProposalProcedure {
    #[n(0)]
    pub deposit: Coin,
    #[n(1)]
    pub reward_account: RewardAccount,
    // #[n(2)]pub gov_action: GovAction,
    #[n(3)]
    pub anchor: Anchor,
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

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct MaryValue(#[n(0)] pub DisplayCoin);

// this handles the custom type with tag 258: https://github.com/input-output-hk/cbor-sets-spec/blob/master/CBOR_SETS.md
#[derive(Debug)]
pub struct CustomSet258<T>(pub Vec<T>);

/*
**Helper functions for Display'ing the types.
*/
fn display_tuple<T: Display, U: Display>(t: &(T, U)) -> String {
    format!("({},{})", t.0, t.1)
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

fn display_option<T: Display>(opt: &Option<T>) -> String {
    match opt {
        Some(x) => format!("{}", x),
        None => "None".to_string(),
    }
}

fn display_hashmap<K: Display, V: Display>(map: &HashMap<K, V>) -> String {
    let entries: Vec<String> = map.iter().map(|t| display_tuple(&t)).collect();
    format!("fromList [{}]", entries.join(" "))
}

fn display_bytes_as_key_hash(b: &Bytes) -> String {
    format!("KeyHash {{unKeyHash = \"{}\"}}", b)
}

fn display_bytes_vector_as_key_hash(v: &Vec<Bytes>) -> String {
    let mut result = String::new();
    for b in v {
        result.push_str(&format!("KeyHash {{unKeyHash = \"{}\"}}", b));
        result.push_str(" :| []");
    }
    result.pop();
    result
}

fn display_strict_maybe<T: HaskellDisplay>(maybe: &StrictMaybe<T>) -> String {
    use StrictMaybe::*;

    match maybe {
        Just(t) => format!("SJust ({})", t.to_haskell_str()),
        Nothing => "SNothing".to_string(),
    }
}

/**
 * Instead of this function, we can use Address type directly from pallas and decorate it with HaskellDisplay implementations
 */
pub fn get_network_and_credentials(bytes: &[u8]) -> (Network, StakeCredential) {
    let network = if bytes[0] & 0b00000001 != 0 {
        // Is Mainnet Address
        Network::Mainnet
    } else {
        Network::Testnet
    };

    let mut hash = [0; 28];
    hash.copy_from_slice(&bytes[1..29]);
    let credential = if &bytes[0] & 0b00010000 != 0 {
        // Credential is a Script
        StakeCredential::ScriptHash(hash.into())
    } else {
        StakeCredential::AddrKeyhash(hash.into())
    };

    (network, credential)
}
