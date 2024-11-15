{-# OPTIONS_GHC -fno-warn-orphans #-}

module Generators where

import qualified Cardano.Api.Eon.ShelleyBasedEra as CAPI
import qualified Cardano.Api.Eras.Core as CAPI
import qualified Cardano.Api.InMode as CAPI
import qualified Cardano.Api.Modes as CAPI
import Cardano.Api.Orphans ()
import Cardano.Binary (FromCBOR (..), ToCBOR (..))
import Cardano.Ledger.Binary (DecCBOR, EncCBOR)
import Codec.Serialise (Serialise)
import qualified Codec.Serialise
import Data.Aeson (ToJSON)
import GHC.Generics (Generic)
import Generic.Random (GenericArbitraryU (..))
import qualified Ouroboros.Consensus.Byron.Ledger as Consensus
import qualified Ouroboros.Consensus.HardFork.Combinator.AcrossEras as O
import qualified Ouroboros.Consensus.Ledger.SupportsMempool as Consensus
import qualified Ouroboros.Consensus.Shelley.Ledger as Consensus
import Test.Consensus.Cardano.Generators ()
import Test.QuickCheck (Arbitrary)
import qualified Test.QuickCheck as QC

genTxValidationErrorInCardanoMode :: QC.Gen CAPI.TxValidationErrorInCardanoMode
genTxValidationErrorInCardanoMode = QC.arbitrary

-- FIXME: where is it originally???
-- FIXME: maybe here? https://github.com/IntersectMBO/ouroboros-consensus/blob/358305b09f8fa1a85f076b20a51b4af03e827071/ouroboros-consensus-cardano/src/byron/Ouroboros/Consensus/Byron/Node/Serialisation.hs#L175-L178
-- instance ToCBOR CAPI.TxValidationErrorInCardanoMode where
--   toCBOR
-- deriving instance EncCBOR CAPI.TxValidationErrorInCardanoMode

genApplyTxError'Byron :: QC.Gen (Consensus.ApplyTxErr Consensus.ByronBlock)
genApplyTxError'Byron = QC.arbitrary

genApplyTxError'Shelley ::
  QC.Gen
    ( Consensus.ApplyTxErr
        ( Consensus.ShelleyBlock
            (CAPI.ConsensusProtocol CAPI.ShelleyEra)
            (CAPI.ShelleyLedgerEra CAPI.ShelleyEra)
        )
    )
genApplyTxError'Shelley = QC.arbitrary

genApplyTxError'Allegra ::
  QC.Gen
    ( Consensus.ApplyTxErr
        ( Consensus.ShelleyBlock
            (CAPI.ConsensusProtocol CAPI.AllegraEra)
            (CAPI.ShelleyLedgerEra CAPI.AllegraEra)
        )
    )
genApplyTxError'Allegra = QC.arbitrary

genApplyTxError'Mary ::
  QC.Gen
    ( Consensus.ApplyTxErr
        ( Consensus.ShelleyBlock
            (CAPI.ConsensusProtocol CAPI.MaryEra)
            (CAPI.ShelleyLedgerEra CAPI.MaryEra)
        )
    )
genApplyTxError'Mary = QC.arbitrary

genApplyTxError'Alonzo ::
  QC.Gen
    ( Consensus.ApplyTxErr
        ( Consensus.ShelleyBlock
            (CAPI.ConsensusProtocol CAPI.AlonzoEra)
            (CAPI.ShelleyLedgerEra CAPI.AlonzoEra)
        )
    )
genApplyTxError'Alonzo = QC.arbitrary

genApplyTxError'Babbage ::
  QC.Gen
    ( Consensus.ApplyTxErr
        ( Consensus.ShelleyBlock
            (CAPI.ConsensusProtocol CAPI.BabbageEra)
            (CAPI.ShelleyLedgerEra CAPI.BabbageEra)
        )
    )
genApplyTxError'Babbage = QC.arbitrary

genApplyTxError'Conway ::
  QC.Gen
    ( Consensus.ApplyTxErr
        ( Consensus.ShelleyBlock
            (CAPI.ConsensusProtocol CAPI.ConwayEra)
            (CAPI.ShelleyLedgerEra CAPI.ConwayEra)
        )
    )
genApplyTxError'Conway = QC.arbitrary

instance Arbitrary CAPI.TxValidationErrorInCardanoMode where
  arbitrary =
    QC.frequency
      [ ( 5,
          CAPI.TxValidationErrorInCardanoMode . CAPI.ByronTxValidationError
            <$> genApplyTxError'Byron
        ),
        ( 5,
          CAPI.TxValidationErrorInCardanoMode . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraShelley
            <$> genApplyTxError'Shelley
        ),
        ( 5,
          CAPI.TxValidationErrorInCardanoMode . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraAllegra
            <$> genApplyTxError'Allegra
        ),
        ( 5,
          CAPI.TxValidationErrorInCardanoMode . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraMary
            <$> genApplyTxError'Mary
        ),
        ( 5,
          CAPI.TxValidationErrorInCardanoMode . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraAlonzo
            <$> genApplyTxError'Alonzo
        ),
        ( 5,
          CAPI.TxValidationErrorInCardanoMode . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraBabbage
            <$> genApplyTxError'Babbage
        ),
        ( 15,
          CAPI.TxValidationErrorInCardanoMode . CAPI.ShelleyTxValidationError CAPI.ShelleyBasedEraConway
            <$> genApplyTxError'Conway
        ),
        (5, CAPI.TxValidationEraMismatch <$> QC.arbitrary)
      ]

instance Arbitrary O.EraMismatch where
  arbitrary = do
    (a, b) <- QC.oneof [pure ("Byron", "Shelley"), pure ("Shelley", "Byron")]
    pure (O.EraMismatch a b)

data ExampleADT
  = SAOne Integer
  | SATwo String
  | SAThree Double
  deriving (Eq, Show, Generic, ToJSON, Serialise, EncCBOR, DecCBOR)
  deriving (Arbitrary) via (GenericArbitraryU ExampleADT)

instance FromCBOR ExampleADT where
  fromCBOR = Codec.Serialise.decode

instance ToCBOR ExampleADT where
  toCBOR = Codec.Serialise.encode
