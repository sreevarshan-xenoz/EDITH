use handlebars::{Handlebars, Helper, RenderContext, RenderError, HelperResult, Output, HelperDef};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use thiserror::Error;
use tokio::fs;

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Template not found: {0}")]
    NotFound(String),
    #[error("Template syntax error: {0}")]
    Syntax(String),
    #[error("Template rendering error: {0}")]
    Rendering(#[from] RenderError),
    #[error("Variable validation error: {0}")]
    Validation(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Security violation: {0}")]
    Security(String),
    #[error("Template composition error: {0}")]
    Composition(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    pub content: String,
    pub description: Option<String>,
    pub variables: Vec<TemplateVariable>,
    pub created_at: SystemTime,
    pub parent_template: Option<String>,
    pub tags: Vec<String>,
    pub usage_examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub var_type: VariableType,
    pub required: bool,
    pub default_value: Option<Value>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariableType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    pub template_dir: Option<PathBuf>,
    pub auto_reload: bool,
    pub enable_sandboxing: bool,
    pub max_template_size: usize,
    pub max_render_time_ms: u64,
    pub allowed_helpers: Vec<String>,
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            template_dir: Some(PathBuf::from("templates")),
            auto_reload: true,
            enable_sandboxing: true,
            max_template_size: 1024 * 1024, // 1MB
            max_render_time_ms: 5000, // 5 seconds
            allowed_helpers: vec![
                "upper".to_string(),
                "lower".to_string(),
                "trim".to_string(),
                "if".to_string(),
                "unless".to_string(),
                "each".to_string(),
                "with".to_string(),
                "format".to_string(),
                "default".to_string(),
                "length".to_string(),
                "join".to_string(),
                "contains".to_string(),
                "eq".to_string(),
                "gt".to_string(),
            ],
        }
    }
}

pub struct TemplateStore {
    templates: HashMap<String, Template>,
    template_dir: Option<PathBuf>,
}

impl TemplateStore {
    pub fn new(template_dir: Option<PathBuf>) -> Self {
        Self {
            templates: HashMap::new(),
            template_dir,
        }
    }

    pub fn add_template(&mut self, template: Template) {
        self.templates.insert(template.name.clone(), template);
    }

    pub fn get_template(&self, name: &str) -> Option<&Template> {
        self.templates.get(name)
    }

    pub fn list_templates(&self) -> Vec<&Template> {
        self.templates.values().collect()
    }

    pub fn remove_template(&mut self, name: &str) -> Option<Template> {
        self.templates.remove(name)
    }

    pub async fn load_from_disk(&mut self) -> Result<(), TemplateError> {
        if let Some(dir) = &self.template_dir {
            if dir.exists() {
                let mut entries = fs::read_dir(dir).await?;
                
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        let content = fs::read_to_string(&path).await?;
                        let template: Template = serde_json::from_str(&content)?;
                        self.templates.insert(template.name.clone(), template);
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn save_to_disk(&self, template: &Template) -> Result<(), TemplateError> {
        if let Some(dir) = &self.template_dir {
            fs::create_dir_all(dir).await?;
            
            let file_path = dir.join(format!("{}.json", template.name));
            let json = serde_json::to_string_pretty(template)?;
            fs::write(file_path, json).await?;
        }
        Ok(())
    }

    pub fn search_templates(&self, query: &str) -> Vec<&Template> {
        self.templates
            .values()
            .filter(|template| {
                template.name.contains(query) ||
                template.description.as_ref().map_or(false, |d| d.contains(query)) ||
                template.tags.iter().any(|tag| tag.contains(query))
            })
            .collect()
    }

    pub fn get_templates_by_tag(&self, tag: &str) -> Vec<&Template> {
        self.templates
            .values()
            .filter(|template| template.tags.contains(&tag.to_string()))
            .collect()
    }
}

pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
    template_store: TemplateStore,
    config: TemplateConfig,
    custom_helpers: HashMap<String, Box<dyn HelperDef + Send + Sync>>,
}

impl TemplateEngine {
    pub fn new(config: TemplateConfig) -> Self {
        let mut handlebars = Handlebars::new();
        
        // Configure Handlebars for security
        handlebars.set_strict_mode(config.enable_sandboxing);
        
        // Register built-in helpers only if allowed
        if config.allowed_helpers.contains(&"upper".to_string()) {
            handlebars.register_helper("upper", Box::new(upper_helper));
        }
        if config.allowed_helpers.contains(&"lower".to_string()) {
            handlebars.register_helper("lower", Box::new(lower_helper));
        }
        if config.allowed_helpers.contains(&"trim".to_string()) {
            handlebars.register_helper("trim", Box::new(trim_helper));
        }
        if config.allowed_helpers.contains(&"format".to_string()) {
            handlebars.register_helper("format", Box::new(format_helper));
        }
        if config.allowed_helpers.contains(&"default".to_string()) {
            handlebars.register_helper("default", Box::new(default_helper));
        }
        if config.allowed_helpers.contains(&"length".to_string()) {
            handlebars.register_helper("length", Box::new(length_helper));
        }
        if config.allowed_helpers.contains(&"join".to_string()) {
            handlebars.register_helper("join", Box::new(join_helper));
        }
        if config.allowed_helpers.contains(&"contains".to_string()) {
            handlebars.register_helper("contains", Box::new(contains_helper));
        }
        if config.allowed_helpers.contains(&"eq".to_string()) {
            handlebars.register_helper("eq", Box::new(eq_helper));
        }
        if config.allowed_helpers.contains(&"gt".to_string()) {
            handlebars.register_helper("gt", Box::new(gt_helper));
        }
        
        Self {
            handlebars,
            template_store: TemplateStore::new(config.template_dir.clone()),
            config,
            custom_helpers: HashMap::new(),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(TemplateConfig::default())
    }

    pub fn render(&mut self, template_name: &str, context: &Value) -> Result<String, TemplateError> {
        let template = self.template_store
            .get_template(template_name)
            .ok_or_else(|| TemplateError::NotFound(template_name.to_string()))?;

        // Security check: validate template size
        if self.config.enable_sandboxing && template.content.len() > self.config.max_template_size {
            return Err(TemplateError::Security(
                format!("Template size {} exceeds maximum allowed size {}", 
                    template.content.len(), self.config.max_template_size)
            ));
        }

        // Validate required variables
        self.validate_context(template, context)?;

        // Handle template composition
        let final_content = if let Some(parent_name) = &template.parent_template {
            self.compose_template(template, parent_name)?
        } else {
            template.content.clone()
        };

        // Register the template if not already registered
        if !self.handlebars.has_template(template_name) {
            self.handlebars
                .register_template_string(template_name, &final_content)
                .map_err(|e| TemplateError::Syntax(e.to_string()))?;
        }

        // Render with timeout if sandboxing is enabled
        let rendered = if self.config.enable_sandboxing {
            self.render_with_timeout(template_name, context)?
        } else {
            self.handlebars.render(template_name, context)?
        };

        Ok(rendered)
    }

    fn compose_template(&self, template: &Template, parent_name: &str) -> Result<String, TemplateError> {
        let parent = self.template_store
            .get_template(parent_name)
            .ok_or_else(|| TemplateError::Composition(
                format!("Parent template '{}' not found", parent_name)
            ))?;

        // Simple composition: replace {{> content}} in parent with child content
        let composed = parent.content.replace("{{> content}}", &template.content);
        Ok(composed)
    }

    fn render_with_timeout(&self, template_name: &str, context: &Value) -> Result<String, TemplateError> {
        // For now, just render normally. In a production system, you'd use tokio::time::timeout
        // or a similar mechanism to enforce rendering timeouts
        self.handlebars.render(template_name, context)
            .map_err(TemplateError::Rendering)
    }

    pub fn register_template(&mut self, template: Template) -> Result<(), TemplateError> {
        // Security validation
        if self.config.enable_sandboxing {
            self.validate_template_security(&template)?;
        }

        // Validate template syntax
        self.validate_template(&template.content)?;
        
        // Register with Handlebars
        self.handlebars
            .register_template_string(&template.name, &template.content)
            .map_err(|e| TemplateError::Syntax(e.to_string()))?;

        // Store template
        self.template_store.add_template(template);
        
        Ok(())
    }

    fn validate_template_security(&self, template: &Template) -> Result<(), TemplateError> {
        // Check template size
        if template.content.len() > self.config.max_template_size {
            return Err(TemplateError::Security(
                format!("Template size {} exceeds maximum allowed size {}", 
                    template.content.len(), self.config.max_template_size)
            ));
        }

        // Check for potentially dangerous patterns
        let dangerous_patterns = [
            "{{#raw}}",
            "{{{{",
            "}}}}",
            "javascript:",
            "<script",
            "eval(",
            "Function(",
        ];

        for pattern in &dangerous_patterns {
            if template.content.contains(pattern) {
                return Err(TemplateError::Security(
                    format!("Template contains potentially dangerous pattern: {}", pattern)
                ));
            }
        }

        Ok(())
    }

    pub fn list_templates(&self) -> Vec<&Template> {
        self.template_store.list_templates()
    }

    pub fn validate_template(&self, content: &str) -> Result<(), TemplateError> {
        // Try to compile the template to check syntax
        match self.handlebars.render_template(content, &Value::Object(serde_json::Map::new())) {
            Ok(_) => Ok(()),
            Err(e) => {
                // Provide more detailed error information with line numbers
                let error_msg = self.format_template_error(content, &e);
                Err(TemplateError::Syntax(error_msg))
            }
        }
    }

    fn format_template_error(&self, content: &str, error: &RenderError) -> String {
        let error_str = error.to_string();
        
        // Try to extract line information from the error
        if let Some(line_info) = self.extract_line_info(&error_str) {
            let lines: Vec<&str> = content.lines().collect();
            let mut result = format!("Template syntax error: {}\n", error_str);
            
            if let Ok(line_num) = line_info.parse::<usize>() {
                if line_num > 0 && line_num <= lines.len() {
                    result.push_str(&format!("Line {}: {}\n", line_num, lines[line_num - 1]));
                    
                    // Add context lines if available
                    if line_num > 1 {
                        result.push_str(&format!("Line {}: {}\n", line_num - 1, lines[line_num - 2]));
                    }
                    if line_num < lines.len() {
                        result.push_str(&format!("Line {}: {}\n", line_num + 1, lines[line_num]));
                    }
                }
            }
            
            result
        } else {
            format!("Template syntax error: {}", error_str)
        }
    }

    fn extract_line_info(&self, error_str: &str) -> Option<String> {
        // Simple regex-like extraction for line numbers
        // This is a simplified approach - in production, you'd use proper regex
        if let Some(start) = error_str.find("line ") {
            let after_line = &error_str[start + 5..];
            if let Some(end) = after_line.find(|c: char| !c.is_ascii_digit()) {
                return Some(after_line[..end].to_string());
            }
        }
        None
    }

    pub fn register_helper<F>(&mut self, name: &str, helper: F) -> Result<(), TemplateError>
    where
        F: HelperDef + Send + Sync + 'static,
    {
        // Check if helper is allowed
        if self.config.enable_sandboxing && !self.config.allowed_helpers.contains(&name.to_string()) {
            return Err(TemplateError::Security(
                format!("Helper '{}' is not in the allowed helpers list", name)
            ));
        }

        self.handlebars.register_helper(name, Box::new(helper));
        Ok(())
    }

    pub fn create_template_with_defaults(
        &self,
        name: String,
        content: String,
        description: Option<String>,
    ) -> Template {
        Template {
            name,
            content,
            description,
            variables: Vec::new(),
            created_at: SystemTime::now(),
            parent_template: None,
            tags: Vec::new(),
            usage_examples: Vec::new(),
        }
    }

    pub fn search_templates(&self, query: &str) -> Vec<&Template> {
        self.template_store.search_templates(query)
    }

    pub fn get_templates_by_tag(&self, tag: &str) -> Vec<&Template> {
        self.template_store.get_templates_by_tag(tag)
    }

    fn validate_context(&self, template: &Template, context: &Value) -> Result<(), TemplateError> {
        let context_obj = context.as_object()
            .ok_or_else(|| TemplateError::Validation("Context must be an object".to_string()))?;

        for var in &template.variables {
            if var.required && !context_obj.contains_key(&var.name) {
                // Check if there's a default value
                if var.default_value.is_none() {
                    return Err(TemplateError::Validation(
                        format!("Required variable '{}' is missing and has no default value", var.name)
                    ));
                }
            }

            // Validate variable type if present
            if let Some(value) = context_obj.get(&var.name) {
                self.validate_variable_type(var, value)?;
            }
        }

        Ok(())
    }

    fn validate_variable_type(&self, var: &TemplateVariable, value: &Value) -> Result<(), TemplateError> {
        let matches = match var.var_type {
            VariableType::String => value.is_string(),
            VariableType::Number => value.is_number(),
            VariableType::Boolean => value.is_boolean(),
            VariableType::Array => value.is_array(),
            VariableType::Object => value.is_object(),
        };

        if !matches {
            return Err(TemplateError::Validation(
                format!("Variable '{}' has incorrect type. Expected {:?}, got {:?}", 
                    var.name, var.var_type, value)
            ));
        }

        Ok(())
    }

    pub fn render_with_defaults(&mut self, template_name: &str, mut context: Value) -> Result<String, TemplateError> {
        let template = self.template_store
            .get_template(template_name)
            .ok_or_else(|| TemplateError::NotFound(template_name.to_string()))?;

        // Apply default values for missing variables
        if let Some(context_obj) = context.as_object_mut() {
            for var in &template.variables {
                if !context_obj.contains_key(&var.name) {
                    if let Some(default_value) = &var.default_value {
                        context_obj.insert(var.name.clone(), default_value.clone());
                    }
                }
            }
        }

        self.render(template_name, &context)
    }

    pub async fn save_template(&mut self, template: Template) -> Result<(), TemplateError> {
        self.template_store.save_to_disk(&template).await?;
        self.register_template(template)?;
        Ok(())
    }

    pub async fn load_templates(&mut self) -> Result<(), TemplateError> {
        self.template_store.load_from_disk().await?;
        
        // Re-register all loaded templates with Handlebars
        for template in self.template_store.list_templates() {
            let final_content = if let Some(parent_name) = &template.parent_template {
                self.compose_template(template, parent_name)?
            } else {
                template.content.clone()
            };
            
            self.handlebars
                .register_template_string(&template.name, &final_content)
                .map_err(|e| TemplateError::Syntax(e.to_string()))?;
        }
        
        Ok(())
    }

    pub async fn reload_template(&mut self, template_name: &str) -> Result<(), TemplateError> {
        if self.config.auto_reload {
            // Remove from Handlebars
            self.handlebars.unregister_template(template_name);
            
            // Reload from disk if template directory is configured
            if let Some(dir) = &self.config.template_dir {
                let file_path = dir.join(format!("{}.json", template_name));
                if file_path.exists() {
                    let content = fs::read_to_string(&file_path).await?;
                    let template: Template = serde_json::from_str(&content)?;
                    self.register_template(template)?;
                }
            }
        }
        Ok(())
    }

    pub fn remove_template(&mut self, template_name: &str) -> Option<Template> {
        self.handlebars.unregister_template(template_name);
        self.template_store.remove_template(template_name)
    }

    pub async fn export_template(&self, template_name: &str, export_path: &PathBuf) -> Result<(), TemplateError> {
        let template = self.template_store
            .get_template(template_name)
            .ok_or_else(|| TemplateError::NotFound(template_name.to_string()))?;

        let json = serde_json::to_string_pretty(template)?;
        fs::write(export_path, json).await?;
        Ok(())
    }

    pub async fn import_template(&mut self, import_path: &PathBuf) -> Result<String, TemplateError> {
        let content = fs::read_to_string(import_path).await?;
        let template: Template = serde_json::from_str(&content)?;
        let template_name = template.name.clone();
        
        self.register_template(template)?;
        Ok(template_name)
    }

    pub fn clone_template(&mut self, source_name: &str, new_name: &str) -> Result<(), TemplateError> {
        let source_template = self.template_store
            .get_template(source_name)
            .ok_or_else(|| TemplateError::NotFound(source_name.to_string()))?;

        let mut cloned_template = source_template.clone();
        cloned_template.name = new_name.to_string();
        cloned_template.created_at = SystemTime::now();
        
        self.register_template(cloned_template)?;
        Ok(())
    }

    pub fn get_template_info(&self, template_name: &str) -> Option<TemplateInfo> {
        self.template_store.get_template(template_name).map(|template| {
            TemplateInfo {
                name: template.name.clone(),
                description: template.description.clone(),
                variable_count: template.variables.len(),
                created_at: template.created_at,
                parent_template: template.parent_template.clone(),
                tags: template.tags.clone(),
                content_length: template.content.len(),
                has_composition: template.parent_template.is_some(),
            }
        })
    }
}

// Helper functions
fn upper_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let param = h.param(0)
        .ok_or_else(|| RenderError::new("upper helper requires one parameter"))?;
    
    let value = param.value().as_str()
        .ok_or_else(|| RenderError::new("upper helper parameter must be a string"))?;
    
    out.write(&value.to_uppercase())?;
    Ok(())
}

fn lower_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let param = h.param(0)
        .ok_or_else(|| RenderError::new("lower helper requires one parameter"))?;
    
    let value = param.value().as_str()
        .ok_or_else(|| RenderError::new("lower helper parameter must be a string"))?;
    
    out.write(&value.to_lowercase())?;
    Ok(())
}

fn trim_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let param = h.param(0)
        .ok_or_else(|| RenderError::new("trim helper requires one parameter"))?;
    
    let value = param.value().as_str()
        .ok_or_else(|| RenderError::new("trim helper parameter must be a string"))?;
    
    out.write(value.trim())?;
    Ok(())
}

// Advanced helper functions
fn length_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let param = h.param(0)
        .ok_or_else(|| RenderError::new("length helper requires one parameter"))?;
    
    let length = match param.value() {
        Value::Array(arr) => arr.len(),
        Value::String(s) => s.len(),
        Value::Object(obj) => obj.len(),
        _ => return Err(RenderError::new("length helper parameter must be an array, string, or object")),
    };
    
    out.write(&length.to_string())?;
    Ok(())
}

fn join_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let array_param = h.param(0)
        .ok_or_else(|| RenderError::new("join helper requires two parameters"))?;
    
    let separator = h.param(1)
        .ok_or_else(|| RenderError::new("join helper requires two parameters"))?
        .value().as_str()
        .ok_or_else(|| RenderError::new("join helper second parameter must be a string"))?;
    
    let array = array_param.value().as_array()
        .ok_or_else(|| RenderError::new("join helper first parameter must be an array"))?;
    
    let joined = array.iter()
        .map(|v| match v {
            Value::String(s) => s.clone(),
            _ => v.to_string(),
        })
        .collect::<Vec<_>>()
        .join(separator);
    
    out.write(&joined)?;
    Ok(())
}

fn contains_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let haystack = h.param(0)
        .ok_or_else(|| RenderError::new("contains helper requires two parameters"))?;
    
    let needle = h.param(1)
        .ok_or_else(|| RenderError::new("contains helper requires two parameters"))?;
    
    let contains = match (haystack.value(), needle.value()) {
        (Value::String(s), Value::String(n)) => s.contains(n),
        (Value::Array(arr), needle_val) => arr.contains(needle_val),
        _ => false,
    };
    
    out.write(&contains.to_string())?;
    Ok(())
}

fn eq_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let left = h.param(0)
        .ok_or_else(|| RenderError::new("eq helper requires two parameters"))?;
    
    let right = h.param(1)
        .ok_or_else(|| RenderError::new("eq helper requires two parameters"))?;
    
    let equal = left.value() == right.value();
    out.write(&equal.to_string())?;
    Ok(())
}

fn gt_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let left = h.param(0)
        .ok_or_else(|| RenderError::new("gt helper requires two parameters"))?;
    
    let right = h.param(1)
        .ok_or_else(|| RenderError::new("gt helper requires two parameters"))?;
    
    let greater = match (left.value(), right.value()) {
        (Value::Number(l), Value::Number(r)) => {
            l.as_f64().unwrap_or(0.0) > r.as_f64().unwrap_or(0.0)
        },
        _ => false,
    };
    
    out.write(&greater.to_string())?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub name: String,
    pub description: Option<String>,
    pub variable_count: usize,
    pub created_at: SystemTime,
    pub parent_template: Option<String>,
    pub tags: Vec<String>,
    pub content_length: usize,
    pub has_composition: bool,
}

// Additional helper functions
fn format_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let format_str = h.param(0)
        .ok_or_else(|| RenderError::new("format helper requires at least one parameter"))?
        .value().as_str()
        .ok_or_else(|| RenderError::new("format helper first parameter must be a string"))?;
    
    let mut result = format_str.to_string();
    
    // Simple placeholder replacement
    for (i, param) in h.params().iter().skip(1).enumerate() {
        let placeholder = format!("{{{}}}", i);
        let value = match param.value() {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => param.value().to_string(),
        };
        result = result.replace(&placeholder, &value);
    }
    
    out.write(&result)?;
    Ok(())
}

fn default_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let value = h.param(0)
        .ok_or_else(|| RenderError::new("default helper requires two parameters"))?;
    
    let default_value = h.param(1)
        .ok_or_else(|| RenderError::new("default helper requires two parameters"))?;
    
    let output = if value.value().is_null() || 
                   (value.value().is_string() && value.value().as_str().unwrap_or("").is_empty()) {
        default_value.value()
    } else {
        value.value()
    };
    
    match output {
        Value::String(s) => out.write(s)?,
        Value::Number(n) => out.write(&n.to_string())?,
        Value::Bool(b) => out.write(&b.to_string())?,
        _ => out.write(&output.to_string())?,
    }
    
    Ok(())
}
#[cfg(
test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn create_test_config() -> TemplateConfig {
        TemplateConfig {
            template_dir: Some(PathBuf::from("test_templates")),
            auto_reload: true,
            enable_sandboxing: true,
            max_template_size: 1024,
            max_render_time_ms: 1000,
            allowed_helpers: vec![
                "upper".to_string(),
                "lower".to_string(),
                "trim".to_string(),
                "if".to_string(),
                "each".to_string(),
            ],
        }
    }

    fn create_test_template() -> Template {
        Template {
            name: "test_template".to_string(),
            content: "Hello {{name}}!".to_string(),
            description: Some("A test template".to_string()),
            variables: vec![
                TemplateVariable {
                    name: "name".to_string(),
                    var_type: VariableType::String,
                    required: true,
                    default_value: None,
                    description: Some("The name to greet".to_string()),
                }
            ],
            created_at: SystemTime::now(),
            parent_template: None,
            tags: vec!["test".to_string()],
            usage_examples: vec!["{{name}} = 'World'".to_string()],
        }
    }

    #[test]
    fn test_template_engine_creation() {
        let engine = TemplateEngine::new(create_test_config());
        assert_eq!(engine.list_templates().len(), 0);
    }

    #[test]
    fn test_template_registration() {
        let mut engine = TemplateEngine::new(create_test_config());
        let template = create_test_template();
        
        let result = engine.register_template(template);
        assert!(result.is_ok());
        assert_eq!(engine.list_templates().len(), 1);
    }

    #[test]
    fn test_template_rendering() {
        let mut engine = TemplateEngine::new(create_test_config());
        let template = create_test_template();
        
        engine.register_template(template).unwrap();
        
        let context = json!({
            "name": "World"
        });
        
        let result = engine.render("test_template", &context);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello World!");
    }

    #[test]
    fn test_template_validation_missing_required_variable() {
        let mut engine = TemplateEngine::new(create_test_config());
        let template = create_test_template();
        
        engine.register_template(template).unwrap();
        
        let context = json!({});
        
        let result = engine.render("test_template", &context);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TemplateError::Validation(_)));
    }

    #[test]
    fn test_template_with_default_values() {
        let mut engine = TemplateEngine::new(create_test_config());
        
        let mut template = create_test_template();
        template.variables[0].required = false;
        template.variables[0].default_value = Some(json!("Anonymous"));
        
        engine.register_template(template).unwrap();
        
        let context = json!({});
        
        let result = engine.render_with_defaults("test_template", context);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello Anonymous!");
    }

    #[test]
    fn test_template_security_validation() {
        let config = create_test_config();
        let mut engine = TemplateEngine::new(config);
        
        let mut dangerous_template = create_test_template();
        dangerous_template.content = "{{#raw}}<script>alert('xss')</script>{{/raw}}".to_string();
        
        let result = engine.register_template(dangerous_template);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TemplateError::Security(_)));
    }

    #[test]
    fn test_template_size_limit() {
        let mut config = create_test_config();
        config.max_template_size = 10; // Very small limit
        
        let mut engine = TemplateEngine::new(config);
        let template = create_test_template(); // This will exceed the limit
        
        let result = engine.register_template(template);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TemplateError::Security(_)));
    }

    #[test]
    fn test_template_composition() {
        let mut engine = TemplateEngine::new(create_test_config());
        
        // Create parent template
        let parent_template = Template {
            name: "parent".to_string(),
            content: "Header\n{{> content}}\nFooter".to_string(),
            description: Some("Parent template".to_string()),
            variables: vec![],
            created_at: SystemTime::now(),
            parent_template: None,
            tags: vec!["layout".to_string()],
            usage_examples: vec![],
        };
        
        // Create child template
        let mut child_template = create_test_template();
        child_template.name = "child".to_string();
        child_template.parent_template = Some("parent".to_string());
        
        engine.register_template(parent_template).unwrap();
        engine.register_template(child_template).unwrap();
        
        let context = json!({
            "name": "World"
        });
        
        let result = engine.render("child", &context);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Header\nHello World!\nFooter");
    }

    #[test]
    fn test_template_search() {
        let mut engine = TemplateEngine::new(create_test_config());
        
        let mut template1 = create_test_template();
        template1.name = "greeting".to_string();
        template1.tags = vec!["greeting".to_string(), "basic".to_string()];
        
        let mut template2 = create_test_template();
        template2.name = "farewell".to_string();
        template2.content = "Goodbye {{name}}!".to_string();
        template2.tags = vec!["farewell".to_string(), "basic".to_string()];
        
        engine.register_template(template1).unwrap();
        engine.register_template(template2).unwrap();
        
        let search_results = engine.search_templates("greeting");
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].name, "greeting");
        
        let tag_results = engine.get_templates_by_tag("basic");
        assert_eq!(tag_results.len(), 2);
    }

    #[test]
    fn test_variable_type_validation() {
        let mut engine = TemplateEngine::new(create_test_config());
        
        let mut template = create_test_template();
        template.variables.push(TemplateVariable {
            name: "age".to_string(),
            var_type: VariableType::Number,
            required: true,
            default_value: None,
            description: Some("Age as a number".to_string()),
        });
        
        engine.register_template(template).unwrap();
        
        // Test with correct types
        let valid_context = json!({
            "name": "Alice",
            "age": 30
        });
        
        let result = engine.render("test_template", &valid_context);
        assert!(result.is_ok());
        
        // Test with incorrect type
        let invalid_context = json!({
            "name": "Alice",
            "age": "thirty" // String instead of number
        });
        
        let result = engine.render("test_template", &invalid_context);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TemplateError::Validation(_)));
    }

    #[test]
    fn test_helper_functions() {
        let mut engine = TemplateEngine::new(create_test_config());
        
        let template = Template {
            name: "helper_test".to_string(),
            content: "{{upper name}} and {{lower name}}".to_string(),
            description: None,
            variables: vec![
                TemplateVariable {
                    name: "name".to_string(),
                    var_type: VariableType::String,
                    required: true,
                    default_value: None,
                    description: None,
                }
            ],
            created_at: SystemTime::now(),
            parent_template: None,
            tags: vec![],
            usage_examples: vec![],
        };
        
        engine.register_template(template).unwrap();
        
        let context = json!({
            "name": "World"
        });
        
        let result = engine.render("helper_test", &context);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "WORLD and world");
    }

    #[test]
    fn test_advanced_helpers() {
        let mut config = create_test_config();
        config.allowed_helpers.extend(vec![
            "length".to_string(),
            "join".to_string(),
            "contains".to_string(),
            "eq".to_string(),
            "gt".to_string(),
        ]);
        
        let mut engine = TemplateEngine::new(config);
        
        let template = Template {
            name: "advanced_test".to_string(),
            content: "Length: {{length items}}, Joined: {{join items \", \"}}, Contains: {{contains text \"hello\"}}".to_string(),
            description: None,
            variables: vec![
                TemplateVariable {
                    name: "items".to_string(),
                    var_type: VariableType::Array,
                    required: true,
                    default_value: None,
                    description: None,
                },
                TemplateVariable {
                    name: "text".to_string(),
                    var_type: VariableType::String,
                    required: true,
                    default_value: None,
                    description: None,
                }
            ],
            created_at: SystemTime::now(),
            parent_template: None,
            tags: vec![],
            usage_examples: vec![],
        };
        
        engine.register_template(template).unwrap();
        
        let context = json!({
            "items": ["apple", "banana", "cherry"],
            "text": "hello world"
        });
        
        let result = engine.render("advanced_test", &context);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Length: 3, Joined: apple, banana, cherry, Contains: true");
    }

    #[test]
    fn test_template_error_formatting() {
        let engine = TemplateEngine::new(create_test_config());
        
        let invalid_template = "{{#if unclosed_block}}\nHello World\n{{/wrong_block}}";
        let result = engine.validate_template(invalid_template);
        
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Template syntax error"));
    }

    #[test]
    fn test_template_management_operations() {
        let mut engine = TemplateEngine::new(create_test_config());
        let template = create_test_template();
        
        // Test registration and info retrieval
        engine.register_template(template).unwrap();
        let info = engine.get_template_info("test_template");
        assert!(info.is_some());
        assert_eq!(info.unwrap().name, "test_template");
        
        // Test cloning
        let clone_result = engine.clone_template("test_template", "cloned_template");
        assert!(clone_result.is_ok());
        assert_eq!(engine.list_templates().len(), 2);
        
        // Test removal
        let removed = engine.remove_template("cloned_template");
        assert!(removed.is_some());
        assert_eq!(engine.list_templates().len(), 1);
    }
}