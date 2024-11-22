{-# OPTIONS_GHC -fno-warn-orphans #-}

module Generators where

import qualified Cardano.Api.Eon.ShelleyBasedEra as CAPI
import qualified Cardano.Api.Eras.Core as CAPI
import qualified Cardano.Api.InMode as CAPI
import qualified Cardano.Api.Modes as CAPI
import Cardano.Api.Orphans ()
import qualified Cardano.Chain.Slotting as CCS
import qualified Cardano.TxSubmit.Types as CTT
import Codec.CBOR.Encoding as C
import Codec.Serialise (Serialise)
import qualified Codec.Serialise
import Data.Aeson (ToJSON)
import qualified Data.Aeson as J
import Data.Text (Text)
import Data.Typeable (Typeable, typeOf)
import GHC.Generics (Generic)
import Generic.Random (GenericArbitraryU (..))
import qualified Ouroboros.Consensus.Byron.Ledger as OCBL
import qualified Ouroboros.Consensus.Cardano.Block as OCCB
import Ouroboros.Consensus.Cardano.Node as OCCN
import qualified Ouroboros.Consensus.HardFork.Combinator.AcrossEras as O
import qualified Ouroboros.Consensus.HardFork.Combinator.Mempool as OCHCM
import Ouroboros.Consensus.HardFork.Combinator.Serialisation.SerialiseNodeToClient ()
import qualified Ouroboros.Consensus.Ledger.SupportsMempool as OCLSM
import qualified Ouroboros.Consensus.Node.ProtocolInfo as OCNPI
import qualified Ouroboros.Consensus.Node.Serialisation as OCNS
import qualified Ouroboros.Consensus.Shelley.Ledger as OCSL
import Test.Consensus.Cardano.Generators ()
import Test.QuickCheck (Arbitrary)
import qualified Test.QuickCheck as QC

genTxValidationErrorInCardanoMode :: QC.Gen CAPI.TxValidationErrorInCardanoMode
genTxValidationErrorInCardanoMode = QC.arbitrary

-- | We define our own type class, to be able to include multiple complex
--  encoders for our `newtype` wrappers under a single interface.
class (Typeable a) => OurCBOR a where
  unwrappedType :: a -> String
  ourToCBOR :: a -> C.Encoding
  ourToJSON :: a -> J.Value

------- Byron ------------------------------------------------------------------

newtype ApplyTxErr'Byron = ApplyTxErr'Byron (OCLSM.ApplyTxErr OCBL.ByronBlock)
  deriving newtype (Eq, Show, ToJSON, Arbitrary)

instance OurCBOR ApplyTxErr'Byron where
  unwrappedType (ApplyTxErr'Byron a) = show . typeOf $ a
  ourToCBOR (ApplyTxErr'Byron a) = hfcEnvelope . OCCB.ApplyTxErrByron $ a
  ourToJSON (ApplyTxErr'Byron a) =
    submitApiEnvelope . CAPI.ByronTxValidationError $ a

------- Shelley ----------------------------------------------------------------

newtype ApplyTxErr'Shelley
  = ApplyTxErr'Shelley
      ( OCLSM.ApplyTxErr
          ( OCSL.ShelleyBlock
              (CAPI.ConsensusProtocol CAPI.ShelleyEra)
              (CAPI.ShelleyLedgerEra CAPI.ShelleyEra)
          )
      )
  deriving newtype (Eq, Show, ToJSON, Arbitrary)

instance OurCBOR ApplyTxErr'Shelley where
  unwrappedType (ApplyTxErr'Shelley a) = show . typeOf $ a
  ourToCBOR (ApplyTxErr'Shelley a) = hfcEnvelope . OCCB.ApplyTxErrShelley $ a
  ourToJSON (ApplyTxErr'Shelley a) =
    submitApiEnvelope . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraShelley $ a

------- Allegra ----------------------------------------------------------------

newtype ApplyTxErr'Allegra
  = ApplyTxErr'Allegra
      ( OCLSM.ApplyTxErr
          ( OCSL.ShelleyBlock
              (CAPI.ConsensusProtocol CAPI.AllegraEra)
              (CAPI.ShelleyLedgerEra CAPI.AllegraEra)
          )
      )
  deriving newtype (Eq, Show, ToJSON, Arbitrary)

instance OurCBOR ApplyTxErr'Allegra where
  unwrappedType (ApplyTxErr'Allegra a) = show . typeOf $ a
  ourToCBOR (ApplyTxErr'Allegra a) = hfcEnvelope . OCCB.ApplyTxErrAllegra $ a
  ourToJSON (ApplyTxErr'Allegra a) =
    submitApiEnvelope . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraAllegra $ a

------- Mary -------------------------------------------------------------------

newtype ApplyTxErr'Mary
  = ApplyTxErr'Mary
      ( OCLSM.ApplyTxErr
          ( OCSL.ShelleyBlock
              (CAPI.ConsensusProtocol CAPI.MaryEra)
              (CAPI.ShelleyLedgerEra CAPI.MaryEra)
          )
      )
  deriving newtype (Eq, Show, ToJSON, Arbitrary)

instance OurCBOR ApplyTxErr'Mary where
  unwrappedType (ApplyTxErr'Mary a) = show . typeOf $ a
  ourToCBOR (ApplyTxErr'Mary a) = hfcEnvelope . OCCB.ApplyTxErrMary $ a
  ourToJSON (ApplyTxErr'Mary a) =
    submitApiEnvelope . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraMary $ a

------- Alonzo -----------------------------------------------------------------

newtype ApplyTxErr'Alonzo
  = ApplyTxErr'Alonzo
      ( OCLSM.ApplyTxErr
          ( OCSL.ShelleyBlock
              (CAPI.ConsensusProtocol CAPI.AlonzoEra)
              (CAPI.ShelleyLedgerEra CAPI.AlonzoEra)
          )
      )
  deriving newtype (Eq, Show, ToJSON, Arbitrary)

instance OurCBOR ApplyTxErr'Alonzo where
  unwrappedType (ApplyTxErr'Alonzo a) = show . typeOf $ a
  ourToCBOR (ApplyTxErr'Alonzo a) = hfcEnvelope . OCCB.ApplyTxErrAlonzo $ a
  ourToJSON (ApplyTxErr'Alonzo a) =
    submitApiEnvelope . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraAlonzo $ a

------- Babbage ----------------------------------------------------------------

newtype ApplyTxErr'Babbage
  = ApplyTxErr'Babbage
      ( OCLSM.ApplyTxErr
          ( OCSL.ShelleyBlock
              (CAPI.ConsensusProtocol CAPI.BabbageEra)
              (CAPI.ShelleyLedgerEra CAPI.BabbageEra)
          )
      )
  deriving newtype (Eq, Show, ToJSON, Arbitrary)

instance OurCBOR ApplyTxErr'Babbage where
  unwrappedType (ApplyTxErr'Babbage a) = show . typeOf $ a
  ourToCBOR (ApplyTxErr'Babbage a) = hfcEnvelope . OCCB.ApplyTxErrBabbage $ a
  ourToJSON (ApplyTxErr'Babbage a) =
    submitApiEnvelope . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraBabbage $ a

------- Conway -----------------------------------------------------------------

newtype ApplyTxErr'Conway
  = ApplyTxErr'Conway
      ( OCLSM.ApplyTxErr
          ( OCSL.ShelleyBlock
              (CAPI.ConsensusProtocol CAPI.ConwayEra)
              (CAPI.ShelleyLedgerEra CAPI.ConwayEra)
          )
      )
  deriving newtype (Eq, Show, ToJSON, Arbitrary)

instance OurCBOR ApplyTxErr'Conway where
  unwrappedType (ApplyTxErr'Conway a) = show . typeOf $ a
  ourToCBOR (ApplyTxErr'Conway a) = hfcEnvelope . OCCB.ApplyTxErrConway $ a
  ourToJSON (ApplyTxErr'Conway a) =
    submitApiEnvelope . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraConway $ a

------- HardForkApplyTxErr -----------------------------------------------------

hfcEnvelope :: OCHCM.HardForkApplyTxErr (OCCB.CardanoEras OCCB.StandardCrypto) -> C.Encoding
hfcEnvelope wrapped =
  OCNS.encodeNodeToClient
    @(OCCB.HardForkBlock (OCCB.CardanoEras OCCB.StandardCrypto))
    @(OCHCM.HardForkApplyTxErr (OCCB.CardanoEras OCCB.StandardCrypto))
    codecConfig
    OCCN.CardanoNodeToClientVersion12
    wrapped
  where
    byronEpochSlots = CCS.EpochSlots 21600 -- probably safe to hardcode in Conway…?
    codecConfig = OCNPI.pClientInfoCodecConfig (OCCN.protocolClientInfoCardano byronEpochSlots)

submitApiEnvelope :: CAPI.TxValidationError era -> J.Value
submitApiEnvelope =
  J.toJSON
    . CTT.TxSubmitFail
    . CTT.TxCmdTxSubmitValidationError
    . CAPI.TxValidationErrorInCardanoMode

------- TxValidationErrorInCardanoMode (all at once) ---------------------------

instance Arbitrary CAPI.TxValidationErrorInCardanoMode where
  arbitrary =
    QC.frequency
      [ ( 5,
          CAPI.TxValidationErrorInCardanoMode
            . CAPI.ByronTxValidationError
            . (\(ApplyTxErr'Byron a) -> a)
            <$> QC.arbitrary
        ),
        ( 5,
          CAPI.TxValidationErrorInCardanoMode
            . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraShelley
            . (\(ApplyTxErr'Shelley a) -> a)
            <$> QC.arbitrary
        ),
        ( 5,
          CAPI.TxValidationErrorInCardanoMode
            . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraAllegra
            . (\(ApplyTxErr'Allegra a) -> a)
            <$> QC.arbitrary
        ),
        ( 5,
          CAPI.TxValidationErrorInCardanoMode
            . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraMary
            . (\(ApplyTxErr'Mary a) -> a)
            <$> QC.arbitrary
        ),
        ( 5,
          CAPI.TxValidationErrorInCardanoMode
            . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraAlonzo
            . (\(ApplyTxErr'Alonzo a) -> a)
            <$> QC.arbitrary
        ),
        ( 5,
          CAPI.TxValidationErrorInCardanoMode
            . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraBabbage
            . (\(ApplyTxErr'Babbage a) -> a)
            <$> QC.arbitrary
        ),
        ( 15,
          CAPI.TxValidationErrorInCardanoMode
            . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraConway
            . (\(ApplyTxErr'Conway a) -> a)
            <$> QC.arbitrary
        ),
        (5, CAPI.TxValidationEraMismatch <$> QC.arbitrary)
      ]

instance Arbitrary O.EraMismatch where
  arbitrary = do
    (a, b) <- QC.oneof [pure ("Byron", "Shelley"), pure ("Shelley", "Byron")]
    pure (O.EraMismatch a b)

------- ExampleADT -------------------------------------------------------------

data ExampleADT
  = SAOne Integer
  | SATwo String
  | SAThree Double
  deriving (Eq, Show, Generic, ToJSON, Serialise)
  deriving (Arbitrary) via (GenericArbitraryU ExampleADT)

instance OurCBOR ExampleADT where
  unwrappedType = show . typeOf
  ourToCBOR = Codec.Serialise.encode
  ourToJSON = J.toJSON

------- basic types ------------------------------------------------------------

instance OurCBOR Text where
  unwrappedType = show . typeOf
  ourToCBOR = Codec.Serialise.encode
  ourToJSON = J.toJSON

instance OurCBOR Integer where
  unwrappedType = show . typeOf
  ourToCBOR = Codec.Serialise.encode
  ourToJSON = J.toJSON

instance OurCBOR Double where
  unwrappedType = show . typeOf
  ourToCBOR = Codec.Serialise.encode
  ourToJSON = J.toJSON
