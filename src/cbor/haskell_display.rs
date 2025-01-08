use std::{collections::HashMap, fmt::{self, format}};

use chrono::format;
use pallas_crypto::key;
use pallas_primitives::{
    conway::{Anchor, Constitution, GovAction, GovActionId, ProposalProcedure, VKeyWitness, Voter}, AddrKeyhash, Coin, DatumHash, Hash, Nullable, ProtocolVersion, RewardAccount, ScriptHash, StakeCredential
};
use tracing::warn;

use super::haskell_types::{
    Array, AsIx, BabbageTxOut, ConwayCertPredFailure, ConwayDelegPredFailure, ConwayGovCertPredFailure, Credential, CustomSet258, DisplayScriptHash, EpochNo, KeyHash, PlutusPurpose, RewardAccountFielded, SafeHash, StrictMaybe, VKey
};


impl fmt::Display for ConwayGovCertPredFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ConwayGovCertPredFailure::*;

        match self {
            ConwayDRepAlreadyRegistered(cred) => {
                write!(f, "ConwayDRepAlreadyRegistered ({})", cred)
            }
            ConwayDRepNotRegistered(cred) => write!(f, "ConwayDRepNotRegistered ({})", cred),
            ConwayDRepIncorrectDeposit(expected, actual) => {
                write!(f, "ConwayDRepIncorrectDeposit ({}, {})", expected, actual)
            }
            ConwayCommitteeHasPreviouslyResigned(cred) => {
                write!(f, "ConwayCommitteeHasPreviouslyResigned ({})", cred)
            }
            ConwayDRepIncorrectRefund(expected, actual) => {
                write!(f, "ConwayDRepIncorrectRefund ({}, {})", expected, actual)
            }
            ConwayCommitteeIsUnknown(cred) => write!(f, "ConwayCommitteeIsUnknown ({})", cred),
        }
    }
}


impl fmt::Display for ConwayCertPredFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ConwayCertPredFailure::*;

        match self {
            DelegFailure(e) => write!(f, "DelegFailure ({})", e.to_haskell_str()),
            PoolFailure(e) => write!(f, "PoolFailure  ({})", e),
            GovCertFailure(e) => write!(f, "GovCertFailure ({})", e),
        }
    }
}
pub trait HaskellDisplay {
    fn to_haskell_str(&self) -> String;
}

pub trait HaskellDisplayParentesis {
    fn to_haskell_str_p(&self) -> String;
}

impl HaskellDisplay for ConwayDelegPredFailure {
    fn to_haskell_str(&self) -> String {
        use ConwayDelegPredFailure::*;

        match self {
            IncorrectDepositDELEG(coin)  => format!("IncorrectDepositDELEG {}", coin.to_haskell_str()),
            StakeKeyRegisteredDELEG(String) => "DelegFailure".to_string(),
            StakeKeyNotRegisteredDELEG(String)  => "DelegFailure".to_string(),
               StakeKeyHasNonZeroRewardAccountBalanceDELEG(String)  => "DelegFailure".to_string(),
              DelegateeDRepNotRegisteredDELEG(cred) => format!("DelegateeDRepNotRegisteredDELEG ({})", cred.to_haskell_str()),
               DelegateeStakePoolNotRegisteredDELEG (String)  => "DelegFailure".to_string(),
        
        }
    }
}

impl HaskellDisplay for Coin {
    fn to_haskell_str(&self) -> String {
        format!("Coin {}", self)
    }
}

impl HaskellDisplay for RewardAccount {
    fn to_haskell_str(&self) -> String {
        let hex = hex::encode(self.as_ref() as &[u8]);

        RewardAccountFielded::new(hex).to_string()
    }
}

impl HaskellDisplay for KeyHash {
    fn to_haskell_str(&self) -> String {
        format!("KeyHash {{unKeyHash = \"{}\"}}", self.0)
    }
}



impl HaskellDisplay for SafeHash {
    fn to_haskell_str(&self) -> String {
        self.0.as_safe_hash()
    }
}

impl HaskellDisplay for GovActionId {
    fn to_haskell_str(&self) -> String {
        format!(
            "GovActionId {{gaidTxId = {}, gaidGovActionIx = {}}}",
            self.transaction_id.as_tx_id(),
            display_governance_action_id_index(&self.action_index)
        )
    }
}

impl<'b, T> HaskellDisplay for Nullable<T>
where
    T: HaskellDisplay + std::clone::Clone,
{
    fn to_haskell_str(&self) -> String {
        match self {
            Nullable::Some(v) => format!("SJust ({})", v.to_haskell_str()),
            _ => "SNothing".to_string(),
        }
    }
}

impl<'b, T> HaskellDisplay for Option<T>
where
    T: HaskellDisplay ,
{
    fn to_haskell_str(&self) -> String {
        match self {
            Option::Some(v) => format!("SJust ({})", v.to_haskell_str()),
            _ => "SNothing".to_string(),
        }
    }
}

fn display_nullable<T>(x: &Nullable<T>) -> String
where
    T: HaskellDisplay + std::clone::Clone,
{
    match x {
        Nullable::Some(v) => format!("(SJust ({}))", v.to_haskell_str()),
        _ => "SNothing".to_string(),
    }
}

impl HaskellDisplay for GovAction {
    fn to_haskell_str(&self) -> String {
        use GovAction::*;

        match self {
            ParameterChange(a, b, c) => {
                format!(
                    "ParameterChange ({}, {:?}, {})",
                    a.to_haskell_str(),
                    b,
                    c.to_haskell_str(),
                )
            }
            HardForkInitiation(a, b) => {
                format!(
                    "HardForkInitiation {} ({})",
                    display_nullable(a),
                    b.as_protocol_version()
                )
            }
            TreasuryWithdrawals(a, b) => {
                format!("TreasuryWithdrawals ({:?}, {})", a, b.to_haskell_str(),)
            }
            NoConfidence(a) => {
                format!("NoConfidence ({})", display_nullable(a))
            }
            UpdateCommittee(a, b, c, d) => {
                format!(
                    "UpdateCommittee ({}, {:?}, {:?}, {:?})",
                    a.to_haskell_str(),
                    b,
                    c,
                    d
                )
            }
            NewConstitution(a, c) => {
                format!(
                    "NewConstitution ({}) ({})",
                    a.to_haskell_str(),
                    c.to_haskell_str()
                )
            }
            Information => "InfoAction".to_string(),
        }
    }
}

impl HaskellDisplay for Constitution {
    fn to_haskell_str(&self) -> String {
        format!(
            "Constitution {{constitutionAnchor = {}, constitutionScript = {}}}",
            self.anchor.to_haskell_str(),
            self.guardrail_script.to_haskell_str()
        )
    }
}
impl HaskellDisplay for Anchor {
    fn to_haskell_str(&self) -> String {
        format!(
            "Anchor {{anchorUrl = {}, anchorDataHash = {}}}",
            self.url.as_url(),
            self.content_hash.as_safe_hash()
        )
    }
}

impl HaskellDisplay for ProposalProcedure {
    fn to_haskell_str(&self) -> String {
        format!(
            "ProposalProcedure {{pProcDeposit = {}, pProcReturnAddr = {}, pProcGovAction = {}, pProcAnchor = {}}}",
            self.deposit.to_haskell_str(), self.reward_account.to_haskell_str(), self.gov_action.to_haskell_str(), self.anchor.to_haskell_str()
        )
    }
}

impl HaskellDisplay for ScriptHash {
    fn to_haskell_str(&self) -> String {
        format!("ScriptHash \"{}\"", self)
    }
}

impl HaskellDisplay for StakeCredential {
    fn to_haskell_str(&self) -> String {
        use StakeCredential::*;

        match self {
            AddrKeyhash(key_hash) => key_hash.as_key_hash_obj(),
            ScriptHash(scrpt) => scrpt.as_script_hash_obj(),
        }
    }
}

impl fmt::Display for Credential {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Credential::*;

        match self {
            ScriptHashObj(key_hash) => write!(f, "ScriptHashObj ({})", key_hash.as_script_hash()),
            KeyHashObj(script_hash) => write!(f, "KeyHashObj ({})", script_hash.as_key_hash()),
        }
    }
}


impl HaskellDisplay for BabbageTxOut {
    fn to_haskell_str(&self) -> String {
        format!("BabbageTxOut NotImplemented {:?}", &self)
    }
}

impl<K, V> HaskellDisplay for HashMap<K, V>
where
    K: HaskellDisplay,
    V: HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
      
        let result =  self.iter().map(|item|        format!("({},{})", item.0.to_haskell_str(), item.1.to_haskell_str())
    ).collect::<Vec<_>>().join(", ");

        format!("fromList [{}]", result)
    }
}



impl HaskellDisplay for EpochNo {
    fn to_haskell_str(&self) -> String {
        format!("EpochNo {}", &self.0)
    }
}

impl HaskellDisplay for i8 {
    fn to_haskell_str(&self) -> String {
        if *self >= 0 {
            format!("{}", self)
        } else {
            format!("({})", self)
        }
    }
}

impl<T> HaskellDisplay for Vec<T>
where
    T: HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
        let mut iter = self.iter();
        if let Some(first) = iter.next() {
            let mut result = first.to_haskell_str();
            result.push_str(" :| [");

            if (iter.len() > 0) {
                for item in iter {
                    result.push_str(&format!("{} ,", item.to_haskell_str()));
                }
                result.truncate(result.len() - 2);
            }
            result.push_str("]");

            result
        } else {
            "fromList []".to_string()
        }
    }
}

impl HaskellDisplay for Credential {
    fn to_haskell_str(&self) -> String {
        use Credential::*;

        match self {
            ScriptHashObj(key_hash) => key_hash.as_script_hash_obj(),
            KeyHashObj(key_hash) => key_hash.as_key_hash_obj(),
        }
    }
}
impl<T, H> HaskellDisplay for (T, H)
where
    T: HaskellDisplay,
    H: HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
        format!("({},{})", self.0.to_haskell_str(), self.1.to_haskell_str())
    }
}



impl HaskellDisplay for Voter {
    fn to_haskell_str(&self) -> String {
        use Voter::*;

        match self {
            ConstitutionalCommitteeKey(addr) => {
                format!("ConstitutionalCommitteeKey({})", addr.as_key_hash())
            }
            ConstitutionalCommitteeScript(scrpt) => {
                format!(
                    "ConstitutionalCommitteeScript ({})",
                    scrpt.as_script_hash_obj()
                )
            }
            DRepKey(addr) => {
                format!("DRepVoter ({})", addr.as_key_hash_obj())
            }
            DRepScript(scrpt) => {
                format!("DRepVoter ({})", scrpt.as_script_hash_obj())
            }
            StakePoolKey(addr) => {
                format!("StakePoolVoter ({})", addr.as_key_hash())
            }
        }
    }
}

impl<T> HaskellDisplay for CustomSet258<T>
where
    T: HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
        self.0.as_from_list()
    }
}

impl HaskellDisplay for DisplayScriptHash {
    fn to_haskell_str(&self) -> String {
        self.0.as_script_hash()
    }
}

impl HaskellDisplay for VKey {
    fn to_haskell_str(&self) -> String {
        self.0.as_vkey()
    }
}
trait AsTransactionId {
    fn as_tx_id(&self) -> String;
}

trait AsSafeHash {
    fn as_safe_hash(&self) -> String;
}

trait AsKeyHash {
    fn as_key_hash(&self) -> String;
}

trait AsVKey {
    fn as_vkey(&self) -> String;
}

trait AsScriptHashObject {
    fn as_script_hash_obj(&self) -> String;
}

trait AsFromList {
    fn as_from_list(&self) -> String;
}

trait AsKeyHashObject {
    fn as_key_hash_obj(&self) -> String;
}

trait AsScriptHash {
    fn as_script_hash(&self) -> String;
}

trait AsUrl {
    fn as_url(&self) -> String;
}

trait AsProtocolVersion {
    fn as_protocol_version(&self) -> String;
}

impl AsUrl for String {
    fn as_url(&self) -> String {
        format!("Url {{urlToText = \"{}\"}}", self)
    }
}
impl AsTransactionId for [u8] {
    fn as_tx_id(&self) -> String {
        format!("TxId {{unTxId = {}}}", self.as_safe_hash())
    }
}

impl AsSafeHash for [u8] {
    fn as_safe_hash(&self) -> String {
        let hex = hex::encode(self);
        format!("SafeHash \"{}\"", hex)
    }
}
impl AsSafeHash for Hash<28> {
    fn as_safe_hash(&self) -> String {
        let hex = hex::encode(self.as_ref());
        format!("SafeHash \"{}\"", hex)
    }
}

impl<T> AsSafeHash for Nullable<T>
where
    T: AsSafeHash + std::clone::Clone,
{
    fn as_safe_hash(&self) -> String {
        match self {
            Nullable::Some(v) => v.as_safe_hash(),
            _ => "SNothing".to_string(),
        }
    }
}

impl AsKeyHash for [u8] {
    fn as_key_hash(&self) -> String {
        let hex = hex::encode(self);
        format!("KeyHash {{unKeyHash = \"{}\"}}", hex)
    }
}

impl AsVKey for [u8] {
    fn as_vkey(&self) -> String {
        let hex = hex::encode(self);
        format!("VKey (VerKeyEd25519DSIGN \"{}\")", hex)
    }
}

impl AsScriptHashObject for [u8] {
    fn as_script_hash_obj(&self) -> String {
        format!("ScriptHashObj ({})", self.as_script_hash())
    }
}

impl AsKeyHashObject for [u8] {
    fn as_key_hash_obj(&self) -> String {
        format!("KeyHashObj ({})", self.as_key_hash())
    }
}

impl AsScriptHash for [u8] {
    fn as_script_hash(&self) -> String {
        let hex = hex::encode(self);
        format!("ScriptHash \"{}\"", hex)
    }
}

impl AsProtocolVersion for ProtocolVersion {
    fn as_protocol_version(&self) -> String {
        format!(
            "ProtVer {{pvMajor = Version {}, pvMinor = {}}}",
            self.0, self.1
        )
    }
}

impl<T: HaskellDisplay> AsFromList for Vec<T> {
    fn as_from_list(&self) -> String {
       // let mut result =String::new();

        let result =  self.iter().map(|item| item.to_haskell_str()).collect::<Vec<_>>().join(", ");

        
         format!("fromList [{}]", result)
        
    }
}

impl HaskellDisplay for AsIx {
    fn to_haskell_str(&self) -> String {
        format!("AsIx {{unAsIx = {}}}", self.0)
    }
}

impl HaskellDisplay for PlutusPurpose {
    fn to_haskell_str(&self) -> String {
        use PlutusPurpose::*;

        match self {
            Minting(policy_id) => format!("ConwayMinting ({})", policy_id.to_haskell_str()),
            Spending(txin) => format!("ConwaySpending ({})", txin.to_haskell_str()),
            Rewarding(reward_account) => format!("ConwayRewarding ({})", reward_account.to_haskell_str()),
            Certifying(cert_index) => format!("ConwayCertifying ({})", cert_index.to_haskell_str()),
            Voting(gov_id) => format!("ConwayVoting ({})", gov_id.to_haskell_str()),
            Proposing(proposal_id) => format!("ConwayProposing ({})", proposal_id.to_haskell_str()),
        }
    }
}
impl<T> HaskellDisplay for StrictMaybe<T>
where
    T: HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
        match self {
            StrictMaybe::Just(v) => format!("(SJust ({}))", v.to_haskell_str()),
            StrictMaybe::Nothing => "SNothing".to_string(),
        }
    }
}
impl<T> HaskellDisplayParentesis for StrictMaybe<T>
where
    T: HaskellDisplay,
{
    fn to_haskell_str_p(&self) -> String {
        match self {
            StrictMaybe::Just(v) => format!("(SJust ({}))", v.to_haskell_str()),
            StrictMaybe::Nothing => "SNothing".to_string(),
        }
    }
}

impl HaskellDisplay for VKeyWitness
 
{
    fn to_haskell_str(&self) -> String {
        format!(
         
            "VKeyWitness {{ vkey: {}, signature: {} }}",
            self.vkey,
            self.signature
        )
    }
}

impl <T> HaskellDisplay for Array<T>
where
    T: HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
        let value = self.0.iter().map(|item| item.to_haskell_str()).collect::<Vec<_>>().join(", ");
        format!("[{}]", value)
        

    }
}

impl HaskellDisplay for DatumHash {
    fn to_haskell_str(&self) -> String {
        format!("DatumHash \"{}\"", hex::encode(self.as_ref()))
    }
}

// HELPER FUNCTIONS

fn display_governance_action_id_index(index: &u32) -> String {
    format!("GovActionIx {{unGovActionIx = {}}}", index)
}
