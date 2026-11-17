//! The fake of the `templates` port: `{{ name }}` replaced by its value, and nothing else read.

use crate::outbound::templates::errors::TemplatesError;
use crate::outbound::templates::interface::TemplatesPort;
use tastemaker_values::domain::contexts::types::{ContextKey, ContextKeys, RenderContext};
use tastemaker_values::domain::definitions::types::TemplateText;
use tastemaker_values::domain::files::types::FileText;

/// What the core's tests stand in for `templates` with.
#[derive(Clone, Copy, Debug, Default)]
pub struct TemplatesFake;

impl TemplatesPort for TemplatesFake {
    fn render(
        &self,
        template: &TemplateText,
        context: &RenderContext,
    ) -> Result<FileText, TemplatesError> {
        if template.0.contains("{%") && !template.0.contains("%}")
            || template.0.contains("{% if %}")
        {
            return Err(TemplatesError::Render {
                reason: "the fake reads no statement".to_owned(),
            });
        }
        let mut text = template.0.to_string();
        for (name, value) in &context.values {
            text = text.replace(&format!("{{{{ {} }}}}", name.0), &value.0);
        }
        Ok(FileText(text.into()))
    }

    fn reads(&self, template: &TemplateText) -> Result<ContextKeys, TemplatesError> {
        Ok(ContextKeys(
            template
                .0
                .split("{{ ")
                .skip(1)
                .filter_map(|rest| rest.split_once(" }}"))
                .map(|(name, _)| ContextKey(name.into()))
                .collect(),
        ))
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
