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

#[derive(Debug)]
pub enum InputType {
    Text,
    Time,
    Textarea,
}

impl InputType {
    fn as_html_type_attr(&self) -> &'static str {
        match self {
            InputType::Text => "text",
            InputType::Time => "time",
            _ => panic!("Input type {:?} should be handled separately.", self),
        }
    }
}

pub struct InputConfiguration<'a> {
    size: InputSize,
    input_type: InputType,
    info: &'a str,
}

impl Default for InputConfiguration<'_> {
    fn default() -> Self {
        Self {
            size: InputSize::Normal,
            input_type: InputType::Text,
            info: "",
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
        self.value.info = info;
        self
    }
    pub fn build(self) -> InputConfiguration<'a> {
        self.value
    }
}

#[derive(Template)]
#[template(path = "forms/form_field.html")]
pub struct FormFieldTemplate<'a, T: FormValueRepresentation> {
    name: &'a str,
    label: &'a str,
    info: Option<&'a str>,
    input_type: InputType,
    size: InputSize,
    data: &'a FormValue<T>,
}

impl<'a, T: FormValueRepresentation> FormFieldTemplate<'a, T> {
    pub fn new(
        data: &'a FormValue<T>,
        name: &'a str,
        label: &'a str,
        config: InputConfiguration,
    ) -> Self {
        Self {
            name,
            label,
            info: None, // TODO
            input_type: config.input_type,
            size: config.size,
            data,
        }
    }
}

#[derive(Template)]
#[template(path = "forms/hidden_input.html")]
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
#[template(path = "forms/select.html")]
pub struct SelectTemplate<'a, T: FormValueRepresentation> {
    name: &'a str,
    entries: &'a Vec<SelectEntry<'a>>,
    label: &'a str,
    info: Option<&'a str>,
    size: InputSize,
    data: &'a FormValue<T>,
}

impl<'a, T: FormValueRepresentation> SelectTemplate<'a, T> {
    pub fn new(
        data: &'a FormValue<T>,
        name: &'a str,
        entries: &'a Vec<SelectEntry>,
        label: &'a str,
        info: Option<&'a str>,
        size: InputSize,
    ) -> Self {
        Self {
            name,
            entries,
            label,
            info,
            size,
            data,
        }
    }
}

#[derive(Template)]
#[template(path = "forms/checkbox.html")]
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
