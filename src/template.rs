use handlebars::{Handlebars, Helper, RenderContext, RenderError, HelperResult, Output};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use thiserror::Error;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    pub content: String,
    pub description: Option<String>,
    pub variables: Vec<TemplateVariable>,
    pub created_at: SystemTime,
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
                // TODO: Implement loading templates from disk
                // For now, this is a placeholder
            }
        }
        Ok(())
    }

    pub async fn save_to_disk(&self, template: &Template) -> Result<(), TemplateError> {
        if let Some(dir) = &self.template_dir {
            if !dir.exists() {
                std::fs::create_dir_all(dir)?;
            }
            
            let file_path = dir.join(format!("{}.json", template.name));
            let json = serde_json::to_string_pretty(template)?;
            std::fs::write(file_path, json)?;
        }
        Ok(())
    }
}

pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
    template_store: TemplateStore,
}

impl TemplateEngine {
    pub fn new(template_dir: Option<PathBuf>) -> Self {
        let mut handlebars = Handlebars::new();
        
        // Register built-in helpers
        handlebars.register_helper("upper", Box::new(upper_helper));
        handlebars.register_helper("lower", Box::new(lower_helper));
        handlebars.register_helper("trim", Box::new(trim_helper));
        
        Self {
            handlebars,
            template_store: TemplateStore::new(template_dir),
        }
    }

    pub fn render(&mut self, template_name: &str, context: &Value) -> Result<String, TemplateError> {
        let template = self.template_store
            .get_template(template_name)
            .ok_or_else(|| TemplateError::NotFound(template_name.to_string()))?;

        // Validate required variables
        self.validate_context(template, context)?;

        // Register the template if not already registered
        if !self.handlebars.has_template(template_name) {
            self.handlebars
                .register_template_string(template_name, &template.content)
                .map_err(|e| TemplateError::Syntax(e.to_string()))?;
        }

        let rendered = self.handlebars.render(template_name, context)?;
        Ok(rendered)
    }

    pub fn register_template(&mut self, template: Template) -> Result<(), TemplateError> {
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

    pub fn list_templates(&self) -> Vec<&Template> {
        self.template_store.list_templates()
    }

    pub fn validate_template(&self, content: &str) -> Result<(), TemplateError> {
        // Try to compile the template to check syntax
        self.handlebars
            .render_template(content, &Value::Object(serde_json::Map::new()))
            .map_err(|e| TemplateError::Syntax(e.to_string()))?;
        
        Ok(())
    }

    fn validate_context(&self, template: &Template, context: &Value) -> Result<(), TemplateError> {
        let context_obj = context.as_object()
            .ok_or_else(|| TemplateError::Validation("Context must be an object".to_string()))?;

        for var in &template.variables {
            if var.required && !context_obj.contains_key(&var.name) {
                return Err(TemplateError::Validation(
                    format!("Required variable '{}' is missing", var.name)
                ));
            }
        }

        Ok(())
    }

    pub async fn save_template(&mut self, template: Template) -> Result<(), TemplateError> {
        self.template_store.save_to_disk(&template).await?;
        self.register_template(template)?;
        Ok(())
    }

    pub async fn load_templates(&mut self) -> Result<(), TemplateError> {
        self.template_store.load_from_disk().await?;
        Ok(())
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