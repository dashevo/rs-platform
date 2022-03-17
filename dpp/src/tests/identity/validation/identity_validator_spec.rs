use jsonschema::error::{TypeKind, ValidationErrorKind};
use jsonschema::paths::PathChunk;
use jsonschema::primitive_type::PrimitiveType::Integer;
use crate::errors::consensus::ConsensusError;
use crate::identity::validation::IdentityValidator;

#[test]
pub fn balance_should_be_an_integer() {
    let mut identity = crate::tests::fixtures::identity_fixture_json();
    //identity.balance = 1.2;

    let identity_validator = IdentityValidator::new().unwrap();

    let result = identity_validator.validate_identity(&identity);

    // expectJsonSchemaError(result);

    let error = result.errors().first().expect("Expected to be at least one validation error");

    match error {
        ConsensusError::JsonSchemaError(error) => {
            //assert_eq!(error.to_string(), "something");
            //let expected_kind = ValidationErrorKind::Type { kind: TypeKind::Single(Integer) };
            //assert_eq!(error.kind(), &expected_kind);
            let keyword = error.keyword().expect("Expected to have a keyword");

            assert_eq!(keyword, "type");
            assert_eq!(error.instance_path().to_string(), "/balance");
        }
        _ => panic!("Expected JsonSchemaError")
    }

    // expect(error.getKeyword()).to.equal('type');
    // expect(error.getInstancePath()).to.equal('/balance');

    //expect(error.getInstancePath()).to.equal('');
    // expect(error.getParams().missingProperty).to.equal('id');
    // expect(error.getKeyword()).to.equal('required');
}