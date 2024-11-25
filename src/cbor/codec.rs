use pallas_codec::minicbor::{decode, Decode, Decoder};

use crate::cbor::haskell_types::{
    ApplyConwayTxPredError, ApplyTxErr, ConwayUtxoPredFailure, ConwayUtxoWPredFailure,
    PlutusPurpose, ShelleyBasedEra, TxValidationError, Utxo,
};

impl<'b> Decode<'b, ()> for TxValidationError {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let era = d.decode()?;
        let error = d.decode()?;
        Ok(TxValidationError::ShelleyTxValidationError { error, era })
    }
}

impl<'b> Decode<'b, ()> for ApplyTxErr {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let errors = d.array_iter::<ApplyConwayTxPredError>()?.collect();

        match errors {
            Ok(errors) => Ok(ApplyTxErr(errors)),
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
            2 => Ok(CertsFailure(d.decode()?)),
            3 => Ok(GovFailure(d.decode()?)),
            // TODO Fix this
            // 4 => Ok(WdrlNotDelegatedToDRep(d.decode()?)),
            5 => Ok(ConwayTreasuryValueMismatch(d.decode()?, d.decode()?)),
            6 => Ok(TxRefScriptsSizeTooBig(d.decode()?)),
            7 => Ok(MempoolFailure(d.decode()?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ApplyTxPredError: {}",
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
            13 => Ok(PPViewHashesDontMatch(d.decode()?)),
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
