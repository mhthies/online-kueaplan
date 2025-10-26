use crate::web::ui::error::AppError;
use crate::web::ui::form_values::{BoolFormValue, FormValue, FormValueRepresentation};
use askama::Template;
use serde::Serialize;
use std::borrow::Cow;

pub enum InputSize {
    Small,
    Normal,
    Large,
}

#[derive(Debug, PartialEq)]
pub enum InputType {
    Text,
    Date,
    Time,
    Color,
    Textarea,
    Integer,
}

impl InputType {
    fn as_html_type_attr(&self) -> &'static str {
        match self {
            InputType::Text => "text",
            InputType::Date => "date",
            InputType::Time => "time",
            InputType::Color => "color",
            InputType::Integer => "number",
            _ => panic!("Input type {:?} should be handled separately.", self),
        }
    }
}

pub struct InputConfiguration<'a> {
    size: InputSize,
    input_type: InputType,
    suffix_text: Option<&'a str>,
    info: Option<&'a str>,
}

impl Default for InputConfiguration<'_> {
    fn default() -> Self {
        Self {
            size: InputSize::Normal,
            input_type: InputType::Text,
            suffix_text: None,
            info: None,
        }
    }
}

impl<'a> InputConfiguration<'a> {
    pub fn builder() -> InputConfigurationBuilder<'a> {
        InputConfigurationBuilder::default()
    }
}

#[derive(Default)]
pub struct InputConfigurationBuilder<'a> {
    value: InputConfiguration<'a>,
}

impl<'a> InputConfigurationBuilder<'a> {
    pub fn size(mut self, size: InputSize) -> Self {
        self.value.size = size;
        self
    }
    pub fn input_type(mut self, input_type: InputType) -> Self {
        self.value.input_type = input_type;
        self
    }
    pub fn info<'b: 'a>(mut self, info: &'b str) -> Self {
        self.value.info = Some(info);
        self
    }
    pub fn suffix_text(mut self, suffix_text: &'a str) -> Self {
        self.value.suffix_text = Some(suffix_text);
        self
    }
    pub fn build(self) -> InputConfiguration<'a> {
        self.value
    }
}

#[derive(Template)]
#[template(path = "sub_templates/form_inputs/form_field.html")]
pub struct FormFieldTemplate<'a, T: FormValueRepresentation> {
    name: &'a str,
    label: &'a str,
    config: InputConfiguration<'a>,
    data: &'a FormValue<T>,
}

impl<'a, T: FormValueRepresentation> FormFieldTemplate<'a, T> {
    pub fn new(
        data: &'a FormValue<T>,
        name: &'a str,
        label: &'a str,
        config: InputConfiguration<'a>,
    ) -> Self {
        Self {
            name,
            label,
            config,
            data,
        }
    }
}

#[derive(Template)]
#[template(path = "sub_templates/form_inputs/hidden_input.html")]
pub struct HiddenInputTemplate<'a, T: FormValueRepresentation> {
    name: &'a str,
    data: &'a FormValue<T>,
}

impl<'a, T: FormValueRepresentation> HiddenInputTemplate<'a, T> {
    pub fn new(data: &'a FormValue<T>, name: &'a str) -> Result<Self, AppError> {
        if data.has_errors() {
            // TODO special error type?
            return Err(AppError::InternalError(format!(
                "Validation error in hidden field {}: {}",
                name,
                data.errors().join(", ")
            )));
        }
        Ok(Self { name, data })
    }
}

#[derive(Serialize)]
pub struct SelectEntry<'a> {
    pub value: Cow<'a, str>,
    pub text: Cow<'a, str>,
}

#[derive(Template)]
#[template(path = "sub_templates/form_inputs/select.html")]
pub struct SelectTemplate<'a, T: FormValueRepresentation> {
    name: &'a str,
    entries: &'a Vec<SelectEntry<'a>>,
    label: &'a str,
    config: InputConfiguration<'a>,
    data: &'a FormValue<T>,
}

impl<'a, T: FormValueRepresentation> SelectTemplate<'a, T> {
    pub fn new(
        data: &'a FormValue<T>,
        name: &'a str,
        entries: &'a Vec<SelectEntry>,
        label: &'a str,
        config: InputConfiguration<'a>,
    ) -> Self {
        Self {
            name,
            entries,
            label,
            config,
            data,
        }
    }
}

#[derive(Template)]
#[template(path = "sub_templates/form_inputs/checkbox.html")]
pub struct CheckboxTemplate<'a> {
    name: &'a str,
    label: &'a str,
    info: Option<&'a str>,
    data: &'a BoolFormValue,
}

impl<'a> CheckboxTemplate<'a> {
    pub fn new(
        data: &'a BoolFormValue,
        name: &'a str,
        label: &'a str,
        info: Option<&'a str>,
    ) -> Self {
        Self {
            name,
            label,
            info,
            data,
        }
    }
}
