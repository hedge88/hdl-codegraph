use crate::preprocessor::SourceMapping;

/// Merge the expanded source back into a final output.
///
/// In the current simple implementation the expander (Pass 3) already
/// produces inline-merged output, so this pass is essentially a pass-through
/// that validates the result.  Future versions may perform more sophisticated
/// splicing and position-adjustment.
pub fn merge(
    _original: &str,
    expanded: &str,
    _mappings: &[SourceMapping],
) -> String {
    expanded.to_string()
}
