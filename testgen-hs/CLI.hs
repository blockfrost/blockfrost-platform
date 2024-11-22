module CLI
  ( Command (..),
    GenerateOptions (..),
    Seed (..),
    GenSize (..),
    NumCases (..),
    TypeCommand (..),
    parse,
  )
where

import Data.ByteString (ByteString)
import qualified Data.ByteString.Base16 as Base16
import qualified Data.ByteString.Char8 as BS8
import Options.Applicative as O

data Command = Generate GenerateOptions | Deserialize ByteString deriving (Show)

data GenerateOptions = GenerateOptions (Maybe Seed) GenSize NumCases TypeCommand deriving (Show)

newtype Seed = Seed Int deriving (Show)

newtype GenSize = GenSize Int deriving (Show)

newtype NumCases = NumCases Int deriving (Show)

data TypeCommand
  = GHCInteger
  | DataText
  | ExampleADT
  | ApplyTxErr'Byron
  | ApplyTxErr'Shelley
  | ApplyTxErr'Allegra
  | ApplyTxErr'Mary
  | ApplyTxErr'Alonzo
  | ApplyTxErr'Babbage
  | ApplyTxErr'Conway
  deriving (Show)

parse :: IO Command
parse = execParser opts

-------------------------------------------------------------------------------------

opts :: ParserInfo Command
opts =
  info
    (commandParser <**> helper)
    ( fullDesc
        <> progDesc "Test case generator for cross-checking CBOR (de)serializers"
    )

commandParser :: Parser Command
commandParser =
  subparser
    ( mempty
        <> ( command
               "generate"
               ( info
                   ( Generate
                       <$> optionsParser
                         <**> helper
                   )
                   (progDesc "Generate random CBOR test cases")
               )
           )
        <> ( command
               "deserialize"
               ( info
                   ( Deserialize
                       <$> argument (eitherReader parseHex) (metavar "CBOR_HEX")
                         <**> helper
                   )
                   (progDesc "Deserialize CBOR of ‘HardForkApplyTxErr’ that you got from cardano-node")
               )
           )
    )

optionsParser :: Parser GenerateOptions
optionsParser =
  GenerateOptions
    <$> optional
      ( Seed
          <$> option
            auto
            ( long "seed"
                <> short 's'
                <> metavar "SEED"
                <> help "Random seed integer (UNIX timestamp by default)"
            )
      )
    <*> ( GenSize
            <$> option
              auto
              ( long "generator-size"
                  <> short 'g'
                  <> metavar "SIZE"
                  <> value 300
                  <> help "Set the relative ‘size’ of the test cases"
              )
        )
    <*> ( NumCases
            <$> option
              positive
              ( long "number"
                  <> short 'n'
                  <> metavar "NUM"
                  <> value 10
                  <> help "How many test cases to generate"
              )
        )
    <*> typeCommandParser

positive :: ReadM Int
positive = do
  n <- auto
  if n > 0
    then return n
    else readerError "NUM must be positive"

typeCommandParser :: Parser TypeCommand
typeCommandParser =
  subparser
    ( mempty
        <> mkTypeCommand ApplyTxErr'Byron
        <> mkTypeCommand ApplyTxErr'Shelley
        <> mkTypeCommand ApplyTxErr'Allegra
        <> mkTypeCommand ApplyTxErr'Mary
        <> mkTypeCommand ApplyTxErr'Alonzo
        <> mkTypeCommand ApplyTxErr'Babbage
        <> mkTypeCommand ApplyTxErr'Conway
        <> mkTypeCommand GHCInteger
        <> mkTypeCommand DataText
        <> mkTypeCommand ExampleADT
    )

mkTypeCommand :: TypeCommand -> Mod CommandFields TypeCommand
mkTypeCommand cmd =
  command
    (replaceQuotes . show $ cmd)
    (info (pure cmd) (progDesc ("Generate CBOR of " ++ show cmd)))
  where
    replaceQuotes = ((\c -> if c == '\'' then '_' else c) <$>)

-- | Parse a hex-encoded ByteString – e.g. CBOR
parseHex :: String -> Either String ByteString
parseHex hexInput =
  let bsInput = BS8.pack hexInput
   in Base16.decode bsInput
