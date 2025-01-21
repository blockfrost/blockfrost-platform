use pallas_addresses::Address;
use pallas_codec::minicbor::{self, decode, Decode, Decoder};
use pallas_crypto::hash::Hasher;

use crate::cbor::haskell_types::{
    ApplyConwayTxPredError, ApplyTxError, ConwayUtxoPredFailure, ConwayUtxoWPredFailure, DatumEnum,
    EpochNo, MaryValue, MultiAsset, PlutusPurpose, ShelleyBasedEra, StrictMaybe, TxValidationError,
    Utxo,
};

use super::{
    haskell_display::HaskellDisplay,
    haskell_types::{
        BabbageTxOut, ConwayCertPredFailure, ConwayCertsPredFailure, ConwayDelegPredFailure,
        ConwayGovCertPredFailure, ConwayGovPredFailure, Credential, CustomSet258, DisplayHash,
        EraScript, Mismatch, Network, RewardAccountFielded, ShelleyPoolPredFailure, SlotNo,
        Timelock, TimelockRaw, ValidityInterval,
    },
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

/*
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
*/

impl<'b> Decode<'b, ()> for ValidityInterval {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;

        let invalid_before: Option<SlotNo> = match d.array()? {
            Some(1) => Some(d.decode()?),
            _ => None,
        };

        let invalid_hereafter: Option<SlotNo> = match d.array()? {
            Some(1) => Some(d.decode()?),
            _ => None,
        };

        Ok(ValidityInterval {
            invalid_before,
            invalid_hereafter,
        })
    }
}
impl<'b> Decode<'b, ()> for ShelleyPoolPredFailure {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let start = d.position();
        let cbor = &d.input()[start..];
        let cbor_hex = hex::encode(cbor);
        println!("ShelleyPoolPredFailure CBOR: {}", cbor_hex);

        d.array()?;
        let tag = d.u16()?;

        use ShelleyPoolPredFailure::*;
        match tag {
            0 => Ok(StakePoolNotRegisteredOnKeyPOOL(d.decode()?)),
            1 => Ok(StakePoolRetirementWrongEpochPOOL(
                Mismatch(EpochNo(1), d.decode()?),
                d.decode()?,
            )),
            3 => Ok(StakePoolCostTooLowPOOL(d.decode()?)),
            4 => Ok(WrongNetworkPOOL(d.decode()?, d.decode()?)),
            5 => Ok(PoolMedataHashTooBig(d.decode()?, d.decode()?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ShelleyPoolPredFailure: {}",
                tag
            ))),
        }
    }
}

impl<'b, T> Decode<'b, ()> for Mismatch<T>
where
    T: Decode<'b, ()> + HaskellDisplay,
{
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let start = d.position();
        let cbor = &d.input()[start..];
        let cbor_hex = hex::encode(cbor);
        println!("Mismatch CBOR: {}", cbor_hex);
        match d.decode() {
            Ok(mis1) => match d.decode() {
                Ok(mis2) => Ok(Mismatch(mis1, mis2)),
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
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
            16 => Ok(MalformedScriptWitnesses(d.decode()?)),
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
            3 => Ok(TreasuryWithdrawalsNetworkIdMismatch(
                d.decode()?,
                d.decode()?,
            )),
            4 => Ok(ProposalDepositIncorrect(d.decode()?, d.decode()?)),
            5 => Ok(DisallowedVoters(d.decode()?)),
            6 => Ok(ConflictingCommitteeUpdate(d.decode()?)),

            7 => Ok(ExpirationEpochTooSmall(d.decode()?)),

            8 => Ok(InvalidPrevGovActionId(d.decode()?)),

            9 => Ok(VotingOnExpiredGovAction(d.decode()?)),

            10 => {
                d.array()?;
                let a = d.decode()?;
                Ok(ProposalCantFollow(a, d.decode()?, d.decode()?))
            }
            11 => Ok(InvalidPolicyHash(d.decode()?, d.decode()?)),
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
        let start = d.position();
        let cbor = &d.input()[start..];
        let cbor_hex = hex::encode(cbor);
        println!("ConwayCertsPredFailure CBOR: {}", cbor_hex);
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
        let start = d.position();
        let cbor = &d.input()[start..];
        let cbor_hex = hex::encode(cbor);
        println!("ConwayCertPredFailure CBOR: {}", cbor_hex);
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

impl<'b> Decode<'b, ()> for ConwayDelegPredFailure {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayDelegPredFailure::*;

        match error {
            1 => Ok(IncorrectDepositDELEG(d.decode()?)),
            2 => Ok(StakeKeyRegisteredDELEG(d.decode()?)),
            3 => Ok(StakeKeyNotRegisteredDELEG(d.decode()?)),
            4 => Ok(StakeKeyHasNonZeroRewardAccountBalanceDELEG(d.decode()?)),
            5 => Ok(DelegateeDRepNotRegisteredDELEG(d.decode()?)),
            6 => Ok(DelegateeStakePoolNotRegisteredDELEG(d.decode()?)),
            _ => Err(decode::Error::message(format!(
                "unknown error code while decoding ConwayDelegPredFailure: {}",
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

impl<'b, T> Decode<'b, ()> for StrictMaybe<T>
where
    T: Decode<'b, ()> + HaskellDisplay,
{
    fn decode(d: &mut Decoder<'b>, ctx: &mut ()) -> Result<Self, decode::Error> {
        let arr = d.array()?;

        match arr {
            Some(len) if len > 0 => Ok(StrictMaybe::Just(d.decode_with(ctx)?)),
            _ => Ok(StrictMaybe::Nothing),
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
        d.array()?;
        let purpose = d.u16()?;

        use PlutusPurpose::*;

        match purpose {
            0 => Ok(Spending(d.decode()?)),
            1 => Ok(Minting(d.decode()?)),
            2 => Ok(Certifying(d.decode()?)),
            3 => Ok(Rewarding(d.decode()?)),
            4 => Ok(Voting(d.decode()?)),
            5 => Ok(Proposing(d.decode()?)),
            _ => Err(decode::Error::message(format!(
                "unknown purpose while decoding PlutusPurpose: {}",
                purpose
            ))),
        }
    }
}

// https://github.com/IntersectMBO/cardano-ledger/blob/ea1d4362226d29ce7e42f4ba83ffeecedd9f0565/eras/babbage/impl/src/Cardano/Ledger/Babbage/TxOut.hs#L484
impl<'b> Decode<'b, ()> for BabbageTxOut {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let len = d.map()?;
        match len {
            Some(2) => Ok(BabbageTxOut::NotImplemented),
            Some(3) => Ok(BabbageTxOut::NotImplemented),
            Some(4) => {
                // key 0
                d.u8()?;
                let addr: Address = Address::from_bytes(d.bytes()?).unwrap();

                // key 1
                d.u8()?;
                d.array()?;
                let value: MaryValue = d.decode()?;

                let multi_asset: MultiAsset = d.decode()?;

                // key 2
                // datum enum
                d.u8()?;
                let datum: DatumEnum = d.decode()?;

                // key 3
                // inner cbor
                d.u8()?;

                d.tag()?;
                let inner_cbor_bytes = d.bytes()?;
                // let inner_cbor = hex::encode(bytes);
                let era_script = minicbor::decode::<EraScript>(inner_cbor_bytes)?;

                println!(
                    "BabbageTxOut::MaryTxOut: addr: {} ",
                    multi_asset.to_haskell_str()
                );

                // println!("BabbageTxOut::MaryTxOut: addr: {}, value: {}, multiAsset: {}, datum: {:?}, era_script: {:?}", addr, value, multiAsset.to_haskell_str(), datum, era_script);

                // Ok(BabbageTxOut::NotImplemented)

                Ok(BabbageTxOut::TxOutCompactRefScript(
                    addr,
                    (value, multi_asset),
                    datum,
                    StrictMaybe::Just(era_script),
                ))
            }
            None => {
                // indef map
                Ok(BabbageTxOut::NotImplemented)
            }
            _ => Err(decode::Error::message(format!(
                "unexpected number of fields while decoding BabbageTxOut: {}",
                len.unwrap_or(0)
            ))),
        }
    }
}

// not tested yet
impl<'b> Decode<'b, ()> for EraScript {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let tag = d.u16()?;

        match tag {
            0 => Ok(EraScript::Native(d.decode()?)),
            1 => Ok(EraScript::PlutusV1(d.decode()?)),
            2 => Ok(EraScript::PlutusV2(d.decode()?)),
            3 => Ok(EraScript::PlutusV3(d.decode()?)),
            _ => Err(decode::Error::message(format!(
                "unknown index while decoding EraScript: {}",
                tag
            ))),
        }
    }
}

// not tested yet
impl<'b> Decode<'b, ()> for TimelockRaw {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let tag = d.u16()?;

        use TimelockRaw::*;
        match tag {
            0 => Ok(Signature(d.decode()?)),
            1 => Ok(AllOf(d.decode()?)),
            2 => Ok(AnyOf(d.decode()?)),
            3 => Ok(MOfN(d.decode()?, d.decode()?)),
            4 => Ok(TimeStart(d.decode()?)),
            5 => Ok(TimeExpire(d.decode()?)),
            _ => Err(decode::Error::message(format!(
                "unknown index while decoding Timelock: {}",
                tag
            ))),
        }
    }
}

// not tested yet
impl<'b> Decode<'b, ()> for Timelock {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let first = d.position();

        let raw: TimelockRaw = d.decode()?;
        let last = d.position();
        let input = d.input();
        let raw_bytes = &input[first..last];

        let mut hasher = Hasher::<256>::new();
        hasher.input(raw_bytes);
        let memo = DisplayHash(hasher.finalize());
        Ok(Timelock { raw, memo })
    }
}

// not tested yet
impl<'b> Decode<'b, ()> for DatumEnum {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let tag = d.u16()?;

        match tag {
            0 => Ok(DatumEnum::DatumHash(d.decode()?)),
            1 => Ok(DatumEnum::Datum(d.decode()?)),
            _ => Ok(DatumEnum::NoDatum),
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

impl<'b, T> Decode<'b, ()> for CustomSet258<T>
where
    T: Decode<'b, ()>,
{
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let _tag = d.tag()?; // we are ignoring the tag 258 here
        Ok(CustomSet258(d.decode()?))
    }
}
