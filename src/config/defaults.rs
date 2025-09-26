/// Default configuration for the simplified prompt system
/// Since we now use a hardcoded prompt, this module provides default settings
pub fn get_default_analysis_focus() -> &'static str {
    "general analysis"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_analysis_focus() {
        let focus = get_default_analysis_focus();
        assert_eq!(focus, "general analysis");
    }
}
