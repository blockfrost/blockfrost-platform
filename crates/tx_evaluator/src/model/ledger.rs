pub enum TransactionScriptFailure {
    RedeemerPointsToUnknownScriptHash,
    MissingScript,
    MissingDatum,
    ValidationFailure,
    UnknownTxIn,
    InvalidTxIn,
    IncompatibleBudget,
    NoCostModelInLedgerState,
    ContextError,
}
