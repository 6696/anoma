use anoma_tx_prelude::*;

#[transaction]
fn apply_tx(tx_data: Vec<u8>) {
    let signed =
        key::ed25519::SignedTxData::try_from_slice(&tx_data[..]).unwrap();

    let tx_data =
        intent::IntentTransfers::try_from_slice(&signed.data.unwrap()[..]);

    let tx_data = tx_data.unwrap();

    log_string(format!("apply_tx called with data: {:#?}", tx_data));

    // make sure that the matchmaker has to validate this tx
    insert_verifier(&tx_data.source);

    for token::Transfer {
        source,
        target,
        token,
        amount,
    } in tx_data.matches.transfers
    {
        token::transfer(&source, &target, &token, amount);
    }

    // tx_data
    //     .matches
    //     .exchanges
    //     .values()
    //     .into_iter()
    //     .for_each(intent::invalidate_exchange);
}


#[cfg(test)]
mod tests {
    use anoma_tests::tx::*;

    use super::*;

    /// An example test, checking that this transaction performs no storage
    /// modifications.
    #[test]
    fn test_no_op_transaction() {
        // The environment must be initialized first
        let mut env = TestTxEnv::default();
        init_tx_env(&mut env);

        let tx_data = vec![];
        apply_tx(tx_data);

        assert!(env.all_touched_storage_keys().is_empty());
    }
}
