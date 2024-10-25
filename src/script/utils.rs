use crate::script::ScriptParameterType;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum SubstitutionResult {
    Single(String),
    Multiple(Vec<String>),
}

pub(crate) trait ParameterSubstitution {
    fn substitute_parameters(
        &self,
        parameters: &HashMap<String, ScriptParameterType>,
        optional: bool,
    ) -> Result<Option<SubstitutionResult>, String>;
}

impl ParameterSubstitution for String {
    fn substitute_parameters(
        &self,
        parameters: &HashMap<String, ScriptParameterType>,
        optional: bool,
    ) -> Result<Option<SubstitutionResult>, String> {
        let mut result = self.clone();

        // Find all occurrences of $(xxx.yyy)
        while let Some(start) = result.find("$(") {
            let remaining = &result[start..];
            let end = remaining
                .find(')')
                .ok_or_else(|| "Missing closing bracket ')'".to_string())?;

            // Extract the full parameter reference including $() brackets
            let full_param_ref = &remaining[..=end];
            // Extract just the parameter name (without $( and ))
            let param_name = &remaining[2..end];

            let param_value = parameters.get(param_name);

            // Check if parameter is optional and result only contains the parameter reference
            if param_value.is_none() && optional && start == 0 && end == remaining.len() - 1 {
                return Ok(None);
            }
            let param_value = param_value.ok_or_else(|| format!("Parameter '{}' not found", param_name))?;

            // If this is a pure parameter reference (no additional text)
            if start == 0 && end == remaining.len() - 1 {
                // For StringArray, return Multiple variant directly
                if let ScriptParameterType::StringArray(arr) = param_value {
                    return Ok(Some(SubstitutionResult::Multiple(arr.clone())));
                }
            }

            // For all other cases, convert to string
            let value = match param_value {
                ScriptParameterType::String(s) => s.clone(),
                ScriptParameterType::Credential(c) => c.clone(),
                ScriptParameterType::Password(p) => p.clone(),
                ScriptParameterType::Boolean(b) => b.to_string(),
                ScriptParameterType::Number(n) => n.to_string(),
                ScriptParameterType::StringArray(a) => a.join(", "),
            };

            result = result.replace(full_param_ref, &value);
        }

        Ok(Some(SubstitutionResult::Single(result)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_substitution() {
        let mut parameters = HashMap::new();
        parameters.insert(
            "parameters.param1".to_string(),
            ScriptParameterType::String("value1".to_string()),
        );
        parameters.insert(
            "steps.step1.value".to_string(),
            ScriptParameterType::String("step_value".to_string()),
        );
        parameters.insert(
            "env.VERSION".to_string(),
            ScriptParameterType::String("1.0.0".to_string()),
        );
        parameters.insert(
            "array.param".to_string(),
            ScriptParameterType::StringArray(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
        );

        // Test string parameter
        let input = "$(parameters.param1)".to_string();
        assert_eq!(
            input.substitute_parameters(&parameters, false).unwrap().unwrap(),
            SubstitutionResult::Single("value1".to_string())
        );

        // Test array parameter
        let input = "$(array.param)".to_string();
        assert_eq!(
            input.substitute_parameters(&parameters, false).unwrap().unwrap(),
            SubstitutionResult::Multiple(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );

        // Test array parameter with additional text (should join array)
        let input = "prefix_$(array.param)_suffix".to_string();
        assert_eq!(
            input.substitute_parameters(&parameters, false).unwrap().unwrap(),
            SubstitutionResult::Single("prefix_a, b, c_suffix".to_string())
        );

        // Test multiple parameters with different prefixes
        let input = "$(parameters.param1)_$(env.VERSION)".to_string();
        assert_eq!(
            input.substitute_parameters(&parameters, false).unwrap().unwrap(),
            SubstitutionResult::Single("value1_1.0.0".to_string())
        );

        // Test optional parameter
        let input = "$(unknown.param)".to_string();
        assert_eq!(input.substitute_parameters(&parameters, true).unwrap(), None);

        // Test missing parameter
        let input = "$(missing.param)".to_string();
        assert!(input.substitute_parameters(&parameters, false).is_err());

        // Test missing closing bracket
        let input = "$(env.VERSION".to_string();
        assert!(input.substitute_parameters(&parameters, false).is_err());
    }
}
