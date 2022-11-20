use evm_callgraph::{eth, etherscan};
use std::env;

fn main() {
    let etherscan_apikey = env::var("ETHERSCAN_API").ok().unwrap();

    let etherscan = etherscan::Client::new(etherscan_apikey.to_owned());

    let addr: eth::Address = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
        .try_into()
        .unwrap();

    let _source_code = etherscan.get_source_code(&addr);
    let _abi = etherscan.get_abi(&addr);
}
