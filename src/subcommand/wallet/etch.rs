use super::*;
use reqwest;
use serde_json::from_str;
#[derive(Debug, Parser)]
pub(crate) struct Etch {
  #[clap(long, help = "Set divisibility to <DIVISIBILITY>.")]
  divisibility: u8,
  #[clap(long, help = "Etch with fee rate of <FEE_RATE> sats/vB.")]
  fee_rate: FeeRate,
  #[clap(long, help = "Etch rune <RUNE>. May contain `.` or `•`as spacers.")]
  rune: SpacedRune,
  #[clap(long, help = "Set supply to <SUPPLY>.")]
  supply: Decimal,
  #[clap(long, help = "Set currency symbol to <SYMBOL>.")]
  symbol: char,
  #[clap(long, help = "Set premine symbol to <SYMBOL>.")]
  premine:u128,
  #[clap(long, help = "Set amount symbol to <SYMBOL>.")]
   amount: u128,
  #[clap(long, help = "Set cap symbol to <SYMBOL>.")]
  cap: u128,
  #[clap(long, help = "Set height start symbol to <SYMBOL>.")]
  height_start: u64,
  #[clap(long, help = "Set height end symbol to <SYMBOL>.")]
  height_end :u64,
  #[clap(long, help = "Set offset start symbol to <SYMBOL>.")]
  offset_start: u64,
  #[clap(long, help = "Set offset start symbol to <SYMBOL>.")]
  offset_end:u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
  pub rune: SpacedRune,
  pub transaction: Txid,
}

#[derive(Debug, Deserialize, Serialize)]
struct ResponseData {
  tx_id: String,
  vout: u32,
}

impl Etch {
  pub(crate) async fn run(self, wallet: Wallet) -> SubcommandResult {
    ensure!(
      wallet.has_rune_index(),
      "`ord wallet etch` requires index created with `--index-runes` flag",
    );

    let SpacedRune { rune, spacers } = self.rune;

    let bitcoin_client = wallet.bitcoin_client();

    let count = bitcoin_client.get_block_count()?;

    ensure!(
      wallet.get_rune(rune)?.is_none(),
      "rune `{}` has already been etched",
      rune,
    );

    let minimum_at_height =
      Rune::minimum_at_height(wallet.chain().network(), Height(u32::try_from(count).unwrap() + 1));

    ensure!(
      rune >= minimum_at_height,
      "rune is less than minimum for next block: {} < {minimum_at_height}",
      rune,
    );

    ensure!(!rune.is_reserved(), "rune `{}` is reserved", rune);

    ensure!(
      self.divisibility <= Etching::MAX_DIVISIBILITY,
      "<DIVISIBILITY> must be equal to or less than 38"
    );

    let destination = wallet.get_change_address()?;
    println!("commitment {:?}",rune.commitment());
    let runestone = Runestone {
      etching: Some(Etching {
        divisibility: Some(self.divisibility),
        premine: Some(self.premine),
        terms: Some(Terms{
          amount: Some(self.amount),
           cap:Some(self.cap),
           height:(Some(self.height_start),Some(self.height_end)),
          offset:(Some(self.offset_start),Some(self.offset_end)),
        }),
        rune: Some(rune),
        spacers:Some(spacers),
        symbol: Some(self.symbol),
      }),
      edicts: vec![Edict {
        amount: self.supply.to_integer(self.divisibility)?,
        id: RuneId::default(),
        output: 0,
      }],
      mint:None,
      pointer:None,
    };

    let script_pubkey = runestone.encipher();
    ensure!(
      script_pubkey.len() <= 82,
      "runestone greater than maximum OP_RETURN size: {} > 82",
      script_pubkey.len()
    );
    let commitment = rune.commitment();

    let response = reqwest::get("https://example.com")
        .await?
        .text()
        .await?;

    let parsed: ResponseData = match from_str(&response) {
      Ok(data) => data,
      Err(e) => {
        eprintln!("Failed to parse JSON: {}", e),
      }
    };
    let mut txIn =TxIn{
      previous_output: OutPoint{
        txid: parsed.tx_id?,
        vout: parsed.vout,
      },
      script_sig: Default::default(),
      sequence: Default::default(),
      witness:Witness::new(),
    };
    txIn.witness.push(commitment);
    let unfunded_transaction = Transaction {
      version: 2,
      lock_time: LockTime::ZERO,
      input: vec![txIn,],
      output: vec![
        TxOut {
          script_pubkey,
          value: 0,
        },
        TxOut {
          script_pubkey: destination.script_pubkey(),
          value: TARGET_POSTAGE.to_sat(),
        },
      ],
    };
    let inscriptions = wallet
      .inscriptions()
      .keys()
      .map(|satpoint| satpoint.outpoint)
      .collect::<Vec<OutPoint>>();

    if !bitcoin_client.lock_unspent(&inscriptions)? {
      bail!("failed to lock UTXOs");
    }

    let unsigned_transaction =
      fund_raw_transaction(bitcoin_client, self.fee_rate, &unfunded_transaction)?;

    let signed_transaction = bitcoin_client
      .sign_raw_transaction_with_wallet(&unsigned_transaction, None, None)?
      .hex;

    let transaction = bitcoin_client.send_raw_transaction(&signed_transaction)?;

    Ok(Some(Box::new(Output {
      rune: self.rune,
      transaction,
    })))
  }
}
