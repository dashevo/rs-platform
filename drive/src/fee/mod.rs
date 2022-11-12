// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Fees Mod File.
//!

use costs::storage_cost::removal::StorageRemovedBytes::{
    BasicStorageRemoval, NoStorageRemoval, SectionedStorageRemoval,
};
use costs::storage_cost::removal::{Identifier, StorageRemovedBytes};
use enum_map::EnumMap;
use intmap::IntMap;
use std::collections::BTreeMap;
use std::ops::AddAssign;

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::op::{BaseOp, DriveCost, DriveOperation};
use crate::fee_pools::epochs::Epoch;

/// Default costs module
pub mod default_costs;
/// Op module
pub mod op;

/// Fee Result
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct FeeResult {
    /// Storage fee
    pub storage_fee: u64,
    /// Processing fee
    pub processing_fee: u64,
    /// Removed bytes from identities
    pub removed_from_identities: BTreeMap<Identifier, IntMap<u32>>,
}

/// Calculates fees for the given operations. Returns the storage and processing costs.
pub fn calculate_fee(
    base_operations: Option<EnumMap<BaseOp, u64>>,
    drive_operations: Option<Vec<DriveOperation>>,
    epoch: &Epoch,
) -> Result<FeeResult, Error> {
    let mut fee_result = FeeResult::default();
    if let Some(base_operations) = base_operations {
        for (base_op, count) in base_operations.iter() {
            match base_op.cost().checked_mul(*count) {
                None => return Err(Error::Fee(FeeError::Overflow("overflow error"))),
                Some(cost) => match processing_cost.checked_add(cost) {
                    None => return Err(Error::Fee(FeeError::Overflow("overflow error"))),
                    Some(value) => processing_cost = value,
                },
            }
        }
    }

    if let Some(drive_operations) = drive_operations {
        // println!("{:#?}", drive_operations);
        for drive_operation in DriveOperation::consume_to_fees(drive_operations)? {
            match processing_cost.checked_add(drive_operation.ephemeral_cost(epoch)?) {
                None => return Err(Error::Fee(FeeError::Overflow("overflow error"))),
                Some(value) => processing_cost = value,
            }

            match storage_cost.checked_add(drive_operation.storage_cost(epoch)?) {
                None => return Err(Error::Fee(FeeError::Overflow("overflow error"))),
                Some(value) => storage_cost = value,
            }

            storage_removed_bytes += drive_operation.storage_cost.removed_bytes;
        }
    }

    let removed_from_identities = match storage_removed_bytes {
        NoStorageRemoval => BTreeMap::default(),
        BasicStorageRemoval(_) => {
            // this is not always considered an error
            BTreeMap::default()
        }
        SectionedStorageRemoval(s) => s,
    };

    let fee_result = FeeResult {
        storage_fee: storage_cost,
        processing_fee: processing_cost,
        removed_from_identities,
    };

    Ok(fee_result)
}

impl FeeResult {
    fn checked_add(&self, rhs: Self) -> Result<Self, Error> {
        let storage_fee = self.storage_fee.checked_add(rhs.storage_fee).ok_or(Error::Fee(FeeError::Overflow("storage fee overflow error")))?;
        let processing_fee = self.processing_fee.checked_add(rhs.processing_fee).ok_or(Error::Fee(FeeError::Overflow("processing fee overflow error")))?;
        Ok(FeeResult {
            storage_fee,
            processing_fee,
            removed_from_identities: Default::default()
        })
    }
}

impl AddAssign for FeeResult{
    fn add_assign(&mut self, rhs: Self) {
        self.storage_fee += rhs.storage_fee;
        self.processing_fee += rhs.processing_fee;
        self.removed_from_identities
    }
}
