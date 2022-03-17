use crate::errors::consensus::{AbstractConsensusError, ConsensusError};

pub struct ValidationResult {
    errors: Vec<ConsensusError>,
    // TODO: data can be anything, figure out what to do with it
    data: Option<ConsensusError>
}

impl ValidationResult {
    pub fn new(errors: Option<Vec<ConsensusError>>) -> Self {
        Self {
            errors: errors.unwrap_or_else(|| Vec::new()),
            data: None
        }
    }

    pub fn add_error(&mut self, ) {

    }

    pub fn add_errors(&mut self, mut errors: Vec<ConsensusError>) {
        self.errors.append(&mut errors)
    }

    pub fn errors(&self) -> &Vec<ConsensusError> {
        &self.errors
    }
}