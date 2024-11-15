module CLI (Options (..), Seed (..), GenSize (..), NumCases (..), Command (..), parse) where

import Options.Applicative as O

newtype Seed = Seed Int deriving (Show)

newtype GenSize = GenSize Int deriving (Show)

newtype NumCases = NumCases Int deriving (Show)

data Options = Options (Maybe Seed) GenSize NumCases Command deriving (Show)

data Command
  = GHCInteger
  | DataText
  | ExampleADT
  | TxValidationErrorInCardanoMode
  | ApplyTxError'Byron
  | ApplyTxError'Shelley
  | ApplyTxError'Allegra
  | ApplyTxError'Mary
  | ApplyTxError'Alonzo
  | ApplyTxError'Babbage
  | ApplyTxError'Conway
  deriving (Show)

parse :: IO Options
parse = execParser opts

-------------------------------------------------------------------------------------

opts :: ParserInfo Options
opts =
  info
    (optionsParser <**> helper)
    ( fullDesc
        <> progDesc "Test case generator for cross-checking CBOR (de)serializers"
    )

optionsParser :: Parser Options
optionsParser =
  Options
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
    <*> commandParser

positive :: ReadM Int
positive = do
  n <- auto
  if n > 0
    then return n
    else readerError "NUM must be positive"

commandParser :: Parser Command
commandParser =
  subparser
    ( mempty
        <> mkCommand ApplyTxError'Byron
        <> mkCommand ApplyTxError'Shelley
        <> mkCommand ApplyTxError'Allegra
        <> mkCommand ApplyTxError'Mary
        <> mkCommand ApplyTxError'Alonzo
        <> mkCommand ApplyTxError'Babbage
        <> mkCommand ApplyTxError'Conway
        <> mkCommand TxValidationErrorInCardanoMode
        <> mkCommand GHCInteger
        <> mkCommand DataText
        <> mkCommand ExampleADT
    )

mkCommand :: Command -> Mod CommandFields Command
mkCommand cmd =
  command
    (replaceQuotes . show $ cmd)
    (info (pure cmd) (progDesc ("Generate CBOR of " ++ show cmd)))
  where
    replaceQuotes = ((\c -> if c == '\'' then '_' else c) <$>)
