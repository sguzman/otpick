use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OtpAlgorithm {
    Sha1,
    Sha256,
    Sha512,
}

#[derive(Deserialize, Serialize)]
pub struct Account {
    pub issuer: String,
    pub account: String,
    pub algorithm: OtpAlgorithm,
    pub digits: u8,
    pub period: u64,
    secret: Zeroizing<Vec<u8>>,
}

impl Account {
    pub fn new(
        issuer: impl Into<String>,
        account: impl Into<String>,
        secret: Vec<u8>,
        algorithm: OtpAlgorithm,
        digits: u8,
        period: u64,
    ) -> Self {
        Self {
            issuer: issuer.into(),
            account: account.into(),
            algorithm,
            digits,
            period,
            secret: Zeroizing::new(secret),
        }
    }

    pub fn secret(&self) -> &[u8] {
        self.secret.as_slice()
    }

    pub fn label(&self) -> String {
        if self.account.is_empty() {
            self.issuer.clone()
        } else if self.issuer.is_empty() {
            self.account.clone()
        } else {
            format!("{} — {}", self.issuer, self.account)
        }
    }
}
