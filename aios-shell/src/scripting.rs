use crate::executor::Executor;
use crate::parser;
use aios_core::CommandOutput;
use std::collections::HashMap;
use std::path::Path;

pub struct ScriptInterpreter {
    pub variables: HashMap<String, String>,
}

impl ScriptInterpreter {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn execute_file(&mut self, path: &Path, executor: &mut Executor) -> Result<i32, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read script: {}", e))?;
        self.execute_string(&content, executor)
    }

    pub fn execute_string(
        &mut self,
        script: &str,
        executor: &mut Executor,
    ) -> Result<i32, String> {
        let lines: Vec<&str> = script.lines().collect();
        let mut i = 0;
        let mut last_exit = 0;

        while i < lines.len() {
            let line = lines[i].trim();
            i += 1;

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(rest) = line.strip_prefix("if ") {
                let (exit_code, skip_to) = self.handle_if(rest, &lines[i..], executor)?;
                last_exit = exit_code;
                i += skip_to;
                continue;
            }

            if let Some(rest) = line.strip_prefix("while ") {
                let (exit_code, skip_to) = self.handle_while(rest, &lines[i..], executor)?;
                last_exit = exit_code;
                i += skip_to;
                continue;
            }

            if let Some(rest) = line.strip_prefix("for ") {
                let (exit_code, skip_to) = self.handle_for(rest, &lines[i..], executor)?;
                last_exit = exit_code;
                i += skip_to;
                continue;
            }

            let expanded = self.expand_vars(line, executor);
            if let Some(pipeline) = parser::parse_pipeline(&expanded) {
                let output = executor.execute_pipeline(&pipeline);
                last_exit = output.exit_code;
            }
        }

        Ok(last_exit)
    }

    fn handle_if(
        &mut self,
        condition: &str,
        remaining: &[&str],
        executor: &mut Executor,
    ) -> Result<(i32, usize), String> {
        let condition = condition.trim().trim_end_matches("; then").trim();
        let expanded = self.expand_vars(condition, executor);

        let cond_result = if let Some(pipeline) = parser::parse_pipeline(&expanded) {
            executor.execute_pipeline(&pipeline).exit_code == 0
        } else {
            false
        };

        let mut then_lines = Vec::new();
        let mut else_lines = Vec::new();
        let mut in_else = false;
        let mut consumed = 0;

        for (i, line) in remaining.iter().enumerate() {
            let trimmed = line.trim();
            consumed = i + 1;
            if trimmed == "fi" {
                break;
            }
            if trimmed == "then" {
                continue;
            }
            if trimmed == "else" {
                in_else = true;
                continue;
            }
            if in_else {
                else_lines.push(*line);
            } else {
                then_lines.push(*line);
            }
        }

        let block = if cond_result { &then_lines } else { &else_lines };
        let script = block.join("\n");
        let exit_code = self.execute_string(&script, executor)?;

        Ok((exit_code, consumed))
    }

    fn handle_while(
        &mut self,
        condition: &str,
        remaining: &[&str],
        executor: &mut Executor,
    ) -> Result<(i32, usize), String> {
        let condition = condition.trim().trim_end_matches("; do").trim();

        let mut body_lines = Vec::new();
        let mut consumed = 0;

        for (i, line) in remaining.iter().enumerate() {
            let trimmed = line.trim();
            consumed = i + 1;
            if trimmed == "done" {
                break;
            }
            if trimmed == "do" {
                continue;
            }
            body_lines.push(*line);
        }

        let body_script = body_lines.join("\n");
        let mut last_exit = 0;
        let mut iterations = 0;
        let max_iterations = 10000;

        loop {
            let expanded_cond = self.expand_vars(condition, executor);
            let cond_result = if let Some(pipeline) = parser::parse_pipeline(&expanded_cond) {
                executor.execute_pipeline(&pipeline).exit_code == 0
            } else {
                false
            };

            if !cond_result || iterations >= max_iterations {
                break;
            }

            last_exit = self.execute_string(&body_script, executor)?;
            iterations += 1;
        }

        Ok((last_exit, consumed))
    }

    fn handle_for(
        &mut self,
        header: &str,
        remaining: &[&str],
        executor: &mut Executor,
    ) -> Result<(i32, usize), String> {
        let parts: Vec<&str> = header.splitn(3, ' ').collect();
        if parts.len() < 3 || parts[1] != "in" {
            return Err("Invalid for loop syntax".to_string());
        }

        let var_name = parts[0];
        let values_str = parts[2].trim().trim_end_matches("; do").trim();
        let values: Vec<&str> = values_str.split_whitespace().collect();

        let mut body_lines = Vec::new();
        let mut consumed = 0;

        for (i, line) in remaining.iter().enumerate() {
            let trimmed = line.trim();
            consumed = i + 1;
            if trimmed == "done" {
                break;
            }
            if trimmed == "do" {
                continue;
            }
            body_lines.push(*line);
        }

        let body_script = body_lines.join("\n");
        let mut last_exit = 0;

        for value in values {
            self.variables.insert(var_name.to_string(), value.to_string());
            executor
                .env_vars
                .insert(var_name.to_string(), value.to_string());
            last_exit = self.execute_string(&body_script, executor)?;
        }

        Ok((last_exit, consumed))
    }

    fn expand_vars(&self, input: &str, executor: &Executor) -> String {
        let mut merged = executor.env_vars.clone();
        for (k, v) in &self.variables {
            merged.insert(k.clone(), v.clone());
        }
        parser::expand_variables(input, &merged)
    }
}
