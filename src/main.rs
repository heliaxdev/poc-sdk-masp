use std::str::FromStr;

use namada_sdk::{
    args::{InputAmount, TxBuilder}, io::NullIo, masp::fs::FsShieldedUtils, rpc, signing::default_sign, tendermint::abci::Code, tx::data::ResultCode, types::{
        address::Address, chain::ChainId, key::{common::SecretKey, RefTo}, masp::{PaymentAddress, TransferSource, TransferTarget}
    }, wallet::fs::FsWalletUtils, Namada, NamadaImpl
};
use tendermint_rpc::{HttpClient, Url};

#[tokio::main]
async fn main() {
    let rpc = "https://proxy.heliax.click/shielded-expedition.88f17d1d14";
    let url = Url::from_str(&rpc).expect("invalid RPC address");
    let http_client = HttpClient::new(url).unwrap();

    let wallet = FsWalletUtils::new("wallet".into());

    let shielded_ctx = FsShieldedUtils::new("masp".into());

    let null_io = NullIo;

    let sdk = NamadaImpl::new(http_client, wallet, shielded_ctx, null_io)
        .await
        .expect("unable to initialize Namada context")
        .chain_id(ChainId::from_str("shielded-expedition.88f17d1d14").unwrap());

    let sk = "0037b6969d8017f4b41549e7555388f5c267a7e887f4e40a890fe3cc05bced8bf6";
    let sk = SecretKey::from_str(&sk).expect("Should be able to decode secret key.");
    let pk = sk.ref_to();
    let source_address = Address::from(&pk);

    let mut wallet = sdk.wallet.write().await;
    wallet
        .insert_keypair(
            "test".to_string(),
            true,
            sk.clone(),
            None,
            Some(source_address.clone()),
            None,
        )
        .unwrap();
    drop(wallet);

    let native_token = rpc::query_native_token(sdk.client()).await.unwrap();

    let denominated_amount =
        rpc::denominate_amount(sdk.client(), sdk.io(), &native_token, 10.into()).await;

    println!("Build transfer tx data...");

    let mut transfer_tx_builder = sdk.new_transfer(
        TransferSource::Address(source_address),
        TransferTarget::PaymentAddress(PaymentAddress::from_str("znam1qqd8rahacunfakac7mw0w3zvgvthg55qffc33vh6qnnu6sdty9gvfvarduku4umrczxsujcyp0scz").unwrap()),
        native_token.clone(),
        InputAmount::Unvalidated(denominated_amount),
    );

    println!("Build transfer tx...");

    let (mut transfer_tx, signing_data, _epoch) = transfer_tx_builder
        .build(&sdk)
        .await
        .expect("unable to build transfer");
    
    println!("Sign transfer tx...");

    sdk
        .sign(
            &mut transfer_tx,
            &transfer_tx_builder.tx,
            signing_data,
            default_sign,
            (),
        )
        .await
        .expect("unable to sign reveal pk tx");

    println!("Submit transfer tx...");

    let process_tx_response = sdk.submit(transfer_tx, &transfer_tx_builder.tx).await;

    let (transfer_result, tx_hash) = if let Ok(response) = process_tx_response {
        match response {
            namada_sdk::tx::ProcessTxResponse::Applied(r) => {
                (r.code.eq(&ResultCode::Ok), Some(r.hash))
            }
            namada_sdk::tx::ProcessTxResponse::Broadcast(r) => {
                (r.code.eq(&Code::Ok), Some(r.hash.to_string()))
            }
            _ => (false, None),
        }
    } else {
        (false, None)
    };

    println!("transfer success: {}, inner tx hash: {:?}", transfer_result, tx_hash);
       
}
