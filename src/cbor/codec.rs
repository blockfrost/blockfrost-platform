use pallas_codec::minicbor::{decode, Decode, Decoder};
use pallas_primitives::{Nullable, ScriptHash};

use crate::cbor::haskell_types::{
    ApplyConwayTxPredError, ApplyTxError, ConwayUtxoPredFailure, ConwayUtxoWPredFailure, PlutusPurpose, ShelleyBasedEra, StrictMaybe, TxValidationError, Utxo
};

use super::haskell_types::{
    ConwayCertPredFailure, ConwayCertsPredFailure, ConwayGovCertPredFailure, ConwayGovPredFailure, Credential, CustomSet258, Network, RewardAccountFielded
};

impl<'b> Decode<'b, ()> for TxValidationError {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let era = d.decode()?;
        let error = d.decode()?;
        Ok(TxValidationError::ShelleyTxValidationError { error, era })
    }
}

impl<'b> Decode<'b, ()> for ApplyTxError {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let errors = d.array_iter::<ApplyConwayTxPredError>()?.collect();

        match errors {
            Ok(errors) => Ok(ApplyTxError(errors)),
            Err(error) => Err(error),
        }
    }
}

impl<'b> Decode<'b, ()> for ApplyConwayTxPredError {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;

        let error = d.u16()?;

        use ApplyConwayTxPredError::*;

        match error {
            1 => Ok(ConwayUtxowFailure(d.decode()?)),
            2 => Ok(ConwayCertsFailure(d.decode()?)),
            3 => Ok(ConwayGovFailure(d.decode()?)),
            4 => Ok(ConwayWdrlNotDelegatedToDRep(d.decode()?)),
            5 => Ok(ConwayTreasuryValueMismatch(d.decode()?, d.decode()?)),
            6 => Ok(ConwayTxRefScriptsSizeTooBig(d.decode()?, d.decode()?)),
            7 => Ok(ConwayMempoolFailure(d.decode()?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ApplyTxPredError: {}",
                error
            ))),
        }
    }
}

impl<'b> Decode<'b, ()> for Network {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let error = d.u16()?;

        match error {
            0 => Ok(Network::Testnet),
            1 => Ok(Network::Mainnet),
            _ => Err(decode::Error::message(format!(
                "unknown network while decoding Network: {}",
                error
            ))),
        }
    }
}

impl<'b> Decode<'b, ()> for ConwayUtxoWPredFailure {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayUtxoWPredFailure::*;

        match error {
            0 => Ok(UtxoFailure(d.decode()?)),
            1 => Ok(InvalidWitnessesUTXOW(d.decode()?)),
            2 => Ok(MissingVKeyWitnessesUTXOW(d.decode()?)),
            3 => Ok(MissingScriptWitnessesUTXOW(d.decode()?)),
            4 => Ok(ScriptWitnessNotValidatingUTXOW(d.decode()?)),
            5 => Ok(MissingTxBodyMetadataHash(d.decode()?)),
            6 => Ok(MissingTxMetadata(d.decode()?)),
            7 => Ok(ConflictingMetadataHash(d.decode()?, d.decode()?)),
            8 => Ok(InvalidMetadata()),
            9 => Ok(ExtraneousScriptWitnessesUTXOW(d.decode()?)),
            10 => Ok(MissingRedeemers(d.decode()?)),
            11 => Ok(MissingRequiredDatums(d.decode()?, d.decode()?)),
            12 => Ok(NotAllowedSupplementalDatums(d.decode()?, d.decode()?)),
            13 => Ok(PPViewHashesDontMatch(d.decode()?, d.decode()?)),
            14 => Ok(UnspendableUTxONoDatumHash(d.decode()?)),
            15 => Ok(ExtraRedeemers(d.decode()?)),
            16 => {
                let t = d.tag(); // we are ignoring the unknown tag 258 here
                Ok(MalformedScriptWitnesses(d.decode()?))
            }
            17 => Ok(MalformedReferenceScripts(d.decode()?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayUtxoWPredFailure: {}",
                error
            ))),
        }
    }
}

impl<'b> Decode<'b, ()> for ConwayUtxoPredFailure {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayUtxoPredFailure::*;

        match error {
            0 => Ok(UtxosFailure(d.decode()?)),
            1 => Ok(BadInputsUTxO(d.decode()?)),
            2 => Ok(OutsideValidityIntervalUTxO(d.decode()?, d.decode()?)),
            3 => Ok(MaxTxSizeUTxO(d.decode()?)),
            4 => Ok(InputSetEmptyUTxO()),
            5 => Ok(FeeTooSmallUTxO(d.decode()?, d.decode()?)),
            6 => Ok(ValueNotConservedUTxO(d.decode()?, d.decode()?)),
            7 => Ok(WrongNetwork(d.decode()?, d.decode()?)),
            8 => Ok(WrongNetworkWithdrawal(d.decode()?, d.decode()?)),
            9 => Ok(OutputTooSmallUTxO(d.decode()?)),
            10 => Ok(OutputBootAddrAttrsTooBig(d.decode()?)),
            11 => Ok(OutputTooBigUTxO(d.decode()?)),
            12 => Ok(InsufficientCollateral(d.decode()?, d.decode()?)),
            13 => Ok(ScriptsNotPaidUTxO(d.decode()?)),
            14 => Ok(ExUnitsTooBigUTxO(d.decode()?)),
            15 => Ok(CollateralContainsNonADA(d.decode()?)),
            16 => Ok(WrongNetworkInTxBody()),
            17 => Ok(OutsideForecast(d.decode()?)),
            18 => Ok(TooManyCollateralInputs(d.decode()?)),
            19 => Ok(NoCollateralInputs()),
            20 => Ok(IncorrectTotalCollateralField(d.decode()?, d.decode()?)),
            21 => Ok(BabbageOutputTooSmallUTxO(d.decode()?)),
            22 => Ok(BabbageNonDisjointRefInputs(d.decode()?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayUtxoPredFailure: {}",
                error
            ))),
        }
    }
}
impl<'b> Decode<'b, ()> for ConwayGovPredFailure {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let era = d.u16()?;

        use ConwayGovPredFailure::*;

        match era {
            0 => Ok(GovActionsDoNotExist(d.decode()?)),
            1 => Ok(MalformedProposal(d.decode()?)),
            2 => Ok(ProposalProcedureNetworkIdMismatch(d.decode()?, d.decode()?)),
            3 => Ok(TreasuryWithdrawalsNetworkIdMismatch(d.decode()?, d.decode()?)),
            4 => Ok(ProposalDepositIncorrect(d.decode()?)),
            5 => Ok(DisallowedVoters(d.decode()?)),
            6 => Ok(ConflictingCommitteeUpdate(d.decode()?)),

            7 => Ok(ExpirationEpochTooSmall(d.decode()?)),

            8 => Ok(InvalidPrevGovActionId(d.decode()?)),

            9 => Ok(VotingOnExpiredGovAction(d.decode()?)),

            10 => Ok(ProposalCantFollow(d.decode()?)),
            11 => {
               // let a = d.probe();

               /* let arr = d.array().unwrap().unwrap_or(0);
                let b1 = d.bytes()?;
                d.array()?;
                let b2 = d.bytes()?;
                 */

                use StrictMaybe::*;

                let maybe_hash1: Nullable<ScriptHash> =  match d.array()? {
                    Some(len) if len > 0 => (d.decode()?),
                    _ => Nullable::Null
                };

                let maybe_hash2: Nullable<ScriptHash> =  match d.array()? {
                    Some(len) if len > 0 => (d.decode()?),
                    _ => Nullable::Null
                };
                  
                Ok(InvalidPolicyHash(maybe_hash1, maybe_hash2))
                
            },
            12 => Ok(DisallowedProposalDuringBootstrap(d.decode()?)),
            13 => Ok(DisallowedVotesDuringBootstrap(d.decode()?)),
            14 => Ok(VotersDoNotExist(d.decode()?)),
            15 => Ok(ZeroTreasuryWithdrawals(d.decode()?)),
            16 => Ok(ProposalReturnAccountDoesNotExist(d.decode()?)),
            17 => Ok(TreasuryWithdrawalReturnAccountsDoNotExist(d.decode()?)),

            _ => Err(decode::Error::message(format!(
                "unknown era while decoding ConwayGovPredFailure: {}",
                era
            ))),
        }
    }
}

impl<'b> Decode<'b, ()> for ConwayCertsPredFailure {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayCertsPredFailure::*;

        match error {
            0 => Ok(WithdrawalsNotInRewardsCERTS(d.decode()?)),
            1 => Ok(CertFailure(d.decode()?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayCertsPredFailure: {}",
                error
            ))),
        }
    }
}

impl<'b> Decode<'b, ()> for ConwayCertPredFailure {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayCertPredFailure::*;

        match error {
            1 => Ok(DelegFailure(d.decode()?)),
            2 => Ok(PoolFailure(d.decode()?)),
            3 => Ok(GovCertFailure(d.decode()?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayCertPredFailure: {}",
                error
            ))),
        }
    }
}

impl<'b> Decode<'b, ()> for ConwayGovCertPredFailure {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayGovCertPredFailure::*;

        match error {
            0 => Ok(ConwayDRepAlreadyRegistered(d.decode()?)),
            1 => Ok(ConwayDRepNotRegistered(d.decode()?)),
            2 => Ok(ConwayDRepIncorrectDeposit(d.decode()?, d.decode()?)),
            3 => Ok(ConwayCommitteeHasPreviouslyResigned(d.decode()?)),
            4 => Ok(ConwayDRepIncorrectRefund(d.decode()?, d.decode()?)),
            5 => Ok(ConwayCommitteeIsUnknown(d.decode()?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayGovCertPredFailure: {}",
                error
            ))),
        }
    }
}

impl<'b> Decode<'b, ()> for Credential {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let tag = d.u16()?;

        use Credential::*;

        match tag {
            0 => Ok(KeyHashObj(d.decode()?)),
            1 => Ok(ScriptHashObj(d.decode()?)),
            _ => Err(decode::Error::message(format!(
                "unknown tag while decoding Credential: {}",
                tag
            ))),
        }
    }
}

impl<'b> Decode<'b, ()> for RewardAccountFielded {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let b = d.bytes()?;
        Ok(RewardAccountFielded::new(hex::encode(b)))
    }
}

impl<'b> Decode<'b, ()> for ShelleyBasedEra {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
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

// not tested yet
impl<'b> Decode<'b, ()> for PlutusPurpose {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        // d.array()?;
        let purpose = d.u16()?;

        use PlutusPurpose::*;

        match purpose {
            0 => Ok(Spending),
            1 => Ok(Minting),
            2 => Ok(Certifying),
            3 => Ok(Rewarding),
            _ => Err(decode::Error::message(format!(
                "unknown purpose while decoding PlutusPurpose: {}",
                purpose
            ))),
        }
    }
}

// not tested yet
impl<'b> Decode<'b, ()> for Utxo {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        // d.array()?;
        let tx_vec = d.decode()?;
        Ok(Utxo(tx_vec))
    }
}

impl<'b, T> Decode<'b, ()> for CustomSet258<T> where T: Decode<'b, ()> {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let tag = d.tag()?; // we are ignoring the unknown tag 258 here
        Ok(CustomSet258 (d.decode()?))
    }
}