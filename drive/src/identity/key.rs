use std::io::{BufReader, Read};

use byteorder::{BigEndian, ReadBytesExt};
use ciborium::value::Value;
use serde::{Deserialize, Serialize};

use crate::common;
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct IdentityKey {
    pub id: u16,
    pub key_type: u8,
    pub purpose: u8,
    pub security_level: u8,
    pub readonly: bool,
    pub public_key_bytes: Vec<u8>,
}

impl IdentityKey {
    pub fn serialize(&self) -> Vec<u8> {
        let IdentityKey {
            id,
            key_type,
            public_key_bytes,
            purpose,
            security_level,
            readonly,
        } = self;
        let mut buffer: Vec<u8> = id.to_be_bytes().to_vec();
        buffer.push(*key_type);
        buffer.push(*purpose);
        buffer.push(*security_level);
        buffer.push(u8::from(*readonly));
        buffer.extend(public_key_bytes);
        buffer
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let mut buf = BufReader::new(bytes);
        if bytes.len() < 38 {
            return Err(Error::Drive(DriveError::CorruptedSerialization(
                "serialized identity is too small, must have id and owner id",
            )));
        }
        let id = buf.read_u16::<BigEndian>().map_err(|_| {
            Error::Drive(DriveError::CorruptedSerialization(
                "error reading from serialized document",
            ))
        })?;
        let key_type = buf.read_u8().map_err(|_| {
            Error::Drive(DriveError::CorruptedSerialization(
                "error reading from serialized document",
            ))
        })?;
        let purpose = buf.read_u8().map_err(|_| {
            Error::Drive(DriveError::CorruptedSerialization(
                "error reading from serialized document",
            ))
        })?;
        let security_level = buf.read_u8().map_err(|_| {
            Error::Drive(DriveError::CorruptedSerialization(
                "error reading from serialized document",
            ))
        })?;

        let readonly = buf.read_u8().map_err(|_| {
            Error::Drive(DriveError::CorruptedSerialization(
                "error reading from serialized document",
            ))
        })? != 0;

        let mut public_key_bytes = vec![];
        buf.read_to_end(&mut public_key_bytes).map_err(|_| {
            Error::Drive(DriveError::CorruptedSerialization(
                "error reading from serialized document",
            ))
        })?;
        Ok(IdentityKey {
            id,
            key_type,
            purpose,
            security_level,
            readonly,
            public_key_bytes,
        })
    }

    pub fn from_cbor_value(key_value_map: &[(Value, Value)]) -> Result<Self, Error> {
        let id = match common::cbor_inner_u16_value(key_value_map, "id") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::Identity(IdentityError::IdentityKeyMissingField(
                    "a key must have an id",
                )))
            }
        };

        let key_type = match common::cbor_inner_u8_value(key_value_map, "type") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::Identity(IdentityError::IdentityKeyMissingField(
                    "a key must have a type",
                )))
            }
        };

        let purpose = match common::cbor_inner_u8_value(key_value_map, "purpose") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::Identity(IdentityError::IdentityKeyMissingField(
                    "a key must have a purpose",
                )))
            }
        };

        let security_level = match common::cbor_inner_u8_value(key_value_map, "securityLevel") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::Identity(IdentityError::IdentityKeyMissingField(
                    "a key must have a securityLevel",
                )))
            }
        };

        let readonly = match common::cbor_inner_bool_value(key_value_map, "readOnly") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::Identity(IdentityError::IdentityKeyMissingField(
                    "a key must have a readOnly value",
                )))
            }
        };

        let public_key_bytes = match common::cbor_inner_bytes_value(key_value_map, "data") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::Identity(IdentityError::IdentityKeyMissingField(
                    "a key must have a data value",
                )))
            }
        };

        Ok(IdentityKey {
            id,
            key_type,
            public_key_bytes,
            purpose,
            security_level,
            readonly,
        })
    }
}
