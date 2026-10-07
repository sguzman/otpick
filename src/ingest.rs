use std::collections::BTreeMap;

use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use percent_encoding::percent_decode_str;
use prost::Message;
use totp_rs::Secret;
use zeroize::{Zeroize, Zeroizing};

use crate::model::{Account, OtpAlgorithm};

const OTPAUTH_PREFIX: &str = "otpauth://totp/";
const MIGRATION_PREFIX: &str = "otpauth-migration://offline?";

pub fn parse_document(input: &str) -> Result<Vec<Account>, IngestError> {
    let mut accounts = Vec::new();
    let mut batches: BTreeMap<i32, BatchAccumulator> = BTreeMap::new();
    let mut saw_input = false;

    for line in input.lines().map(str::trim).filter(|line| !line.is_empty()) {
        saw_input = true;

        if line.starts_with(OTPAUTH_PREFIX) {
            accounts.push(parse_otpauth(line)?);
        } else if line.starts_with(MIGRATION_PREFIX) {
            let fragment = parse_migration(line)?;
            batches
                .entry(fragment.batch_id)
                .or_insert_with(|| BatchAccumulator::new(fragment.batch_size))
                .insert(fragment)?;
        } else {
            return Err(IngestError::UnsupportedUri);
        }
    }

    if !saw_input {
        return Err(IngestError::Empty);
    }

    for (_, batch) in batches {
        accounts.extend(batch.finish()?);
    }

    Ok(accounts)
}

pub fn parse_otpauth(uri: &str) -> Result<Account, IngestError> {
    let remainder = uri
        .strip_prefix(OTPAUTH_PREFIX)
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

fn parse_migration(uri: &str) -> Result<MigrationFragment, IngestError> {
    let query = uri
        .strip_prefix(MIGRATION_PREFIX)
        .ok_or(IngestError::UnsupportedUri)?;

    let mut encoded_data: Option<Zeroizing<String>> = None;
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key == "data" {
            if encoded_data.is_some() {
                return Err(IngestError::DuplicateParameter("data"));
            }

            let decoded = percent_decode_str(value)
                .decode_utf8()
                .map_err(|_| IngestError::InvalidEncoding("migration data"))?;
            encoded_data = Some(Zeroizing::new(decoded.into_owned()));
        }
    }

    let encoded_data = encoded_data.ok_or(IngestError::MissingMigrationData)?;
    let raw = decode_base64(encoded_data.as_bytes())?;
    let mut payload =
        MigrationPayload::decode(raw.as_slice()).map_err(IngestError::MigrationProtobuf)?;

    if !matches!(payload.version, 1 | 2) {
        return Err(IngestError::UnsupportedMigrationVersion(payload.version));
    }
    if payload.batch_size <= 0
        || payload.batch_index < 0
        || payload.batch_index >= payload.batch_size
    {
        return Err(IngestError::InvalidMigrationBatch);
    }

    let params = std::mem::take(&mut payload.otp_parameters);
    let mut accounts = Vec::with_capacity(params.len());
    for parameter in params {
        accounts.push(migration_account(parameter)?);
    }

    Ok(MigrationFragment {
        batch_id: payload.batch_id,
        batch_size: payload.batch_size,
        batch_index: payload.batch_index,
        accounts,
    })
}

fn migration_account(mut parameter: MigrationOtpParameters) -> Result<Account, IngestError> {
    if parameter.otp_type != 2 {
        return Err(match parameter.otp_type {
            1 => IngestError::HotpUnsupported,
            other => IngestError::InvalidMigrationType(other),
        });
    }

    if parameter.secret.is_empty() {
        return Err(IngestError::MissingSecret);
    }

    let algorithm = match parameter.algorithm {
        0 | 1 => OtpAlgorithm::Sha1,
        2 => OtpAlgorithm::Sha256,
        3 => OtpAlgorithm::Sha512,
        other => return Err(IngestError::InvalidMigrationAlgorithm(other)),
    };
    let digits = match parameter.digits {
        0 | 1 => 6,
        2 => 8,
        other => return Err(IngestError::InvalidMigrationDigits(other)),
    };

    let mut issuer = std::mem::take(&mut parameter.issuer);
    let mut account = std::mem::take(&mut parameter.name);

    if issuer.is_empty() {
        if let Some((inferred_issuer, inferred_account)) = account.split_once(':') {
            issuer = inferred_issuer.to_owned();
            account = inferred_account.to_owned();
        }
    } else {
        let prefix = format!("{issuer}:");
        if let Some(stripped) = account.strip_prefix(&prefix) {
            account = stripped.to_owned();
        }
    }

    if account.is_empty() {
        return Err(IngestError::MissingAccount);
    }

    let secret = std::mem::take(&mut parameter.secret);
    Ok(Account::new(issuer, account, secret, algorithm, digits, 30))
}

fn decode_base64(encoded: &[u8]) -> Result<Zeroizing<Vec<u8>>, IngestError> {
    for engine in [&STANDARD, &STANDARD_NO_PAD, &URL_SAFE, &URL_SAFE_NO_PAD] {
        if let Ok(decoded) = engine.decode(encoded) {
            return Ok(Zeroizing::new(decoded));
        }
    }

    Err(IngestError::InvalidMigrationBase64)
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

struct MigrationFragment {
    batch_id: i32,
    batch_size: i32,
    batch_index: i32,
    accounts: Vec<Account>,
}

struct BatchAccumulator {
    batch_size: i32,
    parts: BTreeMap<i32, Vec<Account>>,
}

impl BatchAccumulator {
    fn new(batch_size: i32) -> Self {
        Self {
            batch_size,
            parts: BTreeMap::new(),
        }
    }

    fn insert(&mut self, fragment: MigrationFragment) -> Result<(), IngestError> {
        if fragment.batch_size != self.batch_size {
            return Err(IngestError::InconsistentMigrationBatch);
        }
        if self
            .parts
            .insert(fragment.batch_index, fragment.accounts)
            .is_some()
        {
            return Err(IngestError::DuplicateMigrationPart(fragment.batch_index));
        }
        Ok(())
    }

    fn finish(mut self) -> Result<Vec<Account>, IngestError> {
        if self.parts.len() != self.batch_size as usize {
            return Err(IngestError::IncompleteMigrationBatch {
                expected: self.batch_size,
                found: self.parts.len(),
            });
        }

        let total = self.parts.values().map(Vec::len).sum();
        let mut accounts = Vec::with_capacity(total);
        for index in 0..self.batch_size {
            let part = self
                .parts
                .remove(&index)
                .ok_or(IngestError::MissingMigrationPart(index))?;
            accounts.extend(part);
        }
        Ok(accounts)
    }
}

#[derive(Clone, PartialEq, Message, Zeroize)]
#[zeroize(drop)]
struct MigrationPayload {
    #[prost(message, repeated, tag = "1")]
    otp_parameters: Vec<MigrationOtpParameters>,
    #[prost(int32, tag = "2")]
    version: i32,
    #[prost(int32, tag = "3")]
    batch_size: i32,
    #[prost(int32, tag = "4")]
    batch_index: i32,
    #[prost(int32, tag = "5")]
    batch_id: i32,
}

#[derive(Clone, PartialEq, Message, Zeroize)]
#[zeroize(drop)]
struct MigrationOtpParameters {
    #[prost(bytes = "vec", tag = "1")]
    secret: Vec<u8>,
    #[prost(string, tag = "2")]
    name: String,
    #[prost(string, tag = "3")]
    issuer: String,
    #[prost(int32, tag = "4")]
    algorithm: i32,
    #[prost(int32, tag = "5")]
    digits: i32,
    #[prost(int32, tag = "6")]
    otp_type: i32,
    #[prost(int64, tag = "7")]
    counter: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("unsupported OTP URI")]
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
    #[error("duplicate OTP parameter: {0}")]
    DuplicateParameter(&'static str),
    #[error("duplicate account identity: {0}")]
    DuplicateIdentity(String),
    #[error("import input contains no OTP entries")]
    Empty,
    #[error("Google Authenticator migration data is missing")]
    MissingMigrationData,
    #[error("Google Authenticator migration data is not valid base64")]
    InvalidMigrationBase64,
    #[error("invalid Google Authenticator migration protobuf: {0}")]
    MigrationProtobuf(prost::DecodeError),
    #[error("unsupported Google Authenticator migration version {0}")]
    UnsupportedMigrationVersion(i32),
    #[error("invalid Google Authenticator migration batch metadata")]
    InvalidMigrationBatch,
    #[error("inconsistent Google Authenticator migration batch")]
    InconsistentMigrationBatch,
    #[error("duplicate Google Authenticator migration part {0}")]
    DuplicateMigrationPart(i32),
    #[error(
        "incomplete Google Authenticator migration batch: expected {expected} parts, found {found}"
    )]
    IncompleteMigrationBatch { expected: i32, found: usize },
    #[error("Google Authenticator migration batch is missing part {0}")]
    MissingMigrationPart(i32),
    #[error("HOTP migration entries are not supported")]
    HotpUnsupported,
    #[error("invalid Google Authenticator OTP type {0}")]
    InvalidMigrationType(i32),
    #[error("unsupported Google Authenticator algorithm id {0}")]
    InvalidMigrationAlgorithm(i32),
    #[error("unsupported Google Authenticator digit-count id {0}")]
    InvalidMigrationDigits(i32),
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

        let accounts = parse_document(&input).unwrap();

        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].label(), "One — a");
        assert_eq!(accounts[1].label(), "Two — b");
    }

    #[test]
    fn rejects_hotp_uri() {
        let uri = format!("otpauth://hotp/alice?secret={SECRET}&counter=1");
        assert!(matches!(
            parse_document(&uri),
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

    #[test]
    fn migration_single_part_imports_account() {
        let uri = migration_uri(
            42,
            1,
            0,
            vec![migration_parameter(
                "GitHub:alice@example.com",
                "GitHub",
                2,
                2,
                2,
            )],
        );

        let accounts = parse_document(&uri).unwrap();

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].issuer, "GitHub");
        assert_eq!(accounts[0].account, "alice@example.com");
        assert_eq!(accounts[0].algorithm, OtpAlgorithm::Sha256);
        assert_eq!(accounts[0].digits, 8);
        assert_eq!(accounts[0].period, 30);
        assert_eq!(accounts[0].secret(), b"12345678901234567890");
    }

    #[test]
    fn migration_version_two_imports_account() {
        let uri = migration_uri_with_version(
            2,
            43,
            1,
            0,
            vec![migration_parameter("G:a", "G", 1, 1, 2)],
        );

        let accounts = parse_document(&uri).unwrap();

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].label(), "G — a");
    }

    #[test]
    fn migration_infers_issuer_from_name() {
        let uri = migration_uri(
            9,
            1,
            0,
            vec![migration_parameter("Demo Issuer:Demo Account", "", 1, 1, 2)],
        );

        let accounts = parse_document(&uri).unwrap();

        assert_eq!(accounts[0].label(), "Demo Issuer — Demo Account");
    }

    #[test]
    fn migration_assembles_multiple_parts_in_index_order() {
        let part_one = migration_uri(77, 2, 1, vec![migration_parameter("Two:b", "Two", 1, 1, 2)]);
        let part_zero = migration_uri(77, 2, 0, vec![migration_parameter("One:a", "One", 1, 1, 2)]);

        let accounts = parse_document(&format!("{part_one}\n{part_zero}")).unwrap();

        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].label(), "One — a");
        assert_eq!(accounts[1].label(), "Two — b");
    }

    #[test]
    fn migration_rejects_incomplete_batch() {
        let uri = migration_uri(77, 2, 0, vec![migration_parameter("One:a", "One", 1, 1, 2)]);

        assert!(matches!(
            parse_document(&uri),
            Err(IngestError::IncompleteMigrationBatch { .. })
        ));
    }

    #[test]
    fn migration_rejects_hotp_entry() {
        let uri = migration_uri(77, 1, 0, vec![migration_parameter("One:a", "One", 1, 1, 1)]);

        assert!(matches!(
            parse_document(&uri),
            Err(IngestError::HotpUnsupported)
        ));
    }

    fn migration_parameter(
        name: &str,
        issuer: &str,
        algorithm: i32,
        digits: i32,
        otp_type: i32,
    ) -> MigrationOtpParameters {
        MigrationOtpParameters {
            secret: b"12345678901234567890".to_vec(),
            name: name.to_owned(),
            issuer: issuer.to_owned(),
            algorithm,
            digits,
            otp_type,
            counter: 0,
        }
    }

    fn migration_uri(
        batch_id: i32,
        batch_size: i32,
        batch_index: i32,
        otp_parameters: Vec<MigrationOtpParameters>,
    ) -> String {
        migration_uri_with_version(1, batch_id, batch_size, batch_index, otp_parameters)
    }

    fn migration_uri_with_version(
        version: i32,
        batch_id: i32,
        batch_size: i32,
        batch_index: i32,
        otp_parameters: Vec<MigrationOtpParameters>,
    ) -> String {
        let payload = MigrationPayload {
            otp_parameters,
            version,
            batch_size,
            batch_index,
            batch_id,
        };
        let encoded = STANDARD.encode(payload.encode_to_vec());
        let encoded = encoded
            .replace('+', "%2B")
            .replace('/', "%2F")
            .replace('=', "%3D");

        format!("{MIGRATION_PREFIX}data={encoded}")
    }
}
