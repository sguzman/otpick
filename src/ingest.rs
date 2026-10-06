use percent_encoding::percent_decode_str;
use totp_rs::Secret;
use zeroize::Zeroizing;

use crate::model::{Account, OtpAlgorithm};

const PREFIX: &str = "otpauth://totp/";

pub fn parse_otpauth_document(input: &str) -> Result<Vec<Account>, IngestError> {
    let mut accounts = Vec::new();

    for line in input.lines().map(str::trim).filter(|line| !line.is_empty()) {
        accounts.push(parse_otpauth(line)?);
    }

    if accounts.is_empty() {
        return Err(IngestError::Empty);
    }

    Ok(accounts)
}

pub fn parse_otpauth(uri: &str) -> Result<Account, IngestError> {
    let remainder = uri
        .strip_prefix(PREFIX)
        .ok_or(IngestError::UnsupportedUri)?;
    let (raw_label, raw_query) = remainder.split_once('?').ok_or(IngestError::MissingQuery)?;

    let label = decode_component(raw_label, false, "label")?;
    let (label_issuer, account) = match label.split_once(':') {
        Some((issuer, account)) => (nonempty(issuer), account.to_owned()),
        None => (None, label),
    };

    if account.is_empty() {
        return Err(IngestError::MissingAccount);
    }

    let mut secret: Option<Zeroizing<String>> = None;
    let mut query_issuer: Option<String> = None;
    let mut algorithm: Option<String> = None;
    let mut digits: Option<String> = None;
    let mut period: Option<String> = None;

    for pair in raw_query.split('&').filter(|pair| !pair.is_empty()) {
        let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = decode_component(raw_key, true, "query key")?;
        let value = decode_component(raw_value, true, "query value")?;

        match key.as_str() {
            "secret" => set_once_secret(&mut secret, value)?,
            "issuer" => set_once(&mut query_issuer, value, "issuer")?,
            "algorithm" => set_once(&mut algorithm, value, "algorithm")?,
            "digits" => set_once(&mut digits, value, "digits")?,
            "period" => set_once(&mut period, value, "period")?,
            _ => {}
        }
    }

    let secret = secret.ok_or(IngestError::MissingSecret)?;
    let secret =
        Secret::try_from_base32(secret.as_str()).map_err(|_| IngestError::InvalidSecret)?;

    let algorithm = parse_algorithm(algorithm.as_deref().unwrap_or("SHA1"))?;
    let digits = parse_digits(digits.as_deref().unwrap_or("6"))?;
    let period = parse_period(period.as_deref().unwrap_or("30"))?;

    let query_issuer = query_issuer.and_then(nonempty_owned);
    if let (Some(label_issuer), Some(query_issuer)) = (&label_issuer, &query_issuer)
        && label_issuer != query_issuer
    {
        return Err(IngestError::IssuerMismatch);
    }

    let issuer = query_issuer.or(label_issuer).unwrap_or_default();

    Ok(Account::new(
        issuer,
        account,
        secret.as_bytes().to_vec(),
        algorithm,
        digits,
        period,
    ))
}

pub fn account_from_base32(
    issuer: String,
    account: String,
    secret: &str,
) -> Result<Account, IngestError> {
    if account.is_empty() {
        return Err(IngestError::MissingAccount);
    }

    let secret = Secret::try_from_base32(secret).map_err(|_| IngestError::InvalidSecret)?;
    Ok(Account::new(
        issuer,
        account,
        secret.as_bytes().to_vec(),
        OtpAlgorithm::Sha1,
        6,
        30,
    ))
}

pub fn ensure_unique(existing: &[Account], incoming: &[Account]) -> Result<(), IngestError> {
    for (index, candidate) in incoming.iter().enumerate() {
        if existing
            .iter()
            .any(|account| same_identity(account, candidate))
            || incoming[..index]
                .iter()
                .any(|account| same_identity(account, candidate))
        {
            return Err(IngestError::DuplicateIdentity(candidate.label()));
        }
    }

    Ok(())
}

fn same_identity(left: &Account, right: &Account) -> bool {
    left.issuer == right.issuer && left.account == right.account
}

fn parse_algorithm(value: &str) -> Result<OtpAlgorithm, IngestError> {
    match value.to_ascii_uppercase().as_str() {
        "SHA1" => Ok(OtpAlgorithm::Sha1),
        "SHA256" => Ok(OtpAlgorithm::Sha256),
        "SHA512" => Ok(OtpAlgorithm::Sha512),
        _ => Err(IngestError::InvalidAlgorithm(value.to_owned())),
    }
}

fn parse_digits(value: &str) -> Result<u8, IngestError> {
    let digits = value
        .parse::<u8>()
        .map_err(|_| IngestError::InvalidDigits(value.to_owned()))?;

    if !(6..=8).contains(&digits) {
        return Err(IngestError::InvalidDigits(value.to_owned()));
    }

    Ok(digits)
}

fn parse_period(value: &str) -> Result<u64, IngestError> {
    let period = value
        .parse::<u64>()
        .map_err(|_| IngestError::InvalidPeriod(value.to_owned()))?;

    if period == 0 {
        return Err(IngestError::InvalidPeriod(value.to_owned()));
    }

    Ok(period)
}

fn decode_component(
    value: &str,
    plus_as_space: bool,
    context: &'static str,
) -> Result<String, IngestError> {
    if plus_as_space && value.contains('+') {
        let normalized = value.replace('+', " ");
        return percent_decode_str(&normalized)
            .decode_utf8()
            .map(|value| value.into_owned())
            .map_err(|_| IngestError::InvalidEncoding(context));
    }

    percent_decode_str(value)
        .decode_utf8()
        .map(|value| value.into_owned())
        .map_err(|_| IngestError::InvalidEncoding(context))
}

fn set_once(
    slot: &mut Option<String>,
    value: String,
    name: &'static str,
) -> Result<(), IngestError> {
    if slot.replace(value).is_some() {
        return Err(IngestError::DuplicateParameter(name));
    }
    Ok(())
}

fn set_once_secret(slot: &mut Option<Zeroizing<String>>, value: String) -> Result<(), IngestError> {
    if slot.replace(Zeroizing::new(value)).is_some() {
        return Err(IngestError::DuplicateParameter("secret"));
    }
    Ok(())
}

fn nonempty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

fn nonempty_owned(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("only otpauth://totp URIs are supported")]
    UnsupportedUri,
    #[error("otpauth URI has no query string")]
    MissingQuery,
    #[error("invalid UTF-8 encoding in {0}")]
    InvalidEncoding(&'static str),
    #[error("TOTP account label is empty")]
    MissingAccount,
    #[error("TOTP secret is missing")]
    MissingSecret,
    #[error("TOTP secret is not valid base32")]
    InvalidSecret,
    #[error("unsupported TOTP algorithm: {0}")]
    InvalidAlgorithm(String),
    #[error("invalid TOTP digit count: {0}")]
    InvalidDigits(String),
    #[error("invalid TOTP period: {0}")]
    InvalidPeriod(String),
    #[error("issuer in label does not match issuer query parameter")]
    IssuerMismatch,
    #[error("duplicate otpauth parameter: {0}")]
    DuplicateParameter(&'static str),
    #[error("duplicate account identity: {0}")]
    DuplicateIdentity(String),
    #[error("import input contains no otpauth entries")]
    Empty,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

    #[test]
    fn parses_standard_otpauth_uri() {
        let uri = format!(
            "otpauth://totp/GitHub%3Aalice%40example.com?secret={SECRET}&issuer=GitHub&algorithm=SHA256&digits=8&period=45"
        );
        let account = parse_otpauth(&uri).unwrap();

        assert_eq!(account.issuer, "GitHub");
        assert_eq!(account.account, "alice@example.com");
        assert_eq!(account.algorithm, OtpAlgorithm::Sha256);
        assert_eq!(account.digits, 8);
        assert_eq!(account.period, 45);
        assert_eq!(account.secret(), b"12345678901234567890");
    }

    #[test]
    fn defaults_match_normal_totp() {
        let uri = format!("otpauth://totp/alice?secret={SECRET}");
        let account = parse_otpauth(&uri).unwrap();

        assert_eq!(account.issuer, "");
        assert_eq!(account.account, "alice");
        assert_eq!(account.algorithm, OtpAlgorithm::Sha1);
        assert_eq!(account.digits, 6);
        assert_eq!(account.period, 30);
    }

    #[test]
    fn rejects_issuer_mismatch() {
        let uri = format!("otpauth://totp/GitHub:alice?secret={SECRET}&issuer=GitLab");

        assert!(matches!(
            parse_otpauth(&uri),
            Err(IngestError::IssuerMismatch)
        ));
    }

    #[test]
    fn document_import_is_all_entries() {
        let input = format!(
            "otpauth://totp/One:a?secret={SECRET}&issuer=One\n\notpauth://totp/Two:b?secret={SECRET}&issuer=Two\n"
        );

        let accounts = parse_otpauth_document(&input).unwrap();

        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].label(), "One — a");
        assert_eq!(accounts[1].label(), "Two — b");
    }

    #[test]
    fn rejects_hotp() {
        let uri = format!("otpauth://hotp/alice?secret={SECRET}&counter=1");
        assert!(matches!(
            parse_otpauth(&uri),
            Err(IngestError::UnsupportedUri)
        ));
    }

    #[test]
    fn rejects_duplicate_identity() {
        let first = parse_otpauth(&format!(
            "otpauth://totp/GitHub:alice?secret={SECRET}&issuer=GitHub"
        ))
        .unwrap();
        let second = parse_otpauth(&format!(
            "otpauth://totp/GitHub:alice?secret={SECRET}&issuer=GitHub"
        ))
        .unwrap();

        assert!(matches!(
            ensure_unique(&[], &[first, second]),
            Err(IngestError::DuplicateIdentity(_))
        ));
    }
}
