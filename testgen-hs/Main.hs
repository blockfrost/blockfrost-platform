module Main where

import CLI (GenSize (..), NumCases (..), Seed (..))
import qualified CLI
import Codec.CBOR.Write as C
import Data.Aeson (FromJSON, ToJSON)
import qualified Data.Aeson as J
import qualified Data.Aeson.Encode.Pretty as J
import qualified Data.ByteString.Base16 as B16
import qualified Data.ByteString.Lazy as BL
import qualified Data.ByteString.Lazy.Char8 as B
import Data.Proxy (Proxy (..))
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as T
import Data.Time.Clock.POSIX (getPOSIXTime)
import GHC.Generics (Generic)
import qualified Generators as G
import Test.QuickCheck (Arbitrary)
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
    json :: J.Value,
    haskellRepr :: Text
  }
  deriving (Generic, Show, FromJSON, ToJSON)

runCommand :: CLI.Options -> IO ()
runCommand (CLI.Options maybeSeed genSize numCases command) = do
  seed <- case maybeSeed of
    Just s -> return s
    Nothing -> Seed . round . (* 1000.0) <$> getPOSIXTime

  ( case command of
      CLI.ApplyTxErr'Byron -> writeRandom @G.ApplyTxErr'Byron Proxy
      CLI.ApplyTxErr'Shelley -> writeRandom @G.ApplyTxErr'Shelley Proxy
      CLI.ApplyTxErr'Allegra -> writeRandom @G.ApplyTxErr'Allegra Proxy
      CLI.ApplyTxErr'Mary -> writeRandom @G.ApplyTxErr'Mary Proxy
      CLI.ApplyTxErr'Alonzo -> writeRandom @G.ApplyTxErr'Alonzo Proxy
      CLI.ApplyTxErr'Babbage -> writeRandom @G.ApplyTxErr'Babbage Proxy
      CLI.ApplyTxErr'Conway -> writeRandom @G.ApplyTxErr'Conway Proxy
      CLI.TxValidationErrorInCardanoMode -> writeRandom @Double Proxy -- FIXME: generate the correct type
      CLI.DataText -> writeRandom @Text Proxy
      CLI.GHCInteger -> writeRandom @Integer Proxy
      CLI.ExampleADT -> writeRandom @G.ExampleADT Proxy
    )
    seed
    genSize
    numCases

writeRandom :: forall a. (Arbitrary a, Show a, G.OurCBOR a) => Proxy a -> Seed -> GenSize -> NumCases -> IO ()
writeRandom _ (Seed seed) (GenSize generatorSize) (NumCases numCases) = do
  let qcGen = QC.mkQCGen seed
      values :: [a] = QC.unGen (QC.vectorOf numCases QC.arbitrary) qcGen generatorSize
      typeTag = case values of
        a : _ -> T.pack . G.unwrappedType $ a
        _ -> "unknown"
      testCases :: [TestCase a] = mkTestCase <$> values
      output = Output {typeTag, seed, testCases}
  B.putStrLn $ J.encodePretty' (J.defConfig {J.confIndent = J.Spaces 2}) output

mkTestCase :: forall a. (Show a, G.OurCBOR a) => a -> TestCase a
mkTestCase a =
  TestCase
    { cbor =
        T.decodeUtf8Lenient
          . B16.encode
          . BL.toStrict
          . C.toLazyByteString
          . G.ourToCBOR
          $ a,
      haskellRepr = T.pack . show $ a,
      json = G.ourToJSON a
    }
