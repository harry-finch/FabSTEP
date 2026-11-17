//! Cooperative boundaries between bounded Rust/native processing operations.
use fabstep_core::FabError;

/// A native call already in progress is never interrupted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionPhase {
    Analysis,
    SourceSeparation,
    Body,
    Recognition,
    SourceValidation,
    Build,
    AuditTessellation,
    AuditComparison,
    Combine,
    ResultInspection,
    Finish,
}

/// Body indices refer to the analyzed document, starting at zero.
/// Callbacks identify cooperative boundaries and may repeat for the same phase;
/// they are not progress counters. Native phase calls are checked before/after,
/// including failures; Body also brackets successful body conversions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversionCheckpoint {
    pub phase: ConversionPhase,
    pub body_index: Option<usize>,
}

pub(crate) struct ConversionControl<'a> {
    callback: &'a mut dyn FnMut(ConversionCheckpoint) -> Result<(), FabError>,
    interruption: Option<FabError>,
}

impl<'a> ConversionControl<'a> {
    pub(crate) fn new(
        callback: &'a mut dyn FnMut(ConversionCheckpoint) -> Result<(), FabError>,
    ) -> Self {
        Self {
            callback,
            interruption: None,
        }
    }

    pub(crate) fn check(
        &mut self,
        phase: ConversionPhase,
        body_index: Option<usize>,
    ) -> Result<(), FabError> {
        let result = (self.callback)(ConversionCheckpoint { phase, body_index });
        if let Err(error) = &result {
            self.interruption = Some(error.clone());
        }
        result
    }

    pub(crate) fn run<T>(
        &mut self,
        phase: ConversionPhase,
        body_index: Option<usize>,
        work: impl FnOnce() -> Result<T, FabError>,
    ) -> Result<T, FabError> {
        self.check(phase, body_index)?;
        let result = work();
        self.check(phase, body_index)?;
        result
    }

    /// Preserve callback failures verbatim, including their lack of body context.
    /// In a search these must abort the proposal, never become rejected trials.
    pub(crate) fn interruption(&self) -> Option<FabError> {
        self.interruption.clone()
    }

    pub(crate) fn finish<T>(&self, result: Result<T, FabError>) -> Result<T, FabError> {
        match self.interruption() {
            Some(error) => Err(error),
            None => result,
        }
    }
}

pub(crate) fn controlled<T>(
    callback: &mut dyn FnMut(ConversionCheckpoint) -> Result<(), FabError>,
    work: impl FnOnce(&mut ConversionControl<'_>) -> Result<T, FabError>,
) -> Result<T, FabError> {
    let mut control = ConversionControl::new(callback);
    let result = work(&mut control);
    control.finish(result)
}
