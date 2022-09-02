use anyhow::anyhow;
use lazy_static::lazy_static;
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;

use crate::{
    identity::validation::TPublicKeysValidator,
    util::json_value::JsonValueExt,
    validation::{JsonSchemaValidator, SimpleValidationResult},
    version::ProtocolVersionValidator,
    NonConsensusError, ProtocolError,
};

use super::identity_update_transition::property_names;

lazy_static! {
    static ref IDENTITY_UPDATE_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "./../../../schema/identity/stateTransition/identityUpdate.json"
    ))
    .expect("Identity Update Schema file should exist");
}

pub struct ValidateIdentityUpdateTransitionBasic<T> {
    protocol_version_validator: Arc<ProtocolVersionValidator>,
    json_schema_validator: JsonSchemaValidator,
    public_keys_validator: Arc<T>,
}

impl<T: TPublicKeysValidator> ValidateIdentityUpdateTransitionBasic<T> {
    pub fn new(
        protocol_version_validator: Arc<ProtocolVersionValidator>,
        public_keys_validator: Arc<T>,
    ) -> Result<Self, ProtocolError> {
        let json_schema_validator = JsonSchemaValidator::new(IDENTITY_UPDATE_SCHEMA.clone())
            .map_err(|e| {
                anyhow!(
                    "creating schema validator for Identity Update failed: {}",
                    e
                )
            })?;
        Ok(Self {
            protocol_version_validator,
            public_keys_validator,
            json_schema_validator,
        })
    }

    pub fn validate(
        &self,
        raw_state_transition: &JsonValue,
    ) -> Result<SimpleValidationResult, NonConsensusError> {
        let result = self.json_schema_validator.validate(raw_state_transition)?;
        if !result.is_valid() {
            return Ok(result);
        }

        let protocol_version = raw_state_transition
            .get_u64(property_names::PROTOCOL_VERSION)
            .map_err(|e| NonConsensusError::SerdeJsonError(e.to_string()))?;

        let result = self
            .protocol_version_validator
            .validate(protocol_version as u32)?;
        if !result.is_valid() {
            return Ok(result);
        }

        let maybe_raw_public_keys = raw_state_transition.get(property_names::ADD_PUBLIC_KEYS);
        match maybe_raw_public_keys {
            Some(raw_public_keys) => {
                let raw_public_keys_list = raw_public_keys.as_array().ok_or_else(|| {
                    NonConsensusError::SerdeJsonError(format!(
                        "'{}' property isn't an array",
                        property_names::ADD_PUBLIC_KEYS
                    ))
                })?;
                self.public_keys_validator
                    .validate_keys(raw_public_keys_list)
            }
            None => Ok(result),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        consensus::{basic::TestConsensusError, ConsensusError},
        identity::{
            state_transition::identity_update_transition::identity_update_transition::{
                property_names::{self, IDENTITY_ID},
                IdentityUpdateTransition,
            },
            validation::MockTPublicKeysValidator,
            KeyType, Purpose, SecurityLevel,
        },
        prelude::IdentityPublicKey,
        state_transition::{StateTransitionConvert, StateTransitionIdentitySigned},
        tests::{
            fixtures::{
                get_identity_update_transition_fixture, get_protocol_version_validator_fixture,
            },
            utils::get_schema_error,
        },
        util::json_value::JsonValueExt,
        validation::SimpleValidationResult,
        version::ProtocolVersionValidator,
        NonConsensusError,
    };
    use jsonschema::error::ValidationErrorKind;
    use serde_json::{json, Value as JsonValue};
    use std::{convert::TryInto, sync::Arc};
    use test_case::test_case;

    use super::ValidateIdentityUpdateTransitionBasic;

    struct TestData {
        protocol_version_validator: ProtocolVersionValidator,
        validate_public_keys_mock: MockTPublicKeysValidator,
        ec_public_key: [u8; 33],
        ec_private_key: [u8; 32],
        identity_public_key: IdentityPublicKey,
        state_transition: IdentityUpdateTransition,
        raw_state_transition: JsonValue,
        raw_public_key_to_add: JsonValue,
    }

    fn setup_test() -> TestData {
        let protocol_version_validator = get_protocol_version_validator_fixture();
        let validate_public_keys_mock = MockTPublicKeysValidator::new();
        let mut state_transition = get_identity_update_transition_fixture();

        let secp = dashcore::secp256k1::Secp256k1::new();
        let mut rng = dashcore::secp256k1::rand::thread_rng();
        let (private_key, public_key) = secp.generate_keypair(&mut rng);
        let ec_private_key = private_key.secret_bytes();
        let ec_public_key = public_key.serialize();

        let identity_public_key = IdentityPublicKey {
            id: 1,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            data: ec_public_key.try_into().unwrap(),
            read_only: false,
            disabled_at: None,
        };

        state_transition
            .sign(&identity_public_key, &ec_private_key)
            .expect("transition should be singed");
        let raw_state_transition = state_transition.to_object(false).unwrap();

        let raw_public_key_to_add = json!({
            "id": 0,
            "type": KeyType::ECDSA_SECP256K1,
            "data":  base64::decode("AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di").unwrap(),
            "purpose": Purpose::AUTHENTICATION,
            "securityLevel": SecurityLevel::MASTER,
            "readOnly": false,
        });

        TestData {
            protocol_version_validator,
            validate_public_keys_mock,
            ec_public_key,
            ec_private_key,
            identity_public_key,
            state_transition,
            raw_state_transition,
            raw_public_key_to_add,
        }
    }

    #[test_case(property_names::PROTOCOL_VERSION)]
    #[test_case(property_names::TYPE)]
    #[test_case(property_names::SIGNATURE)]
    #[test_case(property_names::REVISION)]
    #[test_case(property_names::IDENTITY_ID)]
    fn property_should_be_present(property: &str) {
        let TestData {
            protocol_version_validator,
            validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        raw_state_transition.remove(property).unwrap();

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");
        let schema_error = get_schema_error(&result, 0);

        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Required {
                property: JsonValue::String(missing_property)
            } if missing_property == property
        ));
    }

    #[test_case(property_names::IDENTITY_ID)]
    #[test_case(property_names::SIGNATURE)]
    fn property_should_be_byte_array(property_name: &str) {
        let TestData {
            protocol_version_validator,
            validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        let array = ["string"; 32];
        raw_state_transition[property_name] = json!(array);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        let byte_array_schema_error = get_schema_error(&result, 1);
        assert_eq!(
            format!("/{}/0", property_name),
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("type"), schema_error.keyword(),);
        assert_eq!(
            format!("/properties/{}/byteArray/items/type", property_name),
            byte_array_schema_error.schema_path().to_string()
        );
    }

    #[test_case(property_names::PROTOCOL_VERSION)]
    #[test_case(property_names::REVISION)]
    #[test_case(property_names::PUBLIC_KEYS_DISABLED_AT)]
    fn property_should_be_integer(property_name: &str) {
        let TestData {
            protocol_version_validator,
            validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        raw_state_transition[property_name] = json!("1");

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            format!("/{}", property_name),
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test_case(property_names::IDENTITY_ID, 32)]
    #[test_case(property_names::SIGNATURE, 65)]
    fn signature_should_be_not_less_than_n_bytes(property_name: &str, n_bytes: usize) {
        let TestData {
            protocol_version_validator,
            validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        let array = vec![0u8; n_bytes - 1];
        raw_state_transition[property_name] = json!(array);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            format!("/{}", property_name),
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("minItems"), schema_error.keyword(),);
    }

    #[test_case(property_names::IDENTITY_ID, 32)]
    #[test_case(property_names::SIGNATURE, 65)]
    fn signature_should_be_not_longer_than_n_bytes(property_name: &str, n_bytes: usize) {
        let TestData {
            protocol_version_validator,
            validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        let array = vec![0u8; n_bytes + 1];
        raw_state_transition[property_name] = json!(array);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            format!("/{}", property_name),
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("maxItems"), schema_error.keyword(),);
    }

    #[test]
    fn protocol_version_should_be_valid() {
        let TestData {
            protocol_version_validator,
            validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        raw_state_transition[property_names::PROTOCOL_VERSION] = json!(-1);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect_err("error should be returned");

        assert!(matches!(result, NonConsensusError::SerdeJsonError(_)));
    }

    #[test]
    fn raw_state_transition_type_should_be_valid() {
        let TestData {
            protocol_version_validator,
            validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        raw_state_transition[property_names::TYPE] = json!(666);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            format!("/{}", property_names::TYPE),
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("const"), schema_error.keyword());
    }

    #[test]
    fn revision_should_be_greater_or_equal_0() {
        let TestData {
            protocol_version_validator,
            validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        raw_state_transition[property_names::REVISION] = json!(-1);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            format!("/{}", property_names::REVISION),
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("minimum"), schema_error.keyword());
    }

    #[test]
    fn add_public_keys_should_return_valid_result() {
        let TestData {
            protocol_version_validator,
            mut validate_public_keys_mock,
            mut raw_state_transition,
            raw_public_key_to_add,
            ..
        } = setup_test();

        validate_public_keys_mock
            .expect_validate_keys()
            .returning(|_| Ok(Default::default()));

        let _ = raw_state_transition.remove(property_names::DISABLE_PUBLIC_KEYS);
        let _ = raw_state_transition.remove(property_names::PUBLIC_KEYS_DISABLED_AT);
        raw_state_transition[property_names::ADD_PUBLIC_KEYS] = json!(vec![raw_public_key_to_add]);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        assert!(result.is_valid());
    }

    #[test]
    fn add_public_keys_should_not_be_empty() {
        let TestData {
            protocol_version_validator,
            mut validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        validate_public_keys_mock
            .expect_validate_keys()
            .returning(|_| Ok(Default::default()));

        let _ = raw_state_transition.remove(property_names::DISABLE_PUBLIC_KEYS);
        let _ = raw_state_transition.remove(property_names::PUBLIC_KEYS_DISABLED_AT);
        raw_state_transition[property_names::ADD_PUBLIC_KEYS] = json!([]);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            format!("/{}", property_names::ADD_PUBLIC_KEYS),
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("minItems"), schema_error.keyword(),);
    }

    #[test]
    fn add_public_keys_should_not_have_more_than_10_items() {
        let TestData {
            protocol_version_validator,
            mut validate_public_keys_mock,
            mut raw_state_transition,
            raw_public_key_to_add,
            ..
        } = setup_test();

        validate_public_keys_mock
            .expect_validate_keys()
            .returning(|_| Ok(Default::default()));

        let _ = raw_state_transition.remove(property_names::DISABLE_PUBLIC_KEYS);
        let _ = raw_state_transition.remove(property_names::PUBLIC_KEYS_DISABLED_AT);
        let public_keys_to_add: Vec<JsonValue> =
            (0..11).map(|_| raw_public_key_to_add.clone()).collect();
        raw_state_transition[property_names::ADD_PUBLIC_KEYS] = json!(public_keys_to_add);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            format!("/{}", property_names::ADD_PUBLIC_KEYS),
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("maxItems"), schema_error.keyword(),);
    }

    #[test]
    fn add_public_keys_should_be_unique() {
        let TestData {
            protocol_version_validator,
            mut validate_public_keys_mock,
            mut raw_state_transition,
            raw_public_key_to_add,
            ..
        } = setup_test();

        validate_public_keys_mock
            .expect_validate_keys()
            .returning(|_| Ok(Default::default()));

        let _ = raw_state_transition.remove(property_names::DISABLE_PUBLIC_KEYS);
        let _ = raw_state_transition.remove(property_names::PUBLIC_KEYS_DISABLED_AT);
        let public_keys_to_add: Vec<JsonValue> =
            (0..2).map(|_| raw_public_key_to_add.clone()).collect();
        raw_state_transition[property_names::ADD_PUBLIC_KEYS] = json!(public_keys_to_add);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            format!("/{}", property_names::ADD_PUBLIC_KEYS),
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("uniqueItems"), schema_error.keyword(),);
    }

    #[test]
    fn add_public_keys_should_be_valid() {
        let TestData {
            protocol_version_validator,
            mut validate_public_keys_mock,
            mut raw_state_transition,
            raw_public_key_to_add,
            ..
        } = setup_test();

        validate_public_keys_mock
            .expect_validate_keys()
            .return_once(|_| {
                Ok(SimpleValidationResult::new(Some(vec![
                    ConsensusError::TestConsensusError(TestConsensusError::new("test")),
                ])))
            });

        let _ = raw_state_transition.remove(property_names::DISABLE_PUBLIC_KEYS);
        let _ = raw_state_transition.remove(property_names::PUBLIC_KEYS_DISABLED_AT);
        raw_state_transition[property_names::ADD_PUBLIC_KEYS] = json!([raw_public_key_to_add]);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        assert!(matches!(
            result.errors[0],
            ConsensusError::TestConsensusError(_)
        ))
    }

    #[test]
    fn disable_public_keys_should_be_used_only_with_public_keys_disabled_at() {
        let TestData {
            protocol_version_validator,
            mut validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        validate_public_keys_mock
            .expect_validate_keys()
            .returning(|_| Ok(Default::default()));

        let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
        let _ = raw_state_transition.remove(property_names::PUBLIC_KEYS_DISABLED_AT);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Required {
                property: JsonValue::String(missing_property)
            } if missing_property == property_names::PUBLIC_KEYS_DISABLED_AT
        ));
        assert_eq!(Some("dependentRequired"), schema_error.keyword(),);
    }

    #[test]
    fn disable_public_keys_should_be_valid() {
        let TestData {
            protocol_version_validator,
            mut validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        validate_public_keys_mock
            .expect_validate_keys()
            .returning(|_| Ok(Default::default()));

        let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
        raw_state_transition[property_names::DISABLE_PUBLIC_KEYS] = json!(vec![0]);
        raw_state_transition[property_names::PUBLIC_KEYS_DISABLED_AT] = json!(0);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");
        assert!(result.is_valid());
    }

    #[test]
    fn disable_public_keys_should_contain_number_greater_or_equal_0() {
        let TestData {
            protocol_version_validator,
            mut validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        validate_public_keys_mock
            .expect_validate_keys()
            .returning(|_| Ok(Default::default()));

        let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
        raw_state_transition[property_names::DISABLE_PUBLIC_KEYS] = json!(vec![-1, 0]);
        raw_state_transition[property_names::PUBLIC_KEYS_DISABLED_AT] = json!(0);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            format!("/{}/0", property_names::DISABLE_PUBLIC_KEYS),
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("minimum"), schema_error.keyword(),);
    }

    #[test]
    fn disable_public_keys_should_contain_integers() {
        let TestData {
            protocol_version_validator,
            mut validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        validate_public_keys_mock
            .expect_validate_keys()
            .returning(|_| Ok(Default::default()));

        let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
        raw_state_transition[property_names::DISABLE_PUBLIC_KEYS] = json!(vec![1.1]);
        raw_state_transition[property_names::PUBLIC_KEYS_DISABLED_AT] = json!(0);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            format!("/{}/0", property_names::DISABLE_PUBLIC_KEYS),
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("type"), schema_error.keyword(),);
    }

    #[test]
    fn disable_public_keys_should_not_have_more_than_10_items() {
        let TestData {
            protocol_version_validator,
            mut validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        validate_public_keys_mock
            .expect_validate_keys()
            .returning(|_| Ok(Default::default()));

        let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
        let key_ids_to_disable: Vec<usize> = (0..11).collect();
        raw_state_transition[property_names::DISABLE_PUBLIC_KEYS] = json!(key_ids_to_disable);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            format!("/{}", property_names::DISABLE_PUBLIC_KEYS),
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("maxItems"), schema_error.keyword(),);
    }

    #[test]
    fn disable_public_keys_should_be_unique() {
        let TestData {
            protocol_version_validator,
            mut validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        validate_public_keys_mock
            .expect_validate_keys()
            .returning(|_| Ok(Default::default()));

        let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
        let key_ids_to_disable: Vec<usize> = vec![0, 0];
        raw_state_transition[property_names::DISABLE_PUBLIC_KEYS] = json!(key_ids_to_disable);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            format!("/{}", property_names::DISABLE_PUBLIC_KEYS),
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("uniqueItems"), schema_error.keyword(),);
    }

    #[test]
    fn public_keys_disabled_at_should_be_used_only_with_disable_public_keys() {
        let TestData {
            protocol_version_validator,
            mut validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        validate_public_keys_mock
            .expect_validate_keys()
            .returning(|_| Ok(Default::default()));

        let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
        let _ = raw_state_transition.remove(property_names::DISABLE_PUBLIC_KEYS);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert!(matches!(
            schema_error.kind(),
            ValidationErrorKind::Required {
                property: JsonValue::String(missing_property)
            } if missing_property == property_names::DISABLE_PUBLIC_KEYS
        ));
        assert_eq!(Some("dependentRequired"), schema_error.keyword(),);
    }

    #[test]
    fn public_keys_disabled_at_should_be_greater_or_equal_0() {
        let TestData {
            protocol_version_validator,
            mut validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        validate_public_keys_mock
            .expect_validate_keys()
            .returning(|_| Ok(Default::default()));

        let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
        raw_state_transition[property_names::DISABLE_PUBLIC_KEYS] = json!(vec![0]);
        raw_state_transition[property_names::PUBLIC_KEYS_DISABLED_AT] = json!(-1);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!(
            format!("/{}", property_names::PUBLIC_KEYS_DISABLED_AT),
            schema_error.instance_path().to_string()
        );
        assert_eq!(Some("minimum"), schema_error.keyword(),);
    }

    #[test]
    fn public_keys_disabled_at_should_return_valid_result() {
        let TestData {
            protocol_version_validator,
            mut validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        validate_public_keys_mock
            .expect_validate_keys()
            .returning(|_| Ok(Default::default()));

        let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");
        assert!(result.is_valid());
    }

    #[test]
    fn should_return_valid_result() {
        let TestData {
            protocol_version_validator,
            mut validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        validate_public_keys_mock
            .expect_validate_keys()
            .returning(|_| Ok(Default::default()));

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");
        assert!(result.is_valid());
    }

    #[test]
    fn should_have_either_add_public_keys_or_disable_public_keys() {
        let TestData {
            protocol_version_validator,
            mut validate_public_keys_mock,
            mut raw_state_transition,
            ..
        } = setup_test();

        validate_public_keys_mock
            .expect_validate_keys()
            .returning(|_| Ok(Default::default()));

        let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
        let _ = raw_state_transition.remove(property_names::DISABLE_PUBLIC_KEYS);
        let _ = raw_state_transition.remove(property_names::PUBLIC_KEYS_DISABLED_AT);

        let validator = ValidateIdentityUpdateTransitionBasic::new(
            Arc::new(protocol_version_validator),
            Arc::new(validate_public_keys_mock),
        )
        .unwrap();

        let result = validator
            .validate(&raw_state_transition)
            .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("", schema_error.instance_path().to_string());
        assert_eq!(Some("anyOf"), schema_error.keyword(),);
    }
}
