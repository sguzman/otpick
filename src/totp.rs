use crate::model::{Account, OtpAlgorithm};
use totp_rs::{Algorithm, Builder, Totp};

pub fn generate_current(account: &Account) -> String {
    build(account).generate_current().to_string()
}

fn build(account: &Account) -> Totp {
    Builder::new()
        .with_algorithm(match account.algorithm {
            OtpAlgorithm::Sha1 => Algorithm::SHA1,
            OtpAlgorithm::Sha256 => Algorithm::SHA256,
            OtpAlgorithm::Sha512 => Algorithm::SHA512,
        })
        .with_digits(account.digits)
        .with_step_duration(account.period)
        .with_secret(account.secret().to_vec())
        .build_noncompliant()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_legacy_short_secret() {
        let account = Account::new(
            "Legacy",
            "test",
            b"1234567890".to_vec(),
            OtpAlgorithm::Sha1,
            6,
            30,
        );

        assert_eq!(generate_current(&account).len(), 6);
    }

    #[test]
    fn matches_rfc_6238_sha1_vector() {
        let account = Account::new(
            "RFC",
            "test",
            b"12345678901234567890".to_vec(),
            OtpAlgorithm::Sha1,
            8,
            30,
        );

        let token = build(&account).generate(59);

        assert_eq!(token.to_string(), "94287082");
    }
}
