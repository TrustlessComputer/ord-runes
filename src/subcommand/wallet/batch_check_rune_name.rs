use super::*;

#[derive(Debug, Parser)]
pub(crate) struct BatchCheckRuneName {

  #[clap(long, help = "Name <RUNE>. May contain `.` or `•`as spacers.")]
  rune: SpacedRune,
  rune_destination: Address<NetworkUnchecked>,
}

impl BatchCheckRuneName {
  pub(crate) fn run(self, wallet: Wallet) -> SubcommandResult {
    let _destination=  self.rune_destination.clone().require_network(wallet.chain().network())?;
    let rune = self.rune.rune;
    Self::check_etching_rune_name(&wallet, rune)?;
    Ok(Some(Box::new("Success")))
  }

  fn check_etching_rune_name(wallet: &Wallet, rune : Rune) -> Result {
    ensure!(
      wallet.load_etching(rune)?.is_none(),
      "rune `{rune}` has pending etching, resume with `ord wallet resume`"
    );

    ensure!(!rune.is_reserved(), "rune `{rune}` is reserved");
    ensure!(
      wallet.has_rune_index(),
      "etching runes requires index created with `--index-runes`",
    );

    ensure!(
      wallet.get_rune(rune)?.is_none(),
      "rune `{rune}` has already been etched",
    );

    let bitcoin_client = wallet.bitcoin_client();
    let current_height = u32::try_from(bitcoin_client.get_block_count()?).unwrap();
    let reveal_height = current_height + u32::from(Runestone::COMMIT_CONFIRMATIONS);

    let minimum = Rune::minimum_at_height(wallet.chain().into(), Height(reveal_height));
    ensure!(
      rune >= minimum,
      "rune is less than minimum for next block: {rune} < {minimum}",
    );

    Ok(())
  }
}
