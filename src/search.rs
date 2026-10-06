use crate::model::Account;

pub fn rank_accounts(accounts: &[Account], query: &str) -> Vec<usize> {
    let query = query.trim().to_lowercase();

    let mut ranked: Vec<(usize, i32)> = accounts
        .iter()
        .enumerate()
        .filter_map(|(index, account)| score(account, &query).map(|score| (index, score)))
        .collect();

    ranked.sort_by(|(left_index, left_score), (right_index, right_score)| {
        right_score
            .cmp(left_score)
            .then_with(|| {
                accounts[*left_index]
                    .issuer
                    .to_lowercase()
                    .cmp(&accounts[*right_index].issuer.to_lowercase())
            })
            .then_with(|| {
                accounts[*left_index]
                    .account
                    .to_lowercase()
                    .cmp(&accounts[*right_index].account.to_lowercase())
            })
    });

    ranked.into_iter().map(|(index, _)| index).collect()
}

fn score(account: &Account, query: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }

    let issuer = account.issuer.to_lowercase();
    let account_name = account.account.to_lowercase();

    let issuer_score = field_score(&issuer, query).map(|score| score + 25);
    let account_score = field_score(&account_name, query);

    issuer_score.into_iter().chain(account_score).max()
}

fn field_score(field: &str, query: &str) -> Option<i32> {
    if field == query {
        return Some(1_000);
    }
    if field.starts_with(query) {
        return Some(900);
    }
    if field
        .split(|ch: char| !ch.is_alphanumeric())
        .any(|part| part.starts_with(query))
    {
        return Some(800);
    }
    if field.contains(query) {
        return Some(650);
    }

    subsequence_score(field, query)
}

fn subsequence_score(field: &str, query: &str) -> Option<i32> {
    let mut query_chars = query.chars();
    let mut wanted = query_chars.next()?;
    let mut gaps = 0_i32;
    let mut matched = 0_i32;

    for ch in field.chars() {
        if ch == wanted {
            matched += 1;
            match query_chars.next() {
                Some(next) => wanted = next,
                None => return Some(400 + matched * 5 - gaps),
            }
        } else if matched > 0 {
            gaps += 1;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Account, OtpAlgorithm};

    fn account(issuer: &str, name: &str) -> Account {
        Account::new(
            issuer,
            name,
            vec![0; 20],
            OtpAlgorithm::Sha1,
            6,
            30,
        )
    }

    #[test]
    fn exact_and_prefix_matches_beat_contains() {
        let accounts = vec![
            account("Forge", "github-secondary"),
            account("GitLab", "me"),
            account("GitHub", "me"),
        ];

        let ranked = rank_accounts(&accounts, "git");

        assert_eq!(ranked, vec![2, 1, 0]);
    }

    #[test]
    fn empty_query_keeps_every_account() {
        let accounts = vec![account("B", "2"), account("A", "1")];

        let ranked = rank_accounts(&accounts, "");

        assert_eq!(ranked.len(), 2);
    }
}
