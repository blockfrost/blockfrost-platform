#![allow(dead_code)]

use std::fmt::{self};

use pallas_addresses::Address;
use pallas_codec::minicbor::{self, Decode};
use pallas_codec::utils::Bytes;
use pallas_primitives::{
    byron::Blake2b256,
    conway::{
        Anchor, Certificate, CommitteeColdCredential, Constitution, DRep, DRepVotingThresholds,
        DatumHash, ExUnitPrices, ExUnits, GovActionId, Language, PoolVotingThresholds,
        ProposalProcedure, ScriptHash, Voter, VotingProcedures,
    },
    AddrKeyhash, AssetName, BoundedBytes, Coin, CostModel, Epoch, KeyValuePairs, Nullable,
    PolicyId, PoolKeyhash, ProtocolVersion, RationalNumber, RewardAccount, Set, StakeCredential,
    TransactionInput, UnitInterval,
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
                    s1.to_haskell_str_p(),
                    s2.to_haskell_str_p()
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
    MissingVKeyWitnessesUTXOW(Set<KeyHash>),
    MissingScriptWitnessesUTXOW(Set<ScriptHash>),
    ScriptWitnessNotValidatingUTXOW(Set<ScriptHash>),
    MissingTxBodyMetadataHash(Bytes),      // auxDataHash
    MissingTxMetadata(Bytes),              // auxDataHash
    ConflictingMetadataHash(Bytes, Bytes), // Mismatch auxDataHash
    InvalidMetadata(),                     // empty
    ExtraneousScriptWitnessesUTXOW(Set<ScriptHash>),
    MissingRedeemers(Array<(ConwayPlutusPurpose, ScriptHash)>),
    MissingRequiredDatums(Set<SafeHash>, Set<SafeHash>), // set of missing data hashes, set of recieved data hashes
    NotAllowedSupplementalDatums(Set<SafeHash>, Set<SafeHash>), // set of unallowed data hashes, set of acceptable data hashes
    PPViewHashesDontMatch(StrictMaybe<SafeHash>, StrictMaybe<SafeHash>),
    UnspendableUTxONoDatumHash(Set<TransactionInput>), //  Set of transaction inputs that are TwoPhase scripts, and should have a DataHash but don't
    ExtraRedeemers(Array<PlutusPurpose>),              // List of redeemers not needed
    MalformedScriptWitnesses(Set<ScriptHash>),
    MalformedReferenceScripts(Set<ScriptHash>),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/7683b73971a800b36ca7317601552685fa0701ed/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxo.hs#L315
#[derive(Debug)]
pub enum ConwayUtxoPredFailure {
    UtxosFailure(ConwayUtxosPredFailure),
    BadInputsUTxO(Set<TransactionInput>),
    OutsideValidityIntervalUTxO(ValidityInterval, SlotNo), // validity interval, current slot
    MaxTxSizeUTxO(i64, i64),                               // less than or equal
    InputSetEmptyUTxO(),                                   // empty
    FeeTooSmallUTxO(DisplayCoin, DisplayCoin),             // Mismatch expected, supplied
    ValueNotConservedUTxO(DisplayValue, DisplayValue),
    WrongNetwork(Network, Set<DisplayAddress>), // the expaected network id,  the set of addresses with incorrect network IDs
    WrongNetworkWithdrawal(Network, Set<RewardAccountFielded>), // the expected network id ,  the set of reward addresses with incorrect network IDs
    OutputTooSmallUTxO(Array<BabbageTxOut>),
    OutputBootAddrAttrsTooBig(Array<BabbageTxOut>),
    OutputTooBigUTxO(Array<(i8, i8, BabbageTxOut)>), //  list of supplied bad transaction output triples (actualSize,PParameterMaxValue,TxOut)
    InsufficientCollateral(DeltaCoin, DisplayCoin), // balance computed, the required collateral for the given fee
    ScriptsNotPaidUTxO(Utxo), // The UTxO entries which have the wrong kind of script
    ExUnitsTooBigUTxO(ExUnits, ExUnits), // check: The values are serialised in reverse order
    CollateralContainsNonADA(DisplayValue),
    WrongNetworkInTxBody(Network, Network), // take in Network, https://github.com/IntersectMBO/cardano-ledger/blob/78b20b6301b2703aa1fe1806ae3c129846708a10/libs/cardano-ledger-core/src/Cardano/Ledger/BaseTypes.hs#L779
    OutsideForecast(SlotNo),
    TooManyCollateralInputs(u64, u64), // this is Haskell Natural, how many bit is it?
    NoCollateralInputs(),              // empty
    IncorrectTotalCollateralField(DeltaCoin, DisplayCoin), // collateral provided, collateral amount declared in transaction body
    BabbageOutputTooSmallUTxO(Array<(BabbageTxOut, DisplayCoin)>), // list of supplied transaction outputs that are too small, together with the minimum value for the given output
    BabbageNonDisjointRefInputs(Vec<TransactionInput>), // TxIns that appear in both inputs and reference inputs
}

// https://github.com/IntersectMBO/cardano-ledger/blob/5fda7bbf778fb110bd28b306147da3e287ace124/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxos.hs#L73
#[derive(Debug)]
pub enum ConwayUtxosPredFailure {
    ValidationTagMismatch(bool, TagMismatchDescription),
    CollectErrors(Array<CollectError>),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/bc10beb0038319354eefae31baf381193c5f4e32/libs/cardano-ledger-core/src/Cardano/Ledger/Plutus/CostModels.hs#L107
#[derive(Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(transparent)]
pub struct DisplayCostModels(#[n(0)] pub OHashMap<i64, CostModel>);

#[derive(Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(map)]
pub struct DisplayProtocolParamUpdate {
    #[n(0)]
    pub minfee_a: Option<u64>,
    #[n(1)]
    pub minfee_b: Option<u64>,
    #[n(2)]
    pub max_block_body_size: Option<u64>,
    #[n(3)]
    pub max_transaction_size: Option<u64>,
    #[n(4)]
    pub max_block_header_size: Option<u64>,
    #[n(5)]
    pub key_deposit: Option<Coin>,
    #[n(6)]
    pub pool_deposit: Option<Coin>,
    #[n(7)]
    pub maximum_epoch: Option<Epoch>,
    #[n(8)]
    pub desired_number_of_stake_pools: Option<u64>,
    #[n(9)]
    pub pool_pledge_influence: Option<RationalNumber>,
    #[n(10)]
    pub expansion_rate: Option<UnitInterval>,
    #[n(11)]
    pub treasury_growth_rate: Option<UnitInterval>,

    #[n(16)]
    pub min_pool_cost: Option<Coin>,
    #[n(17)]
    pub ada_per_utxo_byte: Option<Coin>,
    #[n(18)]
    pub cost_models_for_script_languages: Option<DisplayCostModels>,
    #[n(19)]
    pub execution_costs: Option<ExUnitPrices>,
    #[n(20)]
    pub max_tx_ex_units: Option<ExUnits>,
    #[n(21)]
    pub max_block_ex_units: Option<ExUnits>,
    #[n(22)]
    pub max_value_size: Option<u64>,
    #[n(23)]
    pub collateral_percentage: Option<u64>,
    #[n(24)]
    pub max_collateral_inputs: Option<u64>,

    #[n(25)]
    pub pool_voting_thresholds: Option<PoolVotingThresholds>,
    #[n(26)]
    pub drep_voting_thresholds: Option<DRepVotingThresholds>,
    #[n(27)]
    pub min_committee_size: Option<u64>,
    #[n(28)]
    pub committee_term_limit: Option<Epoch>,
    #[n(29)]
    pub governance_action_validity_period: Option<Epoch>,
    #[n(30)]
    pub governance_action_deposit: Option<Coin>,
    #[n(31)]
    pub drep_deposit: Option<Coin>,
    #[n(32)]
    pub drep_inactivity_period: Option<Epoch>,
    #[n(33)]
    pub minfee_refscript_cost_per_byte: Option<UnitInterval>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DisplayGovAction {
    ParameterChange(
        Nullable<GovActionId>,
        Box<DisplayProtocolParamUpdate>,
        Nullable<ScriptHash>,
    ),
    HardForkInitiation(Nullable<GovActionId>, ProtocolVersion),
    TreasuryWithdrawals(KeyValuePairs<RewardAccount, Coin>, Nullable<ScriptHash>),
    NoConfidence(Nullable<GovActionId>),
    UpdateCommittee(
        Nullable<GovActionId>,
        Set<CommitteeColdCredential>,
        KeyValuePairs<CommitteeColdCredential, Epoch>,
        UnitInterval,
    ),
    NewConstitution(Nullable<GovActionId>, Constitution),
    Information,
}

// https://github.com/IntersectMBO/cardano-ledger/blob/09dc3774a434677ece12910b2c1c409de4cc2656/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Procedures.hs#L487
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DisplayProposalProcedure {
    pub deposit: Coin,
    pub reward_account: RewardAccount,
    pub gov_action: DisplayGovAction,
    pub anchor: Anchor,
}

// https://github.com/IntersectMBO/cardano-ledger/blob/562ee0869bd40e2386e481b42602fc64121f6a01/eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxos.hs#L365
#[derive(Debug)]
pub enum TagMismatchDescription {
    PassedUnexpectedly,
    FailedUnexpectedly(Vec<FailureDescription>),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/562ee0869bd40e2386e481b42602fc64121f6a01/eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxos.hs#L332
#[derive(Debug)]
pub enum FailureDescription {
    PlutusFailure(String, Bytes),
}

/* #[derive(Debug, Decode)]
pub enum CollectError{
    #[n(0)] NoRedeemer(#[n(0)] ConwayPlutusPurpose),
    #[n(1)] NoWitness(#[n(0)] DisplayScriptHash),
    #[n(2)] NoCostModel(#[n(0)] Language),
    #[n(3)] BadTranslation(#[n(0)] ConwayContextError),
}

 */

// https://github.com/IntersectMBO/cardano-ledger/blob/bc10beb0038319354eefae31baf381193c5f4e32/eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Plutus/Evaluate.hs#L77

#[derive(Debug)]
pub enum CollectError {
    NoRedeemer(ConwayPlutusPurpose),
    NoWitness(DisplayScriptHash),
    NoCostModel(Language),
    BadTranslation(ConwayContextError),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/bc10beb0038319354eefae31baf381193c5f4e32/eras/conway/impl/src/Cardano/Ledger/Conway/TxInfo.hs#L155
#[derive(Debug)]
pub enum ConwayContextError {
    BabbageContextError(BabbageContextError),
    CertificateNotSupported(ConwayTxCert),
    PlutusPurposeNotSupported(ConwayPlutusPurpose),
    CurrentTreasuryFieldNotSupported(DisplayCoin),
    VotingProceduresFieldNotSupported(DisplayVotingProcedures),
    ProposalProceduresFieldNotSupported(DisplayOSet<DisplayProposalProcedure>), // is Vec == OSet ?
    TreasuryDonationFieldNotSupported(DisplayCoin),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/bc10beb0038319354eefae31baf381193c5f4e32/eras/babbage/impl/src/Cardano/Ledger/Babbage/TxInfo.hs#L241
// https://github.com/IntersectMBO/cardano-ledger/blob/bc10beb0038319354eefae31baf381193c5f4e32/eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Plutus/TxInfo.hs#L175
// Flattened AlonzoContextError error into n1 and n7
#[derive(Debug)]
pub enum BabbageContextError {
    ByronTxOutInContext(TxOutSource),
    AlonzoMissingInput(TransactionInput),
    RedeemerPointerPointsToNothing(PlutusPurpose),
    InlineDatumsNotSupported(TxOutSource),
    ReferenceScriptsNotSupported(TxOutSource),
    ReferenceInputsNotSupported(Set<TransactionInput>),
    AlonzoTimeTranslationPastHorizon(String),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/09dc3774a434677ece12910b2c1c409de4cc2656/libs/cardano-ledger-core/src/Cardano/Ledger/Plutus/TxInfo.hs#L91
#[derive(Debug)]
pub enum TxOutSource {
    TxOutFromInput(TransactionInput),
    TxOutFromOutput(TxIx),
}

impl fmt::Display for ConwayUtxoPredFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ConwayUtxoPredFailure::*;

        match self {
            UtxosFailure(e) => write!(f, "(UtxosFailure {})", e.to_haskell_str_p()),
            BadInputsUTxO(e) => write!(f, "(BadInputsUTxO ({}))", e.to_haskell_str()),
            OutsideValidityIntervalUTxO(vi, slot) => {
                write!(
                    f,
                    "(OutsideValidityIntervalUTxO {} {})",
                    vi.to_haskell_str(),
                    slot.to_haskell_str_p()
                )
            }
            MaxTxSizeUTxO(n1, n2) => write!(
                f,
                "(MaxTxSizeUTxO {} {})",
                n1.to_haskell_str(),
                n2.to_haskell_str()
            ),
            InputSetEmptyUTxO() => write!(f, "InputSetEmptyUTxO"),
            FeeTooSmallUTxO(expected, supplied) => {
                write!(
                    f,
                    "(FeeTooSmallUTxO ({}) ({}))",
                    expected.to_haskell_str(),
                    supplied.to_haskell_str()
                )
            }
            ValueNotConservedUTxO(expected, supplied) => {
                write!(
                    f,
                    "(ValueNotConservedUTxO ({}) ({}))",
                    expected.to_haskell_str(),
                    supplied.to_haskell_str()
                )
            }
            WrongNetwork(network, addrs) => {
                write!(
                    f,
                    "(WrongNetwork {} {})",
                    network.to_haskell_str(),
                    addrs.to_haskell_str_p()
                )
            }
            WrongNetworkWithdrawal(network, accounts) => write!(
                f,
                "(WrongNetworkWithdrawal {} {})",
                network.to_haskell_str(),
                accounts.to_haskell_str_p()
            ),
            OutputTooSmallUTxO(tx_outs) => {
                write!(f, "(OutputTooSmallUTxO {})", tx_outs.to_haskell_str_p())
            }
            OutputBootAddrAttrsTooBig(outputs) => {
                write!(
                    f,
                    "(OutputBootAddrAttrsTooBig {})",
                    outputs.to_haskell_str_p()
                )
            }
            OutputTooBigUTxO(outputs) => {
                write!(f, "(OutputTooBigUTxO {})", outputs.to_haskell_str())
            }
            InsufficientCollateral(balance, required) => {
                write!(
                    f,
                    "(InsufficientCollateral ({}) ({}))",
                    balance.to_haskell_str(),
                    required.to_haskell_str()
                )
            }
            ScriptsNotPaidUTxO(utxo) => {
                write!(f, "(ScriptsNotPaidUTxO {})", utxo.to_haskell_str_p())
            }
            ExUnitsTooBigUTxO(u1, u2) => write!(
                f,
                "(ExUnitsTooBigUTxO {} {})",
                u1.to_haskell_str_p(),
                u2.to_haskell_str_p()
            ),
            CollateralContainsNonADA(value) => {
                write!(f, "(CollateralContainsNonADA ({}))", value.to_haskell_str())
            }
            WrongNetworkInTxBody(n1, n2) => write!(
                f,
                "(WrongNetworkInTxBody {} {})",
                n1.to_haskell_str(),
                n2.to_haskell_str()
            ),
            OutsideForecast(slot) => write!(f, "(OutsideForecast ({}))", slot.to_haskell_str()),
            TooManyCollateralInputs(i1, i2) => write!(f, "(TooManyCollateralInputs {} {})", i1, i2),
            NoCollateralInputs() => write!(f, "NoCollateralInputs"),
            IncorrectTotalCollateralField(provided, declared) => write!(
                f,
                "(IncorrectTotalCollateralField {} {})",
                provided.to_haskell_str_p(),
                declared.to_haskell_str_p()
            ),
            BabbageOutputTooSmallUTxO(outputs) => {
                write!(
                    f,
                    "(BabbageOutputTooSmallUTxO {})",
                    outputs.to_haskell_str_p()
                )
            }
            BabbageNonDisjointRefInputs(inputs) => {
                write!(
                    f,
                    "(BabbageNonDisjointRefInputs ({}))",
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
    MalformedProposal(DisplayGovAction),    // GovAction era
    ProposalProcedureNetworkIdMismatch(RewardAccountFielded, Network), // (RewardAccount (EraCrypto era)) Network
    TreasuryWithdrawalsNetworkIdMismatch(Set<RewardAccountFielded>, Network), // (Set.Set (RewardAccount (EraCrypto era))) Network
    ProposalDepositIncorrect(DisplayCoin, DisplayCoin), // !(Mismatch 'RelEQ Coin)
    DisallowedVoters(Vec<(Voter, GovActionId)>), // !(NonEmpty (Voter (EraCrypto era), GovActionId (EraCrypto era)))
    ConflictingCommitteeUpdate(Set<Credential>), // (Set.Set (Credential 'ColdCommitteeRole (EraCrypto era)))
    ExpirationEpochTooSmall(OHashMap<StakeCredential, EpochNo>), // Probably wrong credintial type!, epochno
    InvalidPrevGovActionId(DisplayProposalProcedure),            // (ProposalProcedure era)
    VotingOnExpiredGovAction(Vec<(Voter, GovActionId)>), // (NonEmpty (Voter (EraCrypto era), GovActionId (EraCrypto era)))
    ProposalCantFollow(StrictMaybe<GovActionId>, ProtocolVersion, ProtocolVersion), //        (StrictMaybe (GovPurposeId 'HardForkPurpose era)) |
    InvalidPolicyHash(
        StrictMaybe<DisplayScriptHash>,
        StrictMaybe<DisplayScriptHash>,
    ), //        (StrictMaybe (ScriptHash (EraCrypto era)))    (StrictMaybe (ScriptHash (EraCrypto era)))
    DisallowedProposalDuringBootstrap(DisplayProposalProcedure), // (ProposalProcedure era)
    DisallowedVotesDuringBootstrap(Vec<(Voter, GovActionId)>), //        (NonEmpty (Voter (EraCrypto era), GovActionId (EraCrypto era)))
    VotersDoNotExist(Vec<Voter>),                              // (NonEmpty (Voter (EraCrypto era)))
    ZeroTreasuryWithdrawals(DisplayGovAction),                 // (GovAction era)
    ProposalReturnAccountDoesNotExist(RewardAccountFielded),   // (RewardAccount (EraCrypto era))
    TreasuryWithdrawalReturnAccountsDoNotExist(Vec<RewardAccountFielded>), //(NonEmpty (RewardAccount (EraCrypto era)))
}

// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Certs.hs#L113
#[derive(Debug)]
pub enum ConwayCertsPredFailure {
    WithdrawalsNotInRewardsCERTS(OHashMap<RewardAccountFielded, DisplayCoin>),
    CertFailure(ConwayCertPredFailure),
}

// HashMap loses the CBOR ordering when decoded
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct OHashMap<K, V>(pub Vec<(K, V)>);

impl fmt::Display for ConwayCertsPredFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ConwayCertsPredFailure::*;

        match self {
            WithdrawalsNotInRewardsCERTS(m) => {
                write!(f, "WithdrawalsNotInRewardsCERTS ({})", m.to_haskell_str())
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
    PoolMedataHashTooBig(KeyHash, i8),
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

// https://github.com/IntersectMBO/cardano-ledger/blob/562ee0869bd40e2386e481b42602fc64121f6a01/libs/cardano-ledger-core/src/Cardano/Ledger/Address.hs#L148
#[derive(Debug)]
pub struct DisplayAddress(pub Address);

impl DisplayAddress {
    pub fn from_bytes(bytes: &Bytes) -> Self {
        Self(Address::from_bytes(bytes).unwrap())
    }
}

// https://github.com/IntersectMBO/cardano-ledger/blob/f54489071f4faa4b6209e1ba5288507c824cca50/libs/cardano-ledger-core/src/Cardano/Ledger/Address.hs
// the bytes are not decoded
pub type Addr = Bytes;

// https://github.com/IntersectMBO/cardano-ledger/blob/5fda7bbf778fb110bd28b306147da3e287ace124/eras/conway/impl/src/Cardano/Ledger/Conway/Scripts.hs#L200
// not tested yet
#[derive(Debug)]
pub enum PlutusPurpose {
    Spending(AsIx),       // 0
    Minting(AsIx),        // 1
    Certifying(AsIx),     // 2
    Rewarding(PurposeAs), // 3
    Voting(PurposeAs),
    Proposing(AsIx),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/5fda7bbf778fb110bd28b306147da3e287ace124/eras/conway/impl/src/Cardano/Ledger/Conway/Scripts.hs#L200
// not tested yet
#[derive(Debug)]
pub enum ConwayPlutusPurpose {
    ConwaySpending(AsItem<TransactionInput>),      // 0
    ConwayMinting(AsItem<DisplayPolicyId>),        // 1
    ConwayCertifying(AsItem<ConwayTxCert>),        // 2
    ConwayRewarding(AsItem<RewardAccountFielded>), // 3
    ConwayVoting(AsItem<Voter>),
    ConwayProposing(AsItem<DisplayProposalProcedure>),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/562ee0869bd40e2386e481b42602fc64121f6a01/eras/conway/impl/src/Cardano/Ledger/Conway/TxCert.hs#L587
#[derive(Debug)]
pub enum ConwayTxCert {
    ConwayTxCertDeleg(Certificate),
    ConwayTxCertPool(Certificate),
    ConwayTxCertGov(Certificate),
}
#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct DisplayVotingProcedures(#[n(0)] pub VotingProcedures);

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct AsIx(#[n(0)] pub u64);

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct TxIx(#[n(0)] pub u64);

// https://github.com/IntersectMBO/cardano-ledger/blob/562ee0869bd40e2386e481b42602fc64121f6a01/eras/conway/impl/src/Cardano/Ledger/Conway/TxCert.hs#L357
#[derive(Debug)]
pub enum Delegatee {
    DelegStake(PoolKeyhash),
    DelegVote(DRep),
    DelegStakeVote(KeyHash, DRep),
}

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct AsItem<T>(#[n(0)] pub T)
where
    T: HaskellDisplay;

#[derive(Debug)]
pub enum PurposeAs {
    Ix(AsIx),
    Item(AsItem<RewardAccountFielded>),
}

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct Array<T>(#[n(0)] pub Vec<T>);

// https://github.com/IntersectMBO/cardano-ledger/blob/78b20b6301b2703aa1fe1806ae3c129846708a10/libs/cardano-ledger-core/src/Cardano/Ledger/BaseTypes.hs#L779
#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
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
#[derive(Debug)]
pub struct ValidityInterval {
    pub invalid_before: Option<SlotNo>,
    pub invalid_hereafter: Option<SlotNo>,
}

// https://github.com/IntersectMBO/cardano-ledger/blob/aed1dc28b98c25ea73bc692e7e6c6d3a22381ff5/libs/cardano-ledger-core/src/Cardano/Ledger/UTxO.hs#L83
#[derive(Debug)]
pub struct Utxo(pub OHashMap<TransactionInput, BabbageTxOut>);

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
/*// https://github.com/IntersectMBO/cardano-ledger/blob/0d20d716fc15dc0b7648c448cbd735bebb7521b8/eras/babbage/impl/src/Cardano/Ledger/Babbage/TxOut.hs#L130
#[derive(Debug, Decode)]
#[cbor(map)]
// PseudoPostAlonzoTransactionOutput in pallas
pub struct BabbageTxOut {
    #[n(0)] pub address: DisplayAddress,
    #[n(1)] pub value: Option<DisplayValue>,
    #[n(2)] pub datum: Option<DatumEnum>,
    #[n(3)] pub script: Option<CborWrap<EraScript>>
}*/
#[derive(Debug)]
pub enum DisplayTransactionOutput {
    Legacy(BabbageTxOut),
    PostAlonzo(BabbageTxOut),
}

#[derive(Debug)]
pub struct BabbageTxOut {
    pub address: DisplayAddress,
    pub value: Option<DisplayValue>,
    pub datum: Option<DatumEnum>,
    pub script: Option<EraScript>,
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
pub struct DisplayMultiAsset(
    #[n(0)] pub OHashMap<DisplayPolicyId, OHashMap<DisplayAssetName, u64>>,
);

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

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct StrictSeq<T>(#[n(0)] pub Vec<T>);

// https://github.com/IntersectMBO/cardano-ledger/blob/5aed6e50d9efc9443ec2c17197671cc4c0de5498/libs/cardano-ledger-core/src/Cardano/Ledger/Plutus/Data.hs#L206
#[derive(Debug, Clone)]
pub enum DatumEnum {
    DatumHash(DisplayDatumHash),
    Datum(PlutusDataBytes),
    NoDatum,
}

#[derive(Debug, Clone, Decode)]
#[cbor(transparent)]
pub struct DisplayDatumHash(#[n(0)] pub DatumHash);

impl fmt::Display for DisplayCoin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Coin {}", self.0)
    }
}

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct SlotNo(#[n(0)] pub u64);

// https://github.com/IntersectMBO/ouroboros-consensus/blob/e86b921443bd6e8ea25e7190eb7cb5788e28f4cc/ouroboros-consensus/src/ouroboros-consensus/Ouroboros/Consensus/HardFork/Combinator/AcrossEras.hs#L208
#[derive(Serialize)]
pub struct EraMismatch {
    ledger: String, //  Name of the era of the ledger ("Byron" or "Shelley").
    other: String,  // Era of the block, header, transaction, or query.
}

#[derive(Debug, Decode, Clone)]
#[cbor(transparent)]
pub struct DisplayCoin(#[n(0)] pub Coin);

#[derive(Debug, Decode, Clone)]
#[cbor(transparent)]
pub struct EpochNo(#[n(0)] pub u64);

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct DeltaCoin(#[n(0)] pub i32);

#[derive(Debug)]
//#[cbor(transparent)]
pub struct Mismatch<T>(pub T, pub T)
// supplied, expecte
where
    T: HaskellDisplay;

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

pub struct InvalidPrevGovActionId(ProposalProcedure);

// RewardAcount is serialized into bytes: https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/libs/cardano-ledger-core/src/Cardano/Ledger/Address.hs#L135
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RewardAccountFielded {
    pub ra_network: Network,
    pub ra_credential: StakeCredential,
}

impl RewardAccountFielded {
    pub fn new(hex: String) -> Self {
        let bytes = hex::decode(&hex).expect("Invalid hex string");

        let (ra_network, ra_credential) = get_network_and_credentials(&bytes);
        Self {
            ra_network,
            ra_credential,
        }
    }
}

impl From<&Bytes> for RewardAccountFielded {
    fn from(bytes: &Bytes) -> Self {
        let (ra_network, ra_credential) = get_network_and_credentials(bytes);
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

impl From<&u64> for DisplayCoin {
    fn from(item: &u64) -> Self {
        DisplayCoin(item.to_owned())
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

// todo: This can be replaced by pallas Value
#[derive(Debug)]
pub enum DisplayValue {
    Coin(u64),
    Multiasset(MaryValue, DisplayMultiAsset),
}

// todo CborWrap in pallas
#[derive(Debug)]
pub struct CborBytes<T>(pub T);

#[derive(Debug, Clone)]
pub struct PlutusDataBytes(pub BoundedBytes);

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct MaryValue(#[n(0)] pub DisplayCoin);

// https://github.com/IntersectMBO/cardano-ledger/blob/3dd7401424e8d50cc9f19feef1589f1ce0d83ed6/libs/cardano-data/src/Data/OSet/Strict.hs#L67
#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct DisplayOSet<T>(#[n(0)] pub Set<T>);

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
    if vec.is_empty() {
        return "[]".to_string();
    }

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
