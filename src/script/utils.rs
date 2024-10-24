use crate::script::ScriptParameterType;
use std::collections::HashMap;

pub(crate) trait ParameterSubstitution {
    fn substitute_parameters(
        &self,
        parameters: &HashMap<String, ScriptParameterType>,
        optional: bool,
    ) -> Result<Option<String>, String>;
}

impl ParameterSubstitution for String {
    fn substitute_parameters(
        &self,
        parameters: &HashMap<String, ScriptParameterType>,
        optional: bool,
    ) -> Result<Option<String>, String> {
        let mut result = self.clone();

        // Find all occurrences of $(xxx.yyy)
        while let Some(start) = result.find("$(") {
            let remaining = &result[start..];
            let end = remaining
                .find(')')
                .ok_or_else(|| "Missing closing bracket ')'".to_string())?;

            // Check if parameter is optional and result only contains the parameter reference
            if optional && end == remaining.len() - 1 {
                return Ok(None);
            }

            // Extract the full parameter reference including $() brackets
            let full_param_ref = &remaining[..=end];
            // Extract just the parameter name (without $( and ))
            let param_name = &remaining[2..end];

            let param_value = parameters
                .get(param_name)
                .ok_or_else(|| format!("Parameter not found: {}", param_name))?;

            let value = match param_value {
                ScriptParameterType::String(s) => s.clone(),
                ScriptParameterType::Credential(c) => c.clone(),
                ScriptParameterType::Password(p) => p.clone(),
                ScriptParameterType::Boolean(b) => b.to_string(),
                ScriptParameterType::Number(n) => n.to_string(),
            };

            result = result.replace(full_param_ref, &value);
        }

        Ok(Some(result))
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

        // Test different prefixes
        let input = "$(parameters.param1)".to_string();
        assert_eq!(
            input.substitute_parameters(&parameters, false).unwrap().unwrap(),
            "value1"
        );

        let input = "$(steps.step1.value)".to_string();
        assert_eq!(
            input.substitute_parameters(&parameters, false).unwrap().unwrap(),
            "step_value"
        );

        let input = "$(env.VERSION)".to_string();
        assert_eq!(
            input.substitute_parameters(&parameters, false).unwrap().unwrap(),
            "1.0.0"
        );

        // Test with constants
        let input = "prefix_$(env.VERSION)_suffix".to_string();
        assert_eq!(
            input.substitute_parameters(&parameters, false).unwrap().unwrap(),
            "prefix_1.0.0_suffix"
        );

        // Test multiple parameters with different prefixes
        let input = "$(parameters.param1)_$(env.VERSION)".to_string();
        assert_eq!(
            input.substitute_parameters(&parameters, false).unwrap().unwrap(),
            "value1_1.0.0"
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
