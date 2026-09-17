pub mod audit;
pub mod chat;
pub mod knowledge;
pub mod learning;
pub mod quiz;
pub mod relation;
pub mod settings;
pub mod source;

/// Collect a `query_map` iterator, surfacing the first row-mapping error.
///
/// The previous pattern was `filter_map(|r| r.ok())`. A schema drift, a
/// corrupted JSON column, or a type mismatch then shrank the returned list
/// with no log and no 500 — the caller thought it had the full table. Mapping
/// failures are now internal errors, same as a failed `prepare`.
pub(crate) fn rows<T, I>(mapped: rusqlite::Result<I>) -> Result<Vec<T>, String>
where
    I: Iterator<Item = rusqlite::Result<T>>,
{
    mapped
        .map_err(crate::error::internal)?
        .collect::<rusqlite::Result<Vec<T>>>()
        .map_err(crate::error::internal)
}

/// Same as [`rows`], for the single-row getters that used to turn a mapping
/// failure into `None` (which the caller then treated as "not found").
pub(crate) fn one<T>(
    mut iter: impl Iterator<Item = rusqlite::Result<T>>,
) -> Result<Option<T>, String> {
    match iter.next() {
        None => Ok(None),
        Some(Ok(value)) => Ok(Some(value)),
        Some(Err(err)) => Err(crate::error::internal(err)),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn rows_keeps_every_ok_value() {
        let iter = [Ok(1), Ok(2), Ok(3)].into_iter();
        assert_eq!(super::rows(Ok(iter)).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn rows_propagates_the_first_error() {
        let iter = [Ok(1), Err(rusqlite::Error::InvalidQuery), Ok(3)].into_iter();
        let err = super::rows(Ok(iter)).unwrap_err();
        assert!(
            crate::error::is_internal(&err),
            "a mapping failure must be classified internal, not a user-facing 400"
        );
        let detail = crate::error::internal_detail(&err);
        assert!(
            !detail.is_empty(),
            "the rusqlite detail is kept for the audit log"
        );
    }

    #[test]
    fn rows_does_not_return_the_prefix_of_a_failed_scan() {
        let iter = [Ok("kept"), Err(rusqlite::Error::InvalidQuery)].into_iter();
        assert!(super::rows(Ok(iter)).is_err());
    }

    #[test]
    fn one_distinguishes_missing_from_corrupt() {
        let missing: [rusqlite::Result<i32>; 0] = [];
        assert_eq!(super::one(missing.into_iter()).unwrap(), None::<i32>);

        let ok = [Ok(7)].into_iter();
        assert_eq!(super::one(ok).unwrap(), Some(7));

        let bad: [rusqlite::Result<i32>; 1] = [Err(rusqlite::Error::InvalidQuery)];
        assert!(crate::error::is_internal(&super::one(bad.into_iter()).unwrap_err()));
    }
}
