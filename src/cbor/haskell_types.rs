use std::fmt::{self};

use pallas_codec::minicbor::{self, Decode};
use pallas_codec::utils::Bytes;
use pallas_primitives::conway::{GovAction, ProposalProcedure, Value};
use pallas_primitives::NetworkId;
use pallas_primitives::{
    byron::Blake2b256,
    conway::{
        Certificate, DRep, DatumHash, ExUnits, GovActionId, Language, ScriptHash, Voter,
        VotingProcedures,
    },
    AddrKeyhash, AssetName, BoundedBytes, Coin, PolicyId, PoolKeyhash, ProtocolVersion, Set,
    StakeCredential, TransactionInput,
};
use serde::Serialize;
use serde_with::SerializeDisplay;

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

// https://github.com/IntersectMBO/cardano-ledger/blob/f54489071f4faa4b6209e1ba5288507c824cca50/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxow.hs
#[derive(Debug, SerializeDisplay)]
pub enum ConwayUtxoWPredFailure {
    UtxoFailure(ConwayUtxoPredFailure),
    InvalidWitnessesUTXOW(Array<VKey>),
    MissingVKeyWitnessesUTXOW(Set<KeyHash>),
    MissingScriptWitnessesUTXOW(Set<ScriptHash>),
    ScriptWitnessNotValidatingUTXOW(Set<ScriptHash>),
    MissingTxBodyMetadataHash(Bytes),
    MissingTxMetadata(Bytes),
    ConflictingMetadataHash(Bytes, Bytes),
    InvalidMetadata(),
    ExtraneousScriptWitnessesUTXOW(Set<ScriptHash>),
    MissingRedeemers(Array<(ConwayPlutusPurpose, ScriptHash)>),
    MissingRequiredDatums(Set<SafeHash>, Set<SafeHash>),
    NotAllowedSupplementalDatums(Set<SafeHash>, Set<SafeHash>),
    PPViewHashesDontMatch(StrictMaybe<SafeHash>, StrictMaybe<SafeHash>),
    UnspendableUTxONoDatumHash(Set<TransactionInput>),
    ExtraRedeemers(Array<PlutusPurpose>),
    MalformedScriptWitnesses(Set<ScriptHash>),
    MalformedReferenceScripts(Set<ScriptHash>),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/7683b73971a800b36ca7317601552685fa0701ed/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxo.hs#L315
#[derive(Debug)]
pub enum ConwayUtxoPredFailure {
    UtxosFailure(ConwayUtxosPredFailure),
    BadInputsUTxO(Set<TransactionInput>),
    OutsideValidityIntervalUTxO(ValidityInterval, SlotNo),
    MaxTxSizeUTxO(i64, i64),
    InputSetEmptyUTxO(),
    FeeTooSmallUTxO(DisplayCoin, DisplayCoin),
    ValueNotConservedUTxO(Value, Value),
    WrongNetwork(NetworkId, Set<DisplayAddress>),
    WrongNetworkWithdrawal(NetworkId, Set<DisplayRewardAccount>),
    OutputTooSmallUTxO(Array<BabbageTxOut>),
    OutputBootAddrAttrsTooBig(Array<BabbageTxOut>),
    OutputTooBigUTxO(Array<(i8, i8, BabbageTxOut)>),
    InsufficientCollateral(DeltaCoin, DisplayCoin),
    ScriptsNotPaidUTxO(Utxo),
    ExUnitsTooBigUTxO(ExUnits, ExUnits),
    CollateralContainsNonADA(Value),
    WrongNetworkInTxBody(NetworkId, NetworkId),
    OutsideForecast(SlotNo),
    TooManyCollateralInputs(u64, u64),
    NoCollateralInputs(),
    IncorrectTotalCollateralField(DeltaCoin, DisplayCoin),
    BabbageOutputTooSmallUTxO(Array<(BabbageTxOut, DisplayCoin)>),
    BabbageNonDisjointRefInputs(Vec<TransactionInput>),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/5fda7bbf778fb110bd28b306147da3e287ace124/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxos.hs#L73
#[derive(Debug)]
pub enum ConwayUtxosPredFailure {
    ValidationTagMismatch(bool, TagMismatchDescription),
    CollectErrors(Array<CollectError>),
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
    ProposalProceduresFieldNotSupported(DisplayOSet<ProposalProcedure>),
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

// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Gov.hs#L164
#[derive(Debug)]
pub enum ConwayGovPredFailure {
    GovActionsDoNotExist(Vec<GovActionId>),
    MalformedProposal(GovAction),
    ProposalProcedureNetworkIdMismatch(DisplayRewardAccount, NetworkId),
    TreasuryWithdrawalsNetworkIdMismatch(Set<DisplayRewardAccount>, NetworkId),
    ProposalDepositIncorrect(DisplayCoin, DisplayCoin),
    DisallowedVoters(Vec<(Voter, GovActionId)>),
    ConflictingCommitteeUpdate(Set<Credential>),
    ExpirationEpochTooSmall(OHashMap<StakeCredential, EpochNo>),
    InvalidPrevGovActionId(ProposalProcedure),
    VotingOnExpiredGovAction(Vec<(Voter, GovActionId)>),
    ProposalCantFollow(StrictMaybe<GovActionId>, ProtocolVersion, ProtocolVersion),
    InvalidPolicyHash(
        StrictMaybe<DisplayScriptHash>,
        StrictMaybe<DisplayScriptHash>,
    ),
    DisallowedProposalDuringBootstrap(ProposalProcedure),
    DisallowedVotesDuringBootstrap(Vec<(Voter, GovActionId)>),
    VotersDoNotExist(Vec<Voter>),
    ZeroTreasuryWithdrawals(GovAction),
    ProposalReturnAccountDoesNotExist(DisplayRewardAccount),
    TreasuryWithdrawalReturnAccountsDoNotExist(Vec<DisplayRewardAccount>),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Certs.hs#L113
#[derive(Debug)]
pub enum ConwayCertsPredFailure {
    WithdrawalsNotInRewardsCERTS(OHashMap<DisplayRewardAccount, DisplayCoin>),
    CertFailure(ConwayCertPredFailure),
}

/// HashMap loses the CBOR ordering when decoded
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
    WrongNetworkPOOL(Mismatch<NetworkId>, KeyHash),
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
#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct DisplayAddress(#[n(0)] pub Bytes);

// https://github.com/IntersectMBO/cardano-ledger/blob/5fda7bbf778fb110bd28b306147da3e287ace124/eras/conway/impl/src/Cardano/Ledger/Conway/Scripts.hs#L200
#[derive(Debug)]
pub enum PlutusPurpose {
    Spending(AsIx),
    Minting(AsIx),
    Certifying(AsIx),
    Rewarding(PurposeAs),
    Voting(PurposeAs),
    Proposing(AsIx),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/5fda7bbf778fb110bd28b306147da3e287ace124/eras/conway/impl/src/Cardano/Ledger/Conway/Scripts.hs#L200
#[derive(Debug)]
pub enum ConwayPlutusPurpose {
    ConwaySpending(AsItem<TransactionInput>),
    ConwayMinting(AsItem<DisplayPolicyId>),
    ConwayCertifying(AsItem<ConwayTxCert>),
    ConwayRewarding(AsItem<DisplayRewardAccount>),
    ConwayVoting(AsItem<Voter>),
    ConwayProposing(AsItem<ProposalProcedure>),
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
    Item(AsItem<DisplayRewardAccount>),
}

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct Array<T>(#[n(0)] pub Vec<T>);

// https://github.com/IntersectMBO/cardano-ledger/blob/aed1dc28b98c25ea73bc692e7e6c6d3a22381ff5/eras/allegra/impl/src/Cardano/Ledger/Allegra/Scripts.hs#L109
#[derive(Debug)]
pub struct ValidityInterval {
    pub invalid_before: Option<SlotNo>,
    pub invalid_hereafter: Option<SlotNo>,
}

// https://github.com/IntersectMBO/cardano-ledger/blob/aed1dc28b98c25ea73bc692e7e6c6d3a22381ff5/libs/cardano-ledger-core/src/Cardano/Ledger/UTxO.hs#L83
#[derive(Debug)]
pub struct Utxo(pub OHashMap<TransactionInput, BabbageTxOut>);

// https://github.com/IntersectMBO/cardano-ledger/blob/master/eras/conway/impl/src/Cardano/Ledger/Conway/TxOut.hs
// https://github.com/IntersectMBO/cardano-ledger/blob/0d20d716fc15dc0b7648c448cbd735bebb7521b8/eras/babbage/impl/src/Cardano/Ledger/Babbage/TxOut.hs#L130
#[derive(Debug)]
pub struct BabbageTxOut {
    pub address: DisplayAddress,
    pub value: Option<Value>,
    pub datum: Option<DatumEnum>,
    pub script: Option<EraScript>,
}

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
pub struct Mismatch<T>(pub T, pub T)
where
    T: HaskellDisplay;

/// Represents an optional value that can either be present (`Just`) or absent (`Nothing`).
///
/// This enum is used to handle cases where a value might or might not be available, similar to `Option` in Rust.
/// It provides a way to explicitly represent the absence of a value.
///
/// In CBOR, this value comes in an array, so Rust's Option doesn't handle it.
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

#[derive(Debug, Decode, Clone)]
#[cbor(transparent)]
pub struct DisplayRewardAccount(#[n(0)] pub Bytes);

impl From<&Bytes> for DisplayRewardAccount {
    fn from(bytes: &Bytes) -> Self {
        DisplayRewardAccount(bytes.to_owned())
    }
}

impl From<&u64> for DisplayCoin {
    fn from(item: &u64) -> Self {
        DisplayCoin(item.to_owned())
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

#[derive(Debug, Clone)]
pub struct PlutusDataBytes(pub BoundedBytes);

#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct MaryValue(#[n(0)] pub DisplayCoin);

// https://github.com/IntersectMBO/cardano-ledger/blob/3dd7401424e8d50cc9f19feef1589f1ce0d83ed6/libs/cardano-data/src/Data/OSet/Strict.hs#L67
#[derive(Debug, Decode)]
#[cbor(transparent)]
pub struct DisplayOSet<T>(#[n(0)] pub Set<T>);

/**
 * Instead of this function, we can use Address type directly from pallas and decorate it with HaskellDisplay implementations
 */
pub fn get_network_and_credentials(bytes: &[u8]) -> (NetworkId, StakeCredential) {
    let network = if bytes[0] & 0b00000001 != 0 {
        // Is Mainnet Address
        NetworkId::Mainnet
    } else {
        NetworkId::Testnet
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
