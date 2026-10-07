use std::collections::HashMap;

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


pub fn display_labels(accounts: &[Account]) -> Vec<String> {
    let mut seen: HashMap<String, usize> = HashMap::new();

    accounts
        .iter()
        .map(|account| {
            let label = account.label();
            let count = seen.entry(label.clone()).or_insert(0);
            *count += 1;

            if *count == 1 {
                label
            } else {
                format!("{label} [{}]", *count)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(secret: u8) -> Account {
        Account::new(
            "Discord",
            "user@example.test",
            vec![secret; 20],
            OtpAlgorithm::Sha1,
            6,
            30,
        )
    }

    #[test]
    fn duplicate_visible_labels_get_stable_suffixes() {
        let labels = display_labels(&[account(1), account(2), account(3)]);

        assert_eq!(
            labels,
            vec![
                "Discord — user@example.test",
                "Discord — user@example.test [2]",
                "Discord — user@example.test [3]",
            ]
        );
    }
}
