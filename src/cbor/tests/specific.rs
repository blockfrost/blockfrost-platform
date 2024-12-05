/*

You can generate them from a set of JSON files with something like:

```shell
idx=1; for f in *.json ; do
  symb=$(jq -r '.testCases[0].json.contents.contents.contents.error' "$f" | grep -Eo '([A-Z][a-z]*)+' | head -n 4 | tr '\n' '_' | sed -r 's/(^_+|_+$)//g')
  cbor=$(jq -r '.testCases[0].cbor' "$f")
  numb=$(printf '%04d' "$idx")
  echo '#[tokio::test]'
  echo '#[allow(non_snake_case)]'
  echo 'async fn test_cbor_'"$numb"'_'"$symb"'() {'
  echo '    verify_one("'"$cbor"'").await'
  echo '}'
  echo
  idx=$((idx + 1))
done
```

*/

use super::verify_one;

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0001_ConwayTreasuryValueMismatch_Coin_Coin() {
    verify_one("8182068183051a000de7561a00080fd6").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0002_ConwayMempoolFailure_ConwayGovFailure_ExpirationEpochTooSmall_List() {
    verify_one("8182068282076082038207a0").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0003_ConwayUtxowFailure_MissingTxBodyMetadataHash_AuxiliaryDataHash_AuxiliaryDataHash(
) {
    verify_one(
        "818206818201820558200e13ba83be25492abf84e10545393932480e8ad43dacf8a3d93dff388cce84ed",
    )
    .await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0004_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash() {
    verify_one("81820681820481581c22782faa6bd0c54048b6176eb0cc2f4aa6c56818b3b9075e480e4cbf").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0005_ConwayTxRefScriptsSizeTooBig() {
    verify_one("8182068183060001").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0006_ConwayUtxowFailure_MalformedScriptWitnesses_List() {
    verify_one("8182068182018210d9010280").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0007_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_ConwayTreasuryValueMismatch() {
    verify_one("81820682820481581cab4f400015b95d3b7c45a285fe08da9e4cc110b06105788819890a7283051a0001abb81a0007fc34").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0008_ConwayCertsFailure_CertFailure_GovCertFailure_ConwayCommitteeHasPreviouslyResigned(
) {
    verify_one("8182068282028201820382038200581cde174ee9f903cd93028d16e1bd0df936ddf2a842f2aa414db0598b6782038302581de0c3a48544970283c379904bf33f5ab2b8e1f6fac902a14ddcd18d2bb900").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0009_ConwayTreasuryValueMismatch_Coin_Coin_ConwayTreasuryValueMismatch() {
    verify_one("8182068283051a0006144d1a0007f68283051a000ab04e1a0003c428").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0010_ConwayMempoolFailure() {
    verify_one("8182068182076162").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0011_ConwayCertsFailure_WithdrawalsNotInRewardsCERTS_List_RewardAccount() {
    verify_one("8182068182028200a1581de180c1af75f8e788b08272ee30e8d87bc776e4bfc47adb0da175bf26ac1a000212eb").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0012_ConwayGovFailure_DisallowedProposalDuringBootstrap_ProposalProcedure_ProcDeposit(
) {
    verify_one("818206828203820c841a00043894581de05c60cda4d195859022a5dc288a826c9d413349697e3009dd8163b3358301825820b3a3b00795156a4bd4338afe1d5d1ed55969c088dbc37c134b6346bff9b7210001820b0082783b68747470733a2f2f365859374137397562386e7a755a687a6f73316c4155546a685830416d7a6a715a744837795a4779475a5071694b542e636f6d582049571f726fd12c21b39edd426be658fbc95e5b14cf3b235338573ef9daa1f4c68203830b81581cd3e73bd6d14a851a663cd925ff72ebacad31da7a6cdbddd623087b9780").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0013_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash() {
    verify_one("81820681820481581c3f784466c9efbcbc998ee0121bca9c4dc03dc37b756ed522e4d46ea6").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0014_ConwayTreasuryValueMismatch_Coin_Coin_ConwayTxRefScriptsSizeTooBig() {
    verify_one("8182068283051a00059a381a000393c683060101").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0015_ConwayMempoolFailure() {
    verify_one("8182068182076160").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0016_ConwayTxRefScriptsSizeTooBig_ConwayTreasuryValueMismatch_Coin_Coin() {
    verify_one("818206828306012083051a000218a31a000594fd").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0017_ConwayTreasuryValueMismatch_Coin_Coin_ConwayWdrlNotDelegatedToDRep() {
    verify_one("8182068283051a000d755a1a000438a6820482581cf88d2b1e7a199cc2791ecd58b2dba509ebb3213f8a45d76fe6565acf581c9d7fbbc29cea56a0b2cdb7c98f7ebef884ad509b5fecfbdc5d9e0a78").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0018_ConwayTxRefScriptsSizeTooBig_ConwayGovFailure_VotersDoNotExist_DRepVoter() {
    verify_one("81820682830601008203820e828203581cb921639de9f45aa695050cbb0746979c85e8897e33831a34387222d48204581cd0d83eac6aea2ae34f39c627996457b2eed18ad3283a62b0dbec896a").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0019_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
    verify_one("81820682820482581cc665e067f9c5af2973d41f470db5e85c1f3495958d9ce9f458117a02581cd16ff9615b0f243f73576a54d7b0ee5fd3f1a827899c50191d3cb9b483060120").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0020_ConwayTreasuryValueMismatch_Coin_Coin_ConwayWdrlNotDelegatedToDRep() {
    verify_one("8182068283051a000aec461a000b26cd820481581c651af173086c113865d55b9abbad7c47aa9569b85f26873ed1b281bd").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0021_ConwayTreasuryValueMismatch_Coin_Coin() {
    verify_one("8182068183051a000b55c61a000921de").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0022_ConwayMempoolFailure_R_ConwayGovFailure_VotingOnExpiredGovAction() {
    verify_one("81820682820761528203820981828204581c5e5b6fe689a2a0b842304c912712328aa704b6c49d15e8b60e6d979882582094259fb315f35d28860159dd35231ce60ee3f99d905f9e6519731a505540bb4e01").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0023_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_ConwayUtxowFailure() {
    verify_one("81820682820481581cbb05dc8589898474b225a13314de415931c384c1013d90c9b10d0f9c8201830d8158208ccf10dc8526e35d4c21bbfa98507c1c9e58cd7d2483a6c502213c3d5fc2f40981582027e04b9c68972e6dc9e8cf34916c338a00a7665df06eb526f333b7af3e908315").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0024_ConwayGovFailure_ConflictingCommitteeUpdate_List_ConwayGovFailure() {
    verify_one("8182068282038206d901028082038303d901028001").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0025_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
    verify_one("81820681820482581cea98c5db378729e96764fca2d3ca9f2d1973f59e0abecd37833338c0581ceafa36c204c8c0fee37dcbf776c13f31dbe4fd8915cc334beb26e0e9").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0026_ConwayGovFailure_MalformedProposal_HardForkInitiation_SJust() {
    verify_one("818206828203820183018258203d417a35bce157152945acfe78da6938f3ade318bed839ad70d62cd827abf4470082070182076169").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0027_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
    verify_one("81820682820482581c16e839e30b01738d1115c3efcf1262fac1797d8bd0089de9353aa106581cb5813031a33030f4922a9232e016e25818eefb2e2ccc450cb11f89ec82076138").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0028_ConwayTreasuryValueMismatch_Coin_Coin() {
    verify_one("8182068183051a0003ec3a1a0003e0c0").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0029_ConwayTreasuryValueMismatch_Coin_Coin_ConwayWdrlNotDelegatedToDRep() {
    verify_one("8182068283051a000bbfe91a000c125a820482581ca9091c6a554fb48870c42fdb0e367476f4795fea34177de56591bbef581c4ad1ed28f5d0590603d941434b07083a8bcd13cec2d5be9a0b5c8529").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0030_ConwayTxRefScriptsSizeTooBig_ConwayGovFailure_DisallowedVotesDuringBootstrap_DRepVoter(
) {
    verify_one("81820682830601208203820d82828203581ca8d4974095d18fd37c4ba1da80a36851b8345ddaf233a7d75f927e2f825820da98f0e22a86a94d6a7e6f0f526959092f1a6ded895287b15dde2b78d84baf3b00828202581c6e56c2c5a5040fcda445ca0a87436e002e540870fad295f6557077bb8258204a184435fe098d9e62e429475b4e337be0acb369c84bbe39baa74740ef6d69d401").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0031_ConwayUtxowFailure_UtxoFailure_InsufficientCollateral_DeltaCoin() {
    verify_one("8182068282018200830c1a000d8a871a000b075682018209d9010280").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0032_ConwayTxRefScriptsSizeTooBig_ConwayGovFailure_ExpirationEpochTooSmall_List()
{
    verify_one("818206828306000082038207a18201581cff5c52bb623f42d8d4a48bc9393011167c82650799fe2057d0ffe17800").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0033_ConwayMempoolFailure_ConwayUtxowFailure_MalformedReferenceScripts_List() {
    verify_one("8182068282076082018211d9010281581cfa0165d3392a8938b5b5a4851d5802f43233ae1f9305b408f39477fd").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0034_ConwayUtxowFailure_InvalidWitnessesUTXOW_VKey_VerKeyEd() {
    verify_one("8182068282018201815820db765eede4a13a462c48279c62d3b614d3c936e765d8086470fff6f277e2d76e83051a000a786f1a000b2866").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0035_ConwayGovFailure_MalformedProposal_NewConstitution_SJust() {
    verify_one("81820682820382018305825820b50c881a532f8e51bf3d1b0297acad3066f93e8fc8e16793b07396c01a9d1ad3008282784068747470733a2f2f764875706f33544d47757179795967523271465a545a6c7845615069775875445a7261464c4a49786c584d753134386a514f30332e636f6d5820c6e1e8abededabeba0fb369afd37876919ff1b72c9bd0731304aa3221905e7bb581cbee75c2524e050e9d9c29e1882acc85241563a11d4f4a15158e1093183060100").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0036_ConwayTreasuryValueMismatch_Coin_Coin() {
    verify_one("8182068183051a0007f26b1a000590a2").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0037_ConwayTreasuryValueMismatch_Coin_Coin_ConwayMempoolFailure() {
    verify_one("8182068283051a000b3e831a000356b2820760").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0038_ConwayGovFailure_VotersDoNotExist_DRepVoter_KeyHashObj() {
    verify_one(
        "818206828203820e818202581c6405197a2f6592f55ba348f14d540f35caf3a1dedf1d40cd8e474e04820760",
    )
    .await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0039_ConwayUtxowFailure_PPViewHashesDontMatch_SJust_SafeHash() {
    verify_one("818206828201830d815820d15252d17f47ba042f29becbd844d9aabd83b7dafb52bd46589fb10358adb9618158201d97ba11111aee873b749791e741e6f6e9e3d3e7a5e2fc0e532ac6f323a120bf83051a0009c7ac1a000b7da8").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0040_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
    verify_one("81820682820482581c38bb836dc16459b5d268d6c035b49bd8c430d8f617d02af710795df8581cc39e3c910f66ac8ae6a8349e0878d4e529e89f992f24b811bbae222382038200828258204e1a131ff843d622e7bdecddf54b011955d12943181549df5e392faea5d7ae300182582030f363d5469e0099bd16a6f3f41bcbe77c8c8908a5a9ff3f2bc5eee5e62ce26500").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0041_ConwayMempoolFailure() {
    verify_one("81820681820763e684b9").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0042_ConwayGovFailure_ExpirationEpochTooSmall_List_ConwayWdrlNotDelegatedToDRep()
{
    verify_one("8182068282038207a0820482581c6a6d35b19d4013b919faf9a21cfe32571a540f78ff9d2da0b65d69bf581c8345ef94cf81079de12c4bc2f212f2caeb186eafc7af49d539d561ae").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0043_ConwayCertsFailure_CertFailure_DelegFailure_DelegateeDRepNotRegisteredDELEG(
) {
    verify_one(
        "8182068182028201820182058200581cb2f0655ce3475b94e5d46d3333f02849a53df7a6fbe82edca31c768d",
    )
    .await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0044_ConwayTxRefScriptsSizeTooBig() {
    verify_one("8182068183060100").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0045_ConwayTreasuryValueMismatch_Coin_Coin_ConwayMempoolFailure() {
    verify_one("8182068283051a0008c44d1a000db83b820760").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0046_ConwayTreasuryValueMismatch_Coin_Coin() {
    verify_one("8182068183051a000ccad81a00017d31").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0047_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
    verify_one("81820681820482581ca38aef4ba258adf98d062db8af793b3c0a8c8ac825f92ffb81733ac8581c45acc652b46732a00f29f02b674db5011023e47d639247fe68aa40a7").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0048_ConwayUtxowFailure_MissingScriptWitnessesUTXOW_List() {
    verify_one("8182068182018203d9010280").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0049_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_ConwayMempoolFailure() {
    verify_one("81820682820481581c97724706756be8c7ac5eeb86bc06402b70e44bb063c117fa678caa9382076148")
        .await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0050_ConwayMempoolFailure_ConwayCertsFailure_WithdrawalsNotInRewardsCERTS_List()
{
    verify_one("818206828207616582028200a0").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0051_ConwayTxRefScriptsSizeTooBig() {
    verify_one("8182068183060101").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0052_ConwayTxRefScriptsSizeTooBig() {
    verify_one("8182068183060100").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0053_ConwayMempoolFailure_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash() {
    verify_one("81820682820760820482581cdbcf0991fe989711d07ccfcf0752a3320c12cdb95a0bb6fac43234cc581cb59255ca1a3629862269e016c7be0c6110fc632091ea75aed9d8bba4").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0054_ConwayCertsFailure_WithdrawalsNotInRewardsCERTS_List_RewardAccount() {
    verify_one("8182068282028200a1581df143b29c77a36b9524cf908490eac5798394492f480ed39c16cd46ba851a0006d02283051a0005df2c19b453").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0055_ConwayUtxowFailure_PPViewHashesDontMatch_SJust_SafeHash() {
    verify_one(
        "818206818201830d815820722303d3f0c4127f8f7179faeef6c5865c686cd3833a90a2742e1c52e5403e2d80",
    )
    .await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0056_ConwayUtxowFailure_MissingVKeyWitnessesUTXOW_List_KeyHash() {
    verify_one(
        "8182068182018202d9010281581c615ba1eac6d914f3e9f0460095bd98ecaf0c54e25039cdfbea8d6783",
    )
    .await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0057_ConwayMempoolFailure_ConwayTreasuryValueMismatch_Coin_Coin() {
    verify_one("81820682820763e6888d83051a000b1e56191a4b").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0058_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
    verify_one("81820681820482581c3b7c0ec7f1f5093962dc65b03db33b0d90e0dbf6160a988826878b3f581c8952f1ce3cc36baecc375f5b9df12021d76b4ef8b345f0078f207fe4").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0059_ConwayCertsFailure_CertFailure_GovCertFailure_ConwayDRepNotRegistered() {
    verify_one(
        "8182068182028201820382018201581cce65a879625908607bdef0650cc4e4a651988525e28e93d4973927a3",
    )
    .await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0060_ConwayMempoolFailure() {
    verify_one("8182068182076160").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0061_ConwayTxRefScriptsSizeTooBig_ConwayTxRefScriptsSizeTooBig() {
    verify_one("818206828306010183060001").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0062_ConwayGovFailure_InvalidPrevGovActionId_ProposalProcedure_ProcDeposit() {
    verify_one("8182068282038208841a000c460e581df0e6870facdc5698dc244cd3045574aec568aafe49b62349d35fc0bcc4830582582093a147effacd3320f62c8b5fb6c5e7ccea7129824c3ce6c43421f80925465c3d008282782168747470733a2f2f506d644d65717034767267704852536e4c454546482e636f6d5820a6a9e86c80d2cafde846a4f53e0d14add8b063c40f7ecdf9e4d6457fbdc2d437581c7502bfb969ceace3373c3cbf2d01c414c5292b9464ccfce774366d3482781c68747470733a2f2f3872574c47567a374e646476765833382e636f6d58200c4714c06ce990a03c87dd5b3be0b2b186a4e58d49601f705ed4668f0815a85583051a000558721a0001c0f3").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0063_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_ConwayTxRefScriptsSizeTooBig()
{
    verify_one("81820682820481581cf2a2d54cff1ec0c393f060fdc2ae9e5a7b71abb4761f04de99aa11c883062000")
        .await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0064_ConwayTreasuryValueMismatch_Coin_Coin_ConwayCertsFailure() {
    verify_one("8182068283051a0003ff8c1a0001f35282028200a1581df0fbec21d31a02eefc76bfc8f2309199173103c8157c03c8237bdfd8bc198f8e").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0065_ConwayTreasuryValueMismatch_Coin_Coin() {
    verify_one("8182068183051a000def951a000190ac").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0066_ConwayGovFailure_InvalidPolicyHash_SJust_ScriptHash() {
    verify_one("818206828203830b81581c1588aa1e2f8ed73cdf28cbf7d06241099f44f3747b50708ecc22799081581cb20a46886f87fd81f4b51689734a786f10e683e65aa6f989dc547739820481581c7fab8efda7ee7d6b04955d250f1473970ece5476871b12de0b9435ab").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0067_ConwayCertsFailure_WithdrawalsNotInRewardsCERTS_List() {
    verify_one("8182068182028200a0").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0068_ConwayCertsFailure_WithdrawalsNotInRewardsCERTS_List_RewardAccount() {
    verify_one("8182068182028200a1581de077f4d91b50ac1d97149b599f9df0632a2a492e1e59e20acd72dab75d1a00068b45").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0069_ConwayMempoolFailure() {
    verify_one("81820681820764f0aab883").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0070_ConwayTxRefScriptsSizeTooBig_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash()
{
    verify_one("8182068283060100820481581c5d74ba4656ff6775b653b81cf012a26a9494475ea96bbe74eef53af8")
        .await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0071_ConwayUtxowFailure_MalformedReferenceScripts_List_ScriptHash() {
    verify_one(
        "8182068182018211d9010281581ce989cde904d0d693b63336252b5452163624ddd3beea935f21757e8b",
    )
    .await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0072_ConwayTreasuryValueMismatch_Coin_Coin_ConwayTreasuryValueMismatch() {
    verify_one("8182068283051a000110641a00062c9683051a0009d94a1a000f3cea").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0073_ConwayTreasuryValueMismatch_Coin_Coin_ConwayCertsFailure() {
    verify_one("8182068283051a00013c031a00035c0282028200a0").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0074_ConwayMempoolFailure_ConwayUtxowFailure_ExtraRedeemers_ConwayVoting() {
    verify_one("81820682820764f0a386828201820f81820401").await
}

#[tokio::test]
#[allow(non_snake_case)]
async fn test_cbor_0075_ConwayTreasuryValueMismatch_Coin_Coin_ConwayCertsFailure() {
    verify_one("8182068283051a000255e71a000a07cb82028200a1581de0b49233a1f4271a56406b81e8b1a732ea3d939771c1aefcd46a58e6d91a00027a44").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0076_ConwayMempoolFailure_ConwayUtxowFailure_NotAllowedSupplementalDatums_List()
{
    verify_one("818206828207608201830cd901028158209ee8bb48e4ee4e6af2752bddcec3ec694b31702e40ad7a6f6c7fd67414d06f09d9010280").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0077_ConwayMempoolFailure_SO() {
    verify_one("818206818207610e").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0078_ConwayMempoolFailure() {
    verify_one("81820681820760").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0079_ConwayTreasuryValueMismatch_Coin_Coin_ConwayGovFailure() {
    verify_one("8182068283051a0004964e1a000c916982038207a0").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0080_ConwayMempoolFailure_ConwayTxRefScriptsSizeTooBig() {
    verify_one("8182068282076083062001").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0081_ConwayMempoolFailure_ConwayCertsFailure_WithdrawalsNotInRewardsCERTS_List()
{
    verify_one("8182068282076082028200a0").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0082_ConwayMempoolFailure_S() {
    verify_one("8182068182076153").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0083_ConwayGovFailure_ExpirationEpochTooSmall_List_ConwayGovFailure() {
    verify_one("8182068282038207a08203820e818202581cd6d123ba0dd693a89694142a35714bc5b44f80a23ce66f4674808baf").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0084_ConwayCertsFailure_CertFailure_GovCertFailure_ConwayCommitteeIsUnknown() {
    verify_one(
        "8182068182028201820382058200581cd86ff1220850c197d0e48a4ac76ec60a9333c5e6d2cff13329931be5",
    )
    .await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0085_ConwayTreasuryValueMismatch_Coin_Coin() {
    verify_one("8182068183051a0006297c1a000456b4").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0086_ConwayUtxowFailure_UtxoFailure_OutputTooSmallUTxO_Addr() {
    verify_one("8182068182018200820981a400583931b2d82464ee0f01a997469f62bfd2f86e2b81e2d6b57f32c76f15ee65470d8980522f46457bd0238fb07a7a86a488a78b595e013f9aea9d5601821b229f5c403dc6c2e3a1581cb0c53e2bf180858da4b64eb5598c5615bba7d723d2b604a83b7f9165a141351b1567a03e35825e8c02820058204cd8ff721542ba2426af9d0fa46638f60559d368b7d2fe4d1651c63e28327f4803d818584e82008303008283030181830301818200581cabde7a6c2f96943f3d0258dbb7ac1fc8769230b7c2b1a42e6aa10d758200581c8eab33aa0947bb8fff0df44cf26fe832ed82105a10ade2cb819ca68b").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0087_ConwayMempoolFailure_ConwayMempoolFailure() {
    verify_one("8182068282076168820760").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0088_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
    verify_one("81820681820482581cf1d0ecac93ac1a4aa5dad924d9390424114baa9cc07417a67c648f06581c12749381aef4f99e67b5f6237fa797f3e0910a2f0eed50625b8c6753").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0089_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash() {
    verify_one("81820681820481581c5a6cf4280e7f2704e01ba558a5e2b329e8e8c240564ecac077d16b52").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0090_ConwayGovFailure_GovActionsDoNotExist_GovActionId_TxId() {
    verify_one("8182068182038200828258206d3a5b92857fd4ec3de160e62f194525286882ef073e27c4aa6e3e21f33e0dc100825820e015c9d455993f68e65ce4201f19ef00e24807b686682f1f0248b50d3369dcdf00").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0091_ConwayGovFailure_InvalidPrevGovActionId_ProposalProcedure_ProcDeposit() {
    verify_one("8182068182038208841a000c62e0581df11901d1a388e141decd73a9d866e8b345e628a725fc1446692052172c8400f6b818001a00056731011a000f1ff1020003010401051a000d28b2061a0006cc6f0701080009d81e821b0004a0e77a55d1891b000000012a05f2000ad81e821b00033ee519c0683d1b00038d7ea4c68000111a000cad301382d81e821b45cf3fc2ad19fb0b1b016345785d8a0000d81e821b00848874675c52fd1a000186a015821b27aeb4e29a22e1ff1b43ef678ea318e38316011701181801181985d81e820001d81e821b00022b0ac69461e71b0008e1bc9bf04000d81e821b00004f99d4554e731b00005af3107a4000d81e82190a2f191388d81e821b0000c0e0a2bebdf91b00038d7ea4c68000181a8ad81e821b0000000163fdad471b00000002540be400d81e821b00037889c53344c91b00038d7ea4c68000d81e8219aaf119c350d81e821b06991bf4e85f844d1b06f05b59d3b20000d81e821a011e79ff1a05f5e100d81e821a000785071a004c4b40d81e821b05b25cbd3ce69b111b0de0b6b3a7640000d81e821b000000185b7e3c491b00000048c2739500d81e821902931903e8d81e821a0106631f1a05f5e100181c01181e19b84c181f1a000e36061820001821d81e821b189d306e53f85d3d1819f6826e68747470733a2f2f31512e636f6d58202fa17b7ce34cc8670e9448b2b1b39b1d6262b9ec23abea0df5e61b41a88d55d8").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0092_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
    verify_one("81820682820482581c79eafef623f0826d947fefee02b033714530053fe318529dc9f1d92a581cda3bd010fea70e217b9859095390abaffc75e6d31c8e87409bafce5c82018210d9010280").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0093_ConwayGovFailure_ZeroTreasuryWithdrawals_NoConfidence_SJust() {
    verify_one("818206818203820f8203825820bce7857d66684c376526dd9431a81b9a456fbcc26064c56c43efa05e4e99440101").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0094_ConwayTxRefScriptsSizeTooBig() {
    verify_one("8182068183060101").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0095_ConwayUtxowFailure_NotAllowedSupplementalDatums_List_SafeHash() {
    verify_one("818206828201830cd901028158203c44228aea5f895b5acd0a677dabae4923d6ea1dad941c2f04e9e17fe88a98dbd901028158206f13d3fa99ccd62b8fd11c0d15d2a0b0d64d923e867d65bdcb9b740dd101664b820481581c2476e78bd0a862d6ef3de2a178d1f4f079bd2ac0ce4731a8bb3933b9").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0096_ConwayTreasuryValueMismatch_Coin_Coin() {
    verify_one("8182068183051a000a9c7c1a000f37b5").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0097_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
    verify_one("81820681820482581cce3e13a895ca723a377b52b880ede8d822b34fc497f7add6df50d8d4581ce1c21d6ecb67e54622923d5db2a600a9a53f6a5eec1878a0118838e0").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0098_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_ConwayMempoolFailure() {
    verify_one("81820682820481581c4c74aac383eae519dca60a0dade59b7f37bb4fd57ba0cb5a2908f86682076174")
        .await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0099_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash() {
    verify_one("81820681820481581c9b13cae8380b05cf14ce9744ee556118ce6e244500732bf7ccf9d2b6").await
}

#[tokio::test]
#[ignore]
#[allow(non_snake_case)]
async fn test_cbor_0100_ConwayUtxowFailure_InvalidMetadata_ConwayGovFailure_DisallowedProposalDuringBootstrap(
) {
    verify_one("81820682820181088203820c841a000236ae581df0552d969928f472c24f005ce4aaeb1a888ecc58f6e75ce0ac65a6be41810682783568747470733a2f2f3975576958314e416b6a6d764e7551775759525131705a75415059416e312d705974514568577249392e636f6d5820f70f8c1b6b57c2d9483bc98f062b088723b3f74e760419ccd4232abba36b75d3").await
}
