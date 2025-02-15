use crate::header_verifier::{
    EpochVerifier, NumberVerifier, PowVerifier, TimestampVerifier, VersionVerifier,
};
use crate::{
    BlockVersionError, EpochError, NumberError, PowError, TimestampError, ALLOWED_FUTURE_BLOCKTIME,
};
use ckb_error::assert_error_eq;
use ckb_pow::PowEngine;
use ckb_test_chain_utils::{MockMedianTime, MOCK_MEDIAN_TIME_COUNT};
use ckb_types::{
    constants::BLOCK_VERSION,
    core::{EpochNumberWithFraction, HeaderBuilder},
    packed::Header,
    prelude::*,
};
use faketime::unix_time_as_millis;

fn mock_median_time_context() -> MockMedianTime {
    let now = unix_time_as_millis();
    let timestamps = (0..100).map(|_| now).collect();
    MockMedianTime::new(timestamps)
}

#[test]
pub fn test_version() {
    let header = HeaderBuilder::default()
        .version((BLOCK_VERSION + 1).pack())
        .build();
    let verifier = VersionVerifier::new(&header, BLOCK_VERSION);

    assert_error_eq!(
        verifier.verify().unwrap_err(),
        BlockVersionError {
            expected: BLOCK_VERSION,
            actual: BLOCK_VERSION + 1
        }
    );
}

#[cfg(not(disable_faketime))]
#[test]
fn test_timestamp() {
    let faketime_file = faketime::millis_tempfile(100_000).expect("create faketime file");
    faketime::enable(&faketime_file);
    let fake_block_median_time_context = mock_median_time_context();
    let parent_hash = fake_block_median_time_context.get_block_hash(99);
    let timestamp = unix_time_as_millis() + 1;
    let header = HeaderBuilder::default()
        .parent_hash(parent_hash)
        .number(100u64.pack())
        .timestamp(timestamp.pack())
        .build();
    let timestamp_verifier = TimestampVerifier::new(
        &fake_block_median_time_context,
        &header,
        MOCK_MEDIAN_TIME_COUNT,
    );

    assert!(timestamp_verifier.verify().is_ok());
}

#[cfg(not(disable_faketime))]
#[test]
fn test_timestamp_too_old() {
    let faketime_file = faketime::millis_tempfile(100_000).expect("create faketime file");
    faketime::enable(&faketime_file);
    let fake_block_median_time_context = mock_median_time_context();
    let parent_hash = fake_block_median_time_context.get_block_hash(99);

    let min = unix_time_as_millis();
    let timestamp = unix_time_as_millis() - 1;
    let header = HeaderBuilder::default()
        .number(100u64.pack())
        .parent_hash(parent_hash)
        .timestamp(timestamp.pack())
        .build();
    let timestamp_verifier = TimestampVerifier::new(
        &fake_block_median_time_context,
        &header,
        MOCK_MEDIAN_TIME_COUNT,
    );

    assert_error_eq!(
        timestamp_verifier.verify().unwrap_err(),
        TimestampError::BlockTimeTooOld {
            min,
            actual: timestamp,
        },
    );
}

#[cfg(not(disable_faketime))]
#[test]
fn test_timestamp_too_new() {
    let faketime_file = faketime::millis_tempfile(100_000).expect("create faketime file");
    faketime::enable(&faketime_file);
    let fake_block_median_time_context = mock_median_time_context();
    let parent_hash = fake_block_median_time_context.get_block_hash(99);

    let max = unix_time_as_millis() + ALLOWED_FUTURE_BLOCKTIME;
    let timestamp = max + 1;
    let header = HeaderBuilder::default()
        .number(100u64.pack())
        .parent_hash(parent_hash)
        .timestamp(timestamp.pack())
        .build();
    let timestamp_verifier = TimestampVerifier::new(
        &fake_block_median_time_context,
        &header,
        MOCK_MEDIAN_TIME_COUNT,
    );
    assert_error_eq!(
        timestamp_verifier.verify().unwrap_err(),
        TimestampError::BlockTimeTooNew {
            max,
            actual: timestamp,
        },
    );
}

#[test]
fn test_number() {
    let parent = HeaderBuilder::default().number(10u64.pack()).build();
    let header = HeaderBuilder::default().number(10u64.pack()).build();

    let verifier = NumberVerifier::new(&parent, &header);
    assert_error_eq!(
        verifier.verify().unwrap_err(),
        NumberError {
            expected: 11,
            actual: 10,
        },
    );
}

#[test]
fn test_epoch() {
    {
        let parent = HeaderBuilder::default().number(1u64.pack()).build();
        let epochs_malformed = vec![
            EpochNumberWithFraction::new_unchecked(1, 0, 0),
            EpochNumberWithFraction::new_unchecked(1, 10, 0),
            EpochNumberWithFraction::new_unchecked(1, 10, 5),
            EpochNumberWithFraction::new_unchecked(1, 10, 10),
        ];

        for epoch_malformed in epochs_malformed {
            let malformed = HeaderBuilder::default()
                .epoch(epoch_malformed.pack())
                .build();
            let result = EpochVerifier::new(&parent, &malformed).verify();
            assert!(result.is_err(), "input: {:#}", epoch_malformed);
            assert_error_eq!(
                result.unwrap_err(),
                EpochError::Malformed {
                    value: epoch_malformed
                },
            );
        }
    }
    {
        let epochs = vec![
            (
                EpochNumberWithFraction::new_unchecked(1, 5, 10),
                EpochNumberWithFraction::new_unchecked(1, 5, 10),
            ),
            (
                EpochNumberWithFraction::new_unchecked(1, 5, 10),
                EpochNumberWithFraction::new_unchecked(1, 5, 11),
            ),
            (
                EpochNumberWithFraction::new_unchecked(1, 5, 10),
                EpochNumberWithFraction::new_unchecked(2, 5, 10),
            ),
            (
                EpochNumberWithFraction::new_unchecked(1, 5, 10),
                EpochNumberWithFraction::new_unchecked(1, 6, 11),
            ),
            (
                EpochNumberWithFraction::new_unchecked(1, 5, 10),
                EpochNumberWithFraction::new_unchecked(2, 6, 10),
            ),
            (
                EpochNumberWithFraction::new_unchecked(1, 9, 10),
                EpochNumberWithFraction::new_unchecked(2, 1, 10),
            ),
            (
                EpochNumberWithFraction::new_unchecked(1, 9, 10),
                EpochNumberWithFraction::new_unchecked(3, 0, 10),
            ),
        ];

        for (epoch_parent, epoch_current) in epochs {
            let parent = HeaderBuilder::default()
                .number(1u64.pack())
                .epoch(epoch_parent.pack())
                .build();
            let current = HeaderBuilder::default().epoch(epoch_current.pack()).build();

            let result = EpochVerifier::new(&parent, &current).verify();
            assert!(
                result.is_err(),
                "current: {:#}, parent: {:#}",
                current,
                parent
            );
            assert_error_eq!(
                result.unwrap_err(),
                EpochError::NonContinuous {
                    current: epoch_current,
                    parent: epoch_parent,
                },
            );
        }
    }
    {
        let epochs = vec![
            (
                EpochNumberWithFraction::new_unchecked(1, 5, 10),
                EpochNumberWithFraction::new_unchecked(1, 6, 10),
            ),
            (
                EpochNumberWithFraction::new_unchecked(1, 9, 10),
                EpochNumberWithFraction::new_unchecked(2, 0, 10),
            ),
            (
                EpochNumberWithFraction::new_unchecked(1, 9, 10),
                EpochNumberWithFraction::new_unchecked(2, 0, 11),
            ),
        ];
        for (epoch_parent, epoch_current) in epochs {
            let parent = HeaderBuilder::default()
                .number(1u64.pack())
                .epoch(epoch_parent.pack())
                .build();
            let current = HeaderBuilder::default().epoch(epoch_current.pack()).build();

            let result = EpochVerifier::new(&parent, &current).verify();
            assert!(
                result.is_ok(),
                "current: {:#}, parent: {:#}",
                current,
                parent
            );
        }
    }
}

struct FakePowEngine;

impl PowEngine for FakePowEngine {
    fn verify(&self, _header: &Header) -> bool {
        false
    }
}

#[test]
fn test_pow_verifier() {
    let header = HeaderBuilder::default().build();
    let fake_pow_engine: &dyn PowEngine = &FakePowEngine;
    let verifier = PowVerifier::new(&header, fake_pow_engine);

    assert_error_eq!(verifier.verify().unwrap_err(), PowError::InvalidNonce);
}
