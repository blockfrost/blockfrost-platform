use pallas_codec::minicbor::{self, data::Type, decode, Decode, Decoder};
use pallas_primitives::{conway::Certificate, NetworkId};

use super::{
    haskell_display::HaskellDisplay,
    haskell_types::{
        ApplyConwayTxPredError, ApplyTxError, BabbageContextError, CollectError,
        ConwayCertPredFailure, ConwayCertsPredFailure, ConwayContextError, ConwayDelegPredFailure,
        ConwayGovCertPredFailure, ConwayGovPredFailure, ConwayPlutusPurpose, ConwayTxCert,
        ConwayUtxoPredFailure, ConwayUtxoWPredFailure, ConwayUtxosPredFailure, Credential, EpochNo,
        FailureDescription, Mismatch, OHashMap, PlutusPurpose, PurposeAs, ShelleyBasedEra,
        ShelleyPoolPredFailure, SlotNo, StrictMaybe, TagMismatchDescription, TxOutSource,
        TxValidationError, Utxo, ValidityInterval,
    },
};

macro_rules! decode_err {
    ($msg:expr) => {
        return Err(decode::Error::message($msg))
    };
}

impl<'b, C> Decode<'b, C> for TxValidationError {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let era = d.decode_with(ctx)?;
        let error = d.decode_with(ctx)?;
        Ok(TxValidationError::ShelleyTxValidationError { error, era })
    }
}

impl<'b, C> Decode<'b, C> for ApplyTxError {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        let errors = d
            .array_iter_with::<C, ApplyConwayTxPredError>(ctx)?
            .collect();

        match errors {
            Ok(errors) => Ok(ApplyTxError(errors)),
            Err(error) => Err(error),
        }
    }
}

impl<'b, C> Decode<'b, C> for ApplyConwayTxPredError {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;

        let error = d.u16()?;

        use ApplyConwayTxPredError::*;

        match error {
            1 => Ok(ConwayUtxowFailure(d.decode_with(ctx)?)),
            2 => Ok(ConwayCertsFailure(d.decode_with(ctx)?)),
            3 => Ok(ConwayGovFailure(d.decode_with(ctx)?)),
            4 => Ok(ConwayWdrlNotDelegatedToDRep(d.decode_with(ctx)?)),
            5 => Ok(ConwayTreasuryValueMismatch(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            6 => Ok(ConwayTxRefScriptsSizeTooBig(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            7 => Ok(ConwayMempoolFailure(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ApplyConwayTxPredError: {}",
                error
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for ValidityInterval {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;

        let invalid_before: Option<SlotNo> = match d.array()? {
            Some(1) => Some(d.decode_with(ctx)?),
            _ => None,
        };

        let invalid_hereafter: Option<SlotNo> = match d.array()? {
            Some(1) => Some(d.decode_with(ctx)?),
            _ => None,
        };

        Ok(ValidityInterval {
            invalid_before,
            invalid_hereafter,
        })
    }
}
impl<'b, C> Decode<'b, C> for ShelleyPoolPredFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let tag = d.u16()?;

        use ShelleyPoolPredFailure::*;
        match tag {
            0 => Ok(StakePoolNotRegisteredOnKeyPOOL(d.decode_with(ctx)?)),
            1 => {
                let gt_expected: EpochNo = d.decode_with(ctx)?;
                let lt_supplied: EpochNo = d.decode_with(ctx)?;
                let lt_expected: EpochNo = d.decode_with(ctx)?;

                Ok(StakePoolRetirementWrongEpochPOOL(
                    Mismatch(lt_supplied.clone(), gt_expected),
                    Mismatch(lt_supplied, lt_expected),
                ))
            }
            3 => Ok(StakePoolCostTooLowPOOL(d.decode_with(ctx)?)),
            4 => {
                let expected: NetworkId = d.decode_with(ctx)?;
                let supplied: NetworkId = d.decode_with(ctx)?;

                Ok(WrongNetworkPOOL(
                    Mismatch(supplied, expected),
                    d.decode_with(ctx)?,
                ))
            }
            5 => Ok(PoolMedataHashTooBig(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ShelleyPoolPredFailure: {}",
                tag
            ))),
        }
    }
}

impl<'b, T, C> Decode<'b, C> for Mismatch<T>
where
    T: Decode<'b, C> + HaskellDisplay,
{
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        match d.decode_with(ctx) {
            Ok(mis1) => match d.decode_with(ctx) {
                Ok(mis2) => Ok(Mismatch(mis1, mis2)),
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        }
    }
}

impl<'b, C> Decode<'b, C> for ConwayUtxosPredFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayUtxosPredFailure::*;

        match error {
            0 => Ok(ValidationTagMismatch(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            1 => Ok(CollectErrors(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayUtxosPredFailure: {}",
                error
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for ConwayUtxoWPredFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayUtxoWPredFailure::*;

        match error {
            0 => Ok(UtxoFailure(d.decode_with(ctx)?)),
            1 => Ok(InvalidWitnessesUTXOW(d.decode_with(ctx)?)),
            2 => Ok(MissingVKeyWitnessesUTXOW(d.decode_with(ctx)?)),
            3 => Ok(MissingScriptWitnessesUTXOW(d.decode_with(ctx)?)),
            4 => Ok(ScriptWitnessNotValidatingUTXOW(d.decode_with(ctx)?)),
            5 => Ok(MissingTxBodyMetadataHash(d.decode_with(ctx)?)),
            6 => Ok(MissingTxMetadata(d.decode_with(ctx)?)),
            7 => Ok(ConflictingMetadataHash(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            8 => Ok(InvalidMetadata()),
            9 => Ok(ExtraneousScriptWitnessesUTXOW(d.decode_with(ctx)?)),
            10 => Ok(MissingRedeemers(d.decode_with(ctx)?)),
            11 => Ok(MissingRequiredDatums(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            12 => Ok(NotAllowedSupplementalDatums(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            13 => Ok(PPViewHashesDontMatch(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            14 => Ok(UnspendableUTxONoDatumHash(d.decode_with(ctx)?)),
            15 => Ok(ExtraRedeemers(d.decode_with(ctx)?)),
            16 => Ok(MalformedScriptWitnesses(d.decode_with(ctx)?)),
            17 => Ok(MalformedReferenceScripts(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayUtxoWPredFailure: {}",
                error
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for ConwayUtxoPredFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayUtxoPredFailure::*;

        match error {
            0 => Ok(UtxosFailure(d.decode_with(ctx)?)),
            1 => Ok(BadInputsUTxO(d.decode_with(ctx)?)),
            2 => Ok(OutsideValidityIntervalUTxO(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            3 => Ok(MaxTxSizeUTxO(d.decode_with(ctx)?, d.decode_with(ctx)?)),
            4 => Ok(InputSetEmptyUTxO()),
            5 => Ok(FeeTooSmallUTxO(d.decode_with(ctx)?, d.decode_with(ctx)?)),
            6 => Ok(ValueNotConservedUTxO(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            7 => Ok(WrongNetwork(d.decode_with(ctx)?, d.decode_with(ctx)?)),
            8 => Ok(WrongNetworkWithdrawal(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            9 => Ok(OutputTooSmallUTxO(d.decode_with(ctx)?)),
            10 => Ok(OutputBootAddrAttrsTooBig(d.decode_with(ctx)?)),
            11 => Ok(OutputTooBigUTxO(d.decode_with(ctx)?)),
            12 => Ok(InsufficientCollateral(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            13 => Ok(ScriptsNotPaidUTxO(d.decode_with(ctx)?)),
            14 => Ok(ExUnitsTooBigUTxO(d.decode_with(ctx)?, d.decode_with(ctx)?)),
            15 => Ok(CollateralContainsNonADA(d.decode_with(ctx)?)),
            16 => Ok(WrongNetworkInTxBody(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            17 => Ok(OutsideForecast(d.decode_with(ctx)?)),
            18 => Ok(TooManyCollateralInputs(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            19 => Ok(NoCollateralInputs()),
            20 => Ok(IncorrectTotalCollateralField(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            21 => Ok(BabbageOutputTooSmallUTxO(d.decode_with(ctx)?)),
            22 => Ok(BabbageNonDisjointRefInputs(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayUtxoPredFailure: {}",
                error
            ))),
        }
    }
}
impl<'b, C> Decode<'b, C> for ConwayGovPredFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let era = d.u16()?;

        use ConwayGovPredFailure::*;

        match era {
            0 => Ok(GovActionsDoNotExist(d.decode_with(ctx)?)),
            1 => Ok(MalformedProposal(d.decode_with(ctx)?)),
            2 => Ok(ProposalProcedureNetworkIdMismatch(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            3 => Ok(TreasuryWithdrawalsNetworkIdMismatch(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            4 => Ok(ProposalDepositIncorrect(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            5 => Ok(DisallowedVoters(d.decode_with(ctx)?)),
            6 => Ok(ConflictingCommitteeUpdate(d.decode_with(ctx)?)),

            7 => Ok(ExpirationEpochTooSmall(d.decode_with(ctx)?)),

            8 => Ok(InvalidPrevGovActionId(d.decode_with(ctx)?)),

            9 => Ok(VotingOnExpiredGovAction(d.decode_with(ctx)?)),

            10 => Ok(ProposalCantFollow(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            11 => Ok(InvalidPolicyHash(d.decode_with(ctx)?, d.decode_with(ctx)?)),
            12 => Ok(DisallowedProposalDuringBootstrap(d.decode_with(ctx)?)),
            13 => Ok(DisallowedVotesDuringBootstrap(d.decode_with(ctx)?)),
            14 => Ok(VotersDoNotExist(d.decode_with(ctx)?)),
            15 => Ok(ZeroTreasuryWithdrawals(d.decode_with(ctx)?)),
            16 => Ok(ProposalReturnAccountDoesNotExist(d.decode_with(ctx)?)),
            17 => Ok(TreasuryWithdrawalReturnAccountsDoNotExist(
                d.decode_with(ctx)?,
            )),

            _ => Err(decode::Error::message(format!(
                "unknown era while decoding ConwayGovPredFailure: {}",
                era
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for ConwayCertsPredFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayCertsPredFailure::*;

        match error {
            0 => Ok(WithdrawalsNotInRewardsCERTS(d.decode_with(ctx)?)),
            1 => Ok(CertFailure(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayCertsPredFailure: {}",
                error
            ))),
        }
    }
}

impl<'b, C, K: pallas_codec::minicbor::Decode<'b, C>, V: pallas_codec::minicbor::Decode<'b, C>>
    Decode<'b, C> for OHashMap<K, V>
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let v: Result<Vec<(K, V)>, _> = d.map_iter_with::<C, K, V>(ctx)?.collect();

        Ok(OHashMap(v?))
    }
}

impl<'b, C> Decode<'b, C> for CollectError {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use CollectError::*;
        match error {
            0 => Ok(NoRedeemer(d.decode_with(ctx)?)),
            1 => Ok(NoWitness(d.decode_with(ctx)?)),
            2 => Ok(NoCostModel(d.decode_with(ctx)?)),
            3 => Ok(BadTranslation(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding CollectError: {}",
                error
            ))),
        }
    }
}
impl<'b, C> Decode<'b, C> for ConwayContextError {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayContextError::*;

        match error {
            8 => Ok(BabbageContextError(d.decode_with(ctx)?)),

            9 => Ok(CertificateNotSupported(d.decode_with(ctx)?)),

            10 => Ok(PlutusPurposeNotSupported(d.decode_with(ctx)?)),
            11 => Ok(CurrentTreasuryFieldNotSupported(d.decode_with(ctx)?)),
            12 => Ok(VotingProceduresFieldNotSupported(d.decode_with(ctx)?)),
            13 => Ok(ProposalProceduresFieldNotSupported(d.decode_with(ctx)?)),
            14 => Ok(TreasuryDonationFieldNotSupported(d.decode_with(ctx)?)),

            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding CollectError: {}",
                error
            ))),
        }
    }
}
impl<'b, C> Decode<'b, C> for BabbageContextError {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use BabbageContextError::*;

        match error {
            0 => Ok(ByronTxOutInContext(d.decode_with(ctx)?)),
            1 => Ok(AlonzoMissingInput(d.decode_with(ctx)?)),
            2 => Ok(RedeemerPointerPointsToNothing(d.decode_with(ctx)?)),
            4 => Ok(InlineDatumsNotSupported(d.decode_with(ctx)?)),
            5 => Ok(ReferenceScriptsNotSupported(d.decode_with(ctx)?)),
            6 => Ok(ReferenceInputsNotSupported(d.decode_with(ctx)?)),
            7 => Ok(AlonzoTimeTranslationPastHorizon(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding BabbageContextError: {}",
                error
            ))),
        }
    }
}
impl<'b, C> Decode<'b, C> for TxOutSource {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use TxOutSource::*;

        match error {
            0 => Ok(TxOutFromInput(d.decode_with(ctx)?)),
            1 => Ok(TxOutFromOutput(d.decode_with(ctx)?)),

            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding TxOutSource: {}",
                error
            ))),
        }
    }
}
impl<'b, C> Decode<'b, C> for ConwayPlutusPurpose {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayPlutusPurpose::*;

        match error {
            0 => Ok(ConwaySpending(d.decode_with(ctx)?)),
            1 => Ok(ConwayMinting(d.decode_with(ctx)?)),
            2 => Ok(ConwayCertifying(d.decode_with(ctx)?)),
            3 => Ok(ConwayRewarding(d.decode_with(ctx)?)),
            4 => Ok(ConwayVoting(d.decode_with(ctx)?)),
            5 => Ok(ConwayProposing(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayPlutusPurpose: {}",
                error
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for ConwayCertPredFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayCertPredFailure::*;

        match error {
            1 => Ok(DelegFailure(d.decode_with(ctx)?)),
            2 => Ok(PoolFailure(d.decode_with(ctx)?)),
            3 => Ok(GovCertFailure(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayCertPredFailure: {}",
                error
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for ConwayGovCertPredFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayGovCertPredFailure::*;

        match error {
            0 => Ok(ConwayDRepAlreadyRegistered(d.decode_with(ctx)?)),
            1 => Ok(ConwayDRepNotRegistered(d.decode_with(ctx)?)),
            2 => Ok(ConwayDRepIncorrectDeposit(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            3 => Ok(ConwayCommitteeHasPreviouslyResigned(d.decode_with(ctx)?)),
            4 => Ok(ConwayDRepIncorrectRefund(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            5 => Ok(ConwayCommitteeIsUnknown(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayGovCertPredFailure: {}",
                error
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for ConwayDelegPredFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayDelegPredFailure::*;

        match error {
            1 => Ok(IncorrectDepositDELEG(d.decode_with(ctx)?)),
            2 => Ok(StakeKeyRegisteredDELEG(d.decode_with(ctx)?)),
            3 => Ok(StakeKeyNotRegisteredDELEG(d.decode_with(ctx)?)),
            4 => Ok(StakeKeyHasNonZeroRewardAccountBalanceDELEG(
                d.decode_with(ctx)?,
            )),
            5 => Ok(DelegateeDRepNotRegisteredDELEG(d.decode_with(ctx)?)),
            6 => Ok(DelegateeStakePoolNotRegisteredDELEG(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error code while decoding ConwayDelegPredFailure: {}",
                error
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for ConwayTxCert {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        let pos = d.position();
        d.array()?;
        let variant = d.u16()?;

        d.set_position(pos);
        let cert: Certificate = d.decode_with(ctx)?;

        match variant {
            // shelley deleg certificates
            0..3 => Ok(ConwayTxCert::ConwayTxCertDeleg(cert)),
            // pool certificates
            3..5 => Ok(ConwayTxCert::ConwayTxCertPool(cert)),
            // conway deleg certificates
            5 => decode_err!("Genesis delegation certificates are no longer supported"),
            6 => decode_err!("MIR certificates are no longer supported"),
            7..14 => Ok(ConwayTxCert::ConwayTxCertDeleg(cert)),
            14..19 => Ok(ConwayTxCert::ConwayTxCertGov(cert)),
            _ => Err(decode::Error::message(format!(
                "unknown certificate variant while decoding ConwayTxCert: {}",
                variant
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for TagMismatchDescription {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use TagMismatchDescription::*;

        match error {
            0 => Ok(PassedUnexpectedly),
            1 => Ok(FailedUnexpectedly(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding TagMismatchDescription: {}",
                error
            ))),
        }
    }
}
impl<'b, C> Decode<'b, C> for FailureDescription {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use FailureDescription::*;

        match error {
            1 => Ok(PlutusFailure(d.decode_with(ctx)?, d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding FailureDescription: {}",
                error
            ))),
        }
    }
}

impl<'b, T, C> Decode<'b, C> for StrictMaybe<T>
where
    T: Decode<'b, C> + HaskellDisplay,
{
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        let arr = d.array()?;

        match arr {
            Some(len) if len > 0 => Ok(StrictMaybe::Just(d.decode_with(ctx)?)),
            _ => Ok(StrictMaybe::Nothing),
        }
    }
}
impl<'b, C> Decode<'b, C> for Credential {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let tag = d.u16()?;

        use Credential::*;

        match tag {
            0 => Ok(KeyHashObj(d.decode_with(ctx)?)),
            1 => Ok(ScriptHashObj(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown tag while decoding Credential: {}",
                tag
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for ShelleyBasedEra {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let era = d.u16()?;

        use ShelleyBasedEra::*;

        match era {
            1 => Ok(ShelleyBasedEraShelley),
            2 => Ok(ShelleyBasedEraAllegra),
            3 => Ok(ShelleyBasedEraMary),
            4 => Ok(ShelleyBasedEraAlonzo),
            5 => Ok(ShelleyBasedEraBabbage),
            6 => Ok(ShelleyBasedEraConway),
            _ => Err(decode::Error::message(format!(
                "unknown era while decoding ShelleyBasedEra: {}",
                era
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for PlutusPurpose {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let purpose = d.u16()?;

        use PlutusPurpose::*;

        match purpose {
            0 => Ok(Spending(d.decode_with(ctx)?)),
            1 => Ok(Minting(d.decode_with(ctx)?)),
            2 => Ok(Certifying(d.decode_with(ctx)?)),
            3 => Ok(Rewarding(d.decode_with(ctx)?)),
            4 => Ok(Voting(d.decode_with(ctx)?)),
            5 => Ok(Proposing(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown purpose while decoding PlutusPurpose: {}",
                purpose
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for PurposeAs {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        use PurposeAs::*;

        let tp = d.probe().datatype()?;

        match d.probe().datatype()? {
            Type::U8 => Ok(Ix(d.decode_with(ctx)?)),
            Type::U16 => Ok(Ix(d.decode_with(ctx)?)),
            Type::U32 => Ok(Ix(d.decode_with(ctx)?)),
            Type::U64 => Ok(Ix(d.decode_with(ctx)?)),
            Type::Bytes => Ok(Item(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown datatype while decoding PurposeAs: {:?}",
                tp
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for Utxo {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        let tx_vec = d.decode_with(ctx)?;
        Ok(Utxo(tx_vec))
    }
}
