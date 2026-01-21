use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    /// A collection of pipes the `pow` proxy will open.
    pub pipes: Vec<Pipe>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Protocol {
    Grunt { host: String },
    BattleNET { host: String, port: u16 }
}

#[allow(unused)]
macro_rules! defaulted_amount {
    ($name:ident, $value:expr) => {
        #[derive(Serialize, Deserialize, Debug)]
        pub struct $name(usize);
        impl Default for $name {
            fn default() -> Self {
                Self($value)
            }
        }
        impl Deref for $name {
            type Target = usize;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Pipe {
    /// The endpoint the `pow` proxy will listen on.
    pub source: Protocol,

    /// The target server the `pow` proxy must impersonate.
    pub destination: Protocol,
}
