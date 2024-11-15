module Main where

import CLI (GenSize (..), NumCases (..), Seed (..))
import qualified CLI
import Cardano.Ledger.Binary (EncCBOR)
import qualified Cardano.Ledger.Binary as CLB
import Codec.CBOR.Write as C
import Data.Aeson (FromJSON, ToJSON)
import qualified Data.Aeson.Encode.Pretty as J
import qualified Data.ByteString.Base16 as B16
import qualified Data.ByteString.Lazy as BL
import qualified Data.ByteString.Lazy.Char8 as B
import Data.Proxy (Proxy (..))
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as T
import Data.Time.Clock.POSIX (getPOSIXTime)
import Data.Typeable (typeRep)
import GHC.Generics (Generic)
import qualified Generators as G
import qualified Test.QuickCheck as QC
import qualified Test.QuickCheck.Gen as QC (unGen)
import Test.QuickCheck.Instances.Text ()
import qualified Test.QuickCheck.Random as QC (mkQCGen)

main :: IO ()
main = CLI.parse >>= runCommand

data Output a = Output
  { typeTag :: Text,
    seed :: Int,
    testCases :: [TestCase a]
  }
  deriving (Generic, Show, FromJSON, ToJSON)

data TestCase a = TestCase
  { cbor :: Text,
    json :: a,
    haskellRepr :: Text
    -- FIXME: , submitAPIRepr :: Data.Aeson.Value
  }
  deriving (Generic, Show, FromJSON, ToJSON)

runCommand :: CLI.Options -> IO ()
runCommand (CLI.Options maybeSeed genSize numCases command) = do
  seed <- case maybeSeed of
    Just s -> return s
    Nothing -> (Seed . round) `fmap` getPOSIXTime

  ( case command of
      CLI.ApplyTxError'Byron -> writeRandom G.genApplyTxError'Byron
      CLI.ApplyTxError'Shelley -> writeRandom G.genApplyTxError'Shelley
      CLI.ApplyTxError'Allegra -> writeRandom G.genApplyTxError'Allegra
      CLI.ApplyTxError'Mary -> writeRandom G.genApplyTxError'Mary
      CLI.ApplyTxError'Alonzo -> writeRandom G.genApplyTxError'Alonzo
      CLI.ApplyTxError'Babbage -> writeRandom G.genApplyTxError'Babbage
      CLI.ApplyTxError'Conway -> writeRandom G.genApplyTxError'Conway
      CLI.TxValidationErrorInCardanoMode -> writeRandom (QC.arbitrary @Double) -- FIXME: G.genTxValidationErrorInCardanoMode
      CLI.DataText -> writeRandom (QC.arbitrary @Text)
      CLI.GHCInteger -> writeRandom (QC.arbitrary @Integer)
      CLI.ExampleADT -> writeRandom (QC.arbitrary @G.ExampleADT)
    )
    seed
    genSize
    numCases

writeRandom :: forall a. (Show a, EncCBOR a, ToJSON a) => QC.Gen a -> Seed -> GenSize -> NumCases -> IO ()
writeRandom genA (Seed seed) (GenSize generatorSize) (NumCases numCases) = do
  let qcGen = QC.mkQCGen seed
      values :: [a] = QC.unGen (QC.vectorOf numCases genA) qcGen generatorSize
      testCases :: [TestCase a] = mkTestCase <$> values
      output = Output {typeTag = T.pack . show . typeRep $ Proxy @a, seed, testCases}
  B.putStrLn $ J.encodePretty' (J.defConfig {J.confIndent = J.Spaces 2}) output

mkTestCase :: forall a. (Show a, EncCBOR a) => a -> TestCase a
mkTestCase a =
  let haskellRepr = T.pack $ show a
      -- XXX: we’re using the latest protocol version
      protocolVersion :: CLB.Version = maxBound
      cbor =
        ( T.decodeUtf8Lenient
            . B16.encode
            . BL.toStrict
            . C.toLazyByteString
            . CLB.toPlainEncoding protocolVersion
            . CLB.encCBOR
        )
          a
   in TestCase {cbor, haskellRepr, json = a}
