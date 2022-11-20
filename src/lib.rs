use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use ureq;

pub mod eth {
    use super::*;

    #[derive(Error, Debug)]
    pub enum Errors {
        #[error("Not a valid hex address: {0} is not prefixed by 0x")]
        AddressNotPrefixed(String),

        #[error("Not a valid hex address: {0} is not exactly 42 in length. Got: {1}")]
        AddressIncorrectLength(String, usize),

        #[error("Address {0} is not a contract")]
        NotAContract(String),
    }

    /// An address is a simple 42-byte identification for an account
    #[derive(Debug)]
    pub struct Address(String);

    impl TryFrom<&str> for Address {
        type Error = Errors;

        fn try_from(addr: &str) -> Result<Address, Errors> {
            addr.to_owned().try_into()
        }
    }

    impl TryFrom<String> for Address {
        type Error = Errors;

        fn try_from(addr: String) -> Result<Address, Errors> {
            if !addr.starts_with("0x") {
                return Err(Errors::AddressNotPrefixed(addr));
            }

            let len = addr.len();
            if len != 42 {
                return Err(Errors::AddressIncorrectLength(addr, len));
            }

            Ok(Address(addr))
        }
    }

    impl fmt::Display for Address {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.0)?;
            Ok(())
        }
    }

    /// Account has data associated with an Ethereum account.
    pub struct Account {
        address: Address,
        nonce: usize,
        balance: usize,
        code: Vec<u8>,
    }

    impl fmt::Display for Account {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "{} @ {} ({} ETH)",
                if self.is_eoa() { "EOA" } else { "Contract" },
                self.address,
                self.balance / 10usize.pow(8),
            )
        }
    }

    impl fmt::Debug for Account {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self)
        }
    }

    impl Account {
        pub fn new(address: Address, nonce: usize, balance: usize, code: Vec<u8>) -> Account {
            Account {
                address,
                nonce,
                balance,
                code,
            }
        }

        pub fn is_eoa(&self) -> bool {
            self.code.len() == 0
        }
    }

    pub struct Contract {
        account: eth::Account,
        name: Option<String>,
        abi: Option<String>,
        source: Option<String>,
        bytecode: String,
    }

    impl Contract {
        pub fn new(
            account: eth::Account,
            bytecode: String,
            source: Option<String>,
            abi: Option<String>,
            name: Option<String>,
        ) -> Result<Contract, Errors> {
            if bytecode.len() == 0 {
                return Err(Errors::NotAContract(account.to_string()));
            };

            Ok(Contract {
                account,
                name,
                bytecode,
                source,
                abi,
            })
        }
    }
}

pub mod etherscan {
    use super::*;
    use std::io;
    use std::{fmt::Error, time::Duration};

    use ureq;

    use super::eth;

    const ETHERSCAN_URL: &str = "https://api.etherscan.io/api";

    #[derive(Error, Debug)]
    pub enum Errors {
        #[error("HTTP connection error: ")]
        HTTPError(#[from] ureq::Error),

        #[error("JSON error")]
        JSONError(#[from] io::Error),
    }

    /// The response from the JSON APIs. All of the calls give the same top-level JSON
    /// which only varies in the type T of the result
    #[derive(Serialize, Deserialize, Debug)]
    pub struct Response<T> {
        status: String,
        message: String,
        result: Vec<T>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct SourceCode {
        #[serde(rename = "SourceCode")]
        source: String,
        #[serde(rename = "ConstructorArguments")]
        constructor_args: String,
        #[serde(rename = "ContractName")]
        contract_name: String,
    }

    pub struct ABI(String);

    /// A client to interact with Etherescan
    pub struct Client {
        apikey: String,
        url: String,
        http: ureq::Agent,
    }

    impl Client {
        /// Creates a new client for the Etherescan API with a given API key
        /// The default HTTP client has a five-second timeout
        pub fn new(apikey: String) -> Client {
            let http = ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(5))
                .build();

            Self::new_with_custom_http(apikey, http)
        }

        /// Create a new client for the Etherescan API with a given API key
        /// plus a custom ureq agent.
        // It would greate help if anyone could pass their own abstract
        // http client here. That's easy to achieve in Go, but could not find
        // a reasonable way in Rust.
        pub fn new_with_custom_http(apikey: String, http: ureq::Agent) -> Client {
            let url = format!("{}?apikey={}", ETHERSCAN_URL, apikey);
            Client { apikey, url, http }
        }

        // Weird rust probably incoming?  Higher-ranked trait bounds
        // https://doc.rust-lang.org/nomicon/hrtb.html
        // https://stackoverflow.com/questions/70557666/what-does-this-higher-ranked-trait-bound-mean
        // Deserialize is trait bound by a lifetime 'de, which
        // is the lifetime of the data.
        // Here we say that for all A, the data the deserializer will have access to
        // will outlive it. No matter the lifetime of the deserializer itself.
        fn get<T: for<'a> Deserialize<'a>>(&self, url: &str) -> Result<Response<T>, Errors> {
            let res: Response<T> = self.http.get(&url).call()?.into_json()?;
            Ok(res)
        }

        pub fn get_source_code(&self, addr: &eth::Address) -> Result<Response<SourceCode>, Errors> {
            let url = format!(
                "{}/&module=contract&action=getsourcecode&address={}",
                self.url, addr,
            );

            self.get(&url)
        }

        pub fn get_abi(&self, addr: &eth::Address) -> Result<Response<String>, Errors> {
            let url = format!(
                "{}/&module=contract&action=getsourcecode&address={}",
                self.url, addr,
            );

            self.get(&url)
        }
    }
}
