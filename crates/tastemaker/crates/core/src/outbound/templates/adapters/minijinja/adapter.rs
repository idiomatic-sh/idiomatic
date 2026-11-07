//! The templates port, implemented with minijinja: strict about names the context does not hold, trailing newlines
//! kept, and the case filters of heck.

use heck::{
    ToKebabCase as _, ToLowerCamelCase as _, ToShoutySnakeCase as _, ToSnakeCase as _,
    ToUpperCamelCase as _,
};

use super::constants::{CAMEL, KEBAB, KEBAB_TEXT, PASCAL, REFUSE, SHOUTY, SNAKE};
use crate::domain::contexts::constants::{
    CAPTURES, IDENTIFIER_JOIN, INSTANCES, KEBAB_JOIN, PRESENT,
};
use crate::domain::contexts::types::{ContextKey, ContextKeys, Present, RenderContext};
use crate::domain::definitions::types::TemplateText;
use crate::domain::files::types::FileText;
use crate::outbound::templates::errors::TemplatesError;
use crate::outbound::templates::interface::TemplatesPort;

/// The minijinja templates adapter.
#[derive(Debug)]
pub(crate) struct TemplatesMinijinjaAdapter {
    /// The environment, configured once: strict names, kept trailing newlines, the case filters and `refuse`.
    environment: minijinja::Environment<'static>,
}

impl TemplatesMinijinjaAdapter {
    /// The adapter, its environment configured once.
    #[must_use]
    pub(crate) fn new() -> Self {
        let mut environment = minijinja::Environment::new();
        environment.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
        environment.set_keep_trailing_newline(true);
        environment.add_filter(PASCAL, |value: &str| value.to_upper_camel_case());
        environment.add_filter(CAMEL, |value: &str| value.to_lower_camel_case());
        environment.add_filter(SNAKE, |value: &str| value.to_snake_case());
        environment.add_filter(KEBAB, |value: &str| value.to_kebab_case());
        environment.add_filter(SHOUTY, |value: &str| value.to_shouty_snake_case());
        environment.add_function(
            REFUSE,
            |why: minijinja::Value| -> Result<minijinja::Value, minijinja::Error> {
                Err(minijinja::Error::new(
                    minijinja::ErrorKind::InvalidOperation,
                    why.to_string(),
                ))
            },
        );
        Self { environment }
    }

    /// The names `template` reads that neither `given` nor the environment's globals hold, in order.
    fn unset(&self, template: &TemplateText, given: &minijinja::Value) -> Vec<ContextKey> {
        let Ok(parsed) = self.environment.template_from_str(&template.0) else {
            return Vec::new();
        };
        let mut unset = parsed
            .undeclared_variables(false)
            .into_iter()
            .filter(|name| {
                given.get_attr(name).is_ok_and(|value| value.is_undefined())
                    && self.environment.globals().all(|(global, _)| global != name)
            })
            // Said as it is written, `item-trait`, never as the identifier the template reads it by.
            .map(|name| ContextKey(name.replace(IDENTIFIER_JOIN, KEBAB_TEXT).into()))
            .collect::<Vec<_>>();
        unset.sort();
        unset
    }
}

/// What the content reads: its values, and the three maps under their reserved names. A param is kebab-case and a
/// template's variable an identifier, so `item-trait` is read as `item_trait`.
fn values(context: &RenderContext) -> minijinja::Value {
    let mut values = context
        .values
        .iter()
        .map(|(name, value)| {
            (
                name.0.replace(KEBAB_JOIN, IDENTIFIER_JOIN),
                minijinja::Value::from(&*value.0),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let captured = context
        .captured
        .iter()
        .map(|(name, found)| {
            (
                name.0.to_string(),
                found
                    .iter()
                    .map(|value| value.0.to_string())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let present = context
        .present
        .iter()
        .map(|(name, present)| (name.0.to_string(), *present == Present::Yes))
        .collect::<std::collections::BTreeMap<_, _>>();
    let instances = context
        .instances
        .iter()
        .map(|(name, found)| {
            (
                name.0.to_string(),
                found
                    .iter()
                    .map(|instance| {
                        instance
                            .0
                            .iter()
                            .map(|(key, value)| (key.0.to_string(), value.0.to_string()))
                            .collect::<std::collections::BTreeMap<_, _>>()
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    values.extend([
        (
            CAPTURES.to_owned(),
            minijinja::Value::from_serialize(captured),
        ),
        (
            PRESENT.to_owned(),
            minijinja::Value::from_serialize(present),
        ),
        (
            INSTANCES.to_owned(),
            minijinja::Value::from_serialize(instances),
        ),
    ]);
    minijinja::Value::from_serialize(values)
}

impl TemplatesPort for TemplatesMinijinjaAdapter {
    fn render(
        &self,
        template: &TemplateText,
        context: &RenderContext,
    ) -> Result<FileText, TemplatesError> {
        let given = values(context);
        self.environment
            .render_str(&template.0, &given)
            .map(|text| FileText(text.into()))
            .map_err(|error| {
                let unset = self.unset(template, &given);
                if error.kind() == minijinja::ErrorKind::UndefinedError && !unset.is_empty() {
                    TemplatesError::Unset {
                        reason: error.to_string(),
                        names: unset.iter().fold(String::new(), |names, name| {
                            if names.is_empty() {
                                format!("`{}`", name.0)
                            } else {
                                format!("{names}, `{}`", name.0)
                            }
                        }),
                    }
                } else {
                    TemplatesError::Render {
                        reason: error.to_string(),
                    }
                }
            })
    }

    fn reads(&self, template: &TemplateText) -> Result<ContextKeys, TemplatesError> {
        let compiled = self
            .environment
            .template_from_str(&template.0)
            .map_err(|error| TemplatesError::Render {
                reason: error.to_string(),
            })?;
        Ok(ContextKeys(
            compiled
                .undeclared_variables(true)
                .iter()
                .flat_map(|name| {
                    name.split('.')
                        .map(|part| ContextKey(part.replace(IDENTIFIER_JOIN, KEBAB_TEXT).into()))
                        .collect::<Vec<_>>()
                })
                .collect(),
        ))
    }
}
