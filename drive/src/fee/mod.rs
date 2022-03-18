use crate::fee::op::{InsertOperation, BaseOp, QueryOperation, DeleteOperation};
use enum_map::EnumMap;
use grovedb::Error;

pub mod op;

pub fn calculate_fee(
    base_operations: Option<EnumMap<BaseOp, u64>>,
    query_operations: Option<Vec<QueryOperation>>,
    insert_operations: Option<Vec<InsertOperation>>,
    delete_operations: Option<Vec<DeleteOperation>>,
) -> Result<(u64, u64), Error> {
    let mut storage_cost = 0u64;
    let mut cpu_cost = 0u64;
    if let Some(base_operations) = base_operations {
        for (base_op, count) in base_operations.iter() {
            match base_op.cost().checked_mul(*count) {
                // Todo: This should be made into an overflow error
                None => { return Err(Error::InternalError("overflow error")) }
                Some(cost) => {
                    match cpu_cost.checked_add(cost) {
                        None => { return Err(Error::InternalError("overflow error")) }
                        Some(value) => { cpu_cost = value}
                    }
                }
            }
        }
    }
    if let Some(query_operations) = query_operations {
        for query_operation in query_operations {
            match cpu_cost.checked_add(query_operation.cpu_cost()) {
                None => { return Err(Error::InternalError("overflow error")) }
                Some(value) => { cpu_cost = value}
            }
        }
    }

    if let Some(insert_operations) = insert_operations {
        for insert_operation in insert_operations {
            match cpu_cost.checked_add(insert_operation.cpu_cost()) {
                None => { return Err(Error::InternalError("overflow error")) }
                Some(value) => { cpu_cost = value}
            }

            match storage_cost.checked_add(insert_operation.cpu_cost()) {
                None => { return Err(Error::InternalError("overflow error")) }
                Some(value) => { cpu_cost = value}
            }
        }
    }

    Ok((storage_cost, cpu_cost))
}
