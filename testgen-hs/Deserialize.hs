{-# OPTIONS_GHC -fno-warn-orphans #-}

module Deserialize (deserialize) where

import Cardano.Api.Orphans ()
import qualified Cardano.Chain.Slotting as CCS
import qualified Codec.CBOR.Decoding as C
import qualified Codec.CBOR.Read as C
import Data.ByteString (ByteString)
import qualified Data.ByteString.Base16 as B16
import qualified Data.ByteString.Lazy as BL
import qualified Ouroboros.Consensus.Cardano.Block as OCCB
import Ouroboros.Consensus.Cardano.Node as OCCN
import qualified Ouroboros.Consensus.HardFork.Combinator.Mempool as OCHCM
import Ouroboros.Consensus.HardFork.Combinator.Serialisation.SerialiseNodeToClient ()
import qualified Ouroboros.Consensus.Node.ProtocolInfo as OCNPI
import qualified Ouroboros.Consensus.Node.Serialisation as OCNS
import Test.Consensus.Cardano.Generators ()

deserialize :: ByteString -> Either String (OCHCM.HardForkApplyTxErr (OCCB.CardanoEras OCCB.StandardCrypto))
deserialize cbor =
  case C.deserialiseFromBytes hfcEnvelopeDecoder (BL.fromStrict cbor) of
    Left err -> Left (show err)
    Right ("", ok) -> Right ok
    Right (remainder, _) -> Left ("Deserialization successful, but the following bytes remained: " <> (show . B16.encode . BL.toStrict) remainder)

hfcEnvelopeDecoder :: forall s. C.Decoder s (OCHCM.HardForkApplyTxErr (OCCB.CardanoEras OCCB.StandardCrypto))
hfcEnvelopeDecoder =
  OCNS.decodeNodeToClient
    @(OCCB.HardForkBlock (OCCB.CardanoEras OCCB.StandardCrypto))
    @(OCHCM.HardForkApplyTxErr (OCCB.CardanoEras OCCB.StandardCrypto))
    codecConfig
    OCCN.CardanoNodeToClientVersion12
  where
    byronEpochSlots = CCS.EpochSlots 21600 -- probably safe to hardcode in Conway…?
    codecConfig = OCNPI.pClientInfoCodecConfig (OCCN.protocolClientInfoCardano byronEpochSlots)
