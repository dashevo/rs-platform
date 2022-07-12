use crate::drive::Drive;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee_pools::epochs::EpochPool;
use grovedb::{Element, TransactionArg};

use crate::fee_pools::epochs::tree_key_constants;

impl Drive {
    pub fn get_epoch_start_time(
        &self,
        epoch_pool: &EpochPool,
        transaction: TransactionArg,
    ) -> Result<u64, Error> {
        let element = self
            .grove
            .get(
                epoch_pool.get_path(),
                tree_key_constants::KEY_START_TIME.as_slice(),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        if let Element::Item(item, _) = element {
            Ok(u64::from_be_bytes(item.as_slice().try_into().map_err(
                |_| Error::Fee(FeeError::CorruptedStartTimeLength()),
            )?))
        } else {
            Err(Error::Fee(FeeError::CorruptedStartTimeNotItem()))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::common::tests::helpers::setup::{setup_drive, setup_fee_pools};
    use crate::drive::batch::GroveDbOpBatch;
    use chrono::Utc;
    use grovedb::Element;
    use rust_decimal_macros::dec;

    use crate::error;
    use crate::error::fee::FeeError;

    use super::EpochPool;

    #[test]
    fn test_update_start_time() {
        let drive = setup_drive();

        let (transaction, _) = setup_fee_pools(&drive, None);

        let epoch_pool = super::EpochPool::new(0);

        let start_time_ms: u64 = Utc::now().timestamp_millis() as u64;

        let mut batch = GroveDbOpBatch::new();

        batch.push(epoch_pool.update_start_time_operation(start_time_ms));

        drive
            .grove_apply_batch(batch, false, Some(&transaction))
            .expect("should apply batch");

        let actual_start_time_ms = drive
            .get_epoch_start_time(&epoch_pool, Some(&transaction))
            .expect("should get start time");

        assert_eq!(start_time_ms, actual_start_time_ms);
    }

    mod get_start_time {
        use crate::fee_pools::epochs::tree_key_constants::KEY_START_TIME;

        #[test]
        fn test_error_if_epoch_pool_is_not_initiated() {
            let drive = super::setup_drive();

            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let non_initiated_epoch_pool = super::EpochPool::new(7000);

            match drive.get_epoch_start_time(&non_initiated_epoch_pool, Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to get start time on uninit epochs pool"
                ),
                Err(e) => match e {
                    super::error::Error::GroveDB(grovedb::Error::PathNotFound(_)) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_value_is_not_set() {
            let drive = super::setup_drive();

            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let epoch_pool = super::EpochPool::new(0);

            match drive.get_epoch_start_time(&epoch_pool, Some(&transaction)) {
                Ok(_) => assert!(false, "must be an error"),
                Err(e) => match e {
                    super::error::Error::GroveDB(_) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_element_has_invalid_type() {
            let drive = super::setup_drive();

            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let epoch = super::EpochPool::new(0);

            drive
                .grove
                .insert(
                    epoch.get_path(),
                    KEY_START_TIME.as_slice(),
                    super::Element::empty_tree(),
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            match drive.get_epoch_start_time(&epoch, Some(&transaction)) {
                Ok(_) => assert!(false, "must be an error"),
                Err(e) => match e {
                    super::error::Error::Fee(super::FeeError::CorruptedStartTimeNotItem()) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_value_has_invalid_length() {
            let drive = super::setup_drive();

            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let epoch_pool = super::EpochPool::new(0);

            drive
                .grove
                .insert(
                    epoch_pool.get_path(),
                    KEY_START_TIME.as_slice(),
                    super::Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            match drive.get_epoch_start_time(&epoch_pool, Some(&transaction)) {
                Ok(_) => assert!(false, "must be an error"),
                Err(e) => match e {
                    super::error::Error::Fee(super::FeeError::CorruptedStartTimeLength()) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }
    }
}
