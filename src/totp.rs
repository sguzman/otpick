use crate::model::{Account, OtpAlgorithm};
use totp_rs::{Algorithm, Builder, Totp, TotpError};

pub fn generate_current(account: &Account) -> Result<String, TotpError> {
    Ok(build(account)?.generate_current().to_string())
}

fn build(account: &Account) -> Result<Totp, TotpError> {
    Builder::new()
        .with_algorithm(match account.algorithm {
            OtpAlgorithm::Sha1 => Algorithm::SHA1,
            OtpAlgorithm::Sha256 => Algorithm::SHA256,
            OtpAlgorithm::Sha512 => Algorithm::SHA512,
        })
        .with_digits(account.digits)
        .with_step_duration(account.period)
        .with_secret(account.secret().to_vec())
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let token = build(&account).unwrap().generate(59);

        assert_eq!(token.to_string(), "94287082");
    }
}
