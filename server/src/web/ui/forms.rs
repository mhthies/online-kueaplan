use crate::web::ui::error::AppError;
use askama::Template;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use std::borrow::Cow;
use std::fmt::Formatter;

#[derive(Debug, Deserialize, Default)]
#[serde(transparent)]
pub struct FormValue {
    value: String,
    #[serde(skip)]
    errors: Vec<String>,
}

impl FormValue {
    pub fn validate<'a, T: FromFormValue<'a>>(&'a mut self) -> Option<T> {
        match T::from_form_value(&self.value) {
            Ok(v) => Some(v),
            Err(e) => {
                self.errors.push(e);
                None
            }
        }
    }
    pub fn validate_with<'a, 'd, T: FromFormValueWithData<'a, 'd>>(
        &'a mut self,
        data: T::AdditionalData,
    ) -> Option<T> {
        match T::from_form_value(&self.value, data) {
            Ok(v) => Some(v),
            Err(e) => {
                self.errors.push(e);
                None
            }
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error)
    }

    pub fn create_form_field(
        &self,
        name: &str,
        label: &str,
        info: Option<&str>,
        input_type: InputType,
        size: InputSize,
    ) -> Result<askama::filters::Safe<String>, askama::Error> {
        let template = FormFieldTemplate {
            name,
            label,
            info,
            input_type,
            size,
            data: self,
        };
        Ok(askama::filters::Safe(template.render()?))
    }

    pub fn create_select(
        &self,
        name: &str,
        entries: &Vec<SelectEntry>,
        label: &str,
        info: Option<&str>,
        size: InputSize,
    ) -> Result<askama::filters::Safe<String>, askama::Error> {
        let template = SelectTemplate {
            name,
            entries,
            label,
            info,
            size,
            data: self,
        };
        Ok(askama::filters::Safe(template.render()?))
    }

    pub fn create_hidden_input(
        &self,
        name: &str,
    ) -> Result<askama::filters::Safe<String>, AppError> {
        if !self.errors.is_empty() {
            // TODO special error type?
            return Err(AppError::InternalError(format!(
                "Validation error in hidden field {}: {}",
                name,
                self.errors.join(", ")
            )));
        }
        let template = HiddenInputTemplate { name, data: self };
        Ok(askama::filters::Safe(template.render()?))
    }
}

pub trait IntoFormValue {
    fn into_form_value_string(self) -> String;
}
pub trait FromFormValue<'a>: Sized {
    fn from_form_value(value: &'a str) -> Result<Self, String>;
}
pub trait FromFormValueWithData<'a, 'd>: Sized {
    type AdditionalData: 'd;

    fn from_form_value(value: &'a str, data: Self::AdditionalData) -> Result<Self, String>;
}

impl<T> IntoFormValue for T
where
    T: ToString,
{
    fn into_form_value_string(self) -> String {
        self.to_string()
    }
}
impl<'a, T> FromFormValue<'a> for T
where
    T: TryFrom<&'a str> + Sized,
    T::Error: std::fmt::Display,
{
    fn from_form_value(value: &'a str) -> Result<Self, String> {
        <T as TryFrom<&str>>::try_from(value).map_err(|e| e.to_string())
    }
}
impl<T: IntoFormValue> From<T> for FormValue {
    fn from(value: T) -> Self {
        FormValue {
            value: value.into_form_value_string(),
            errors: Vec::new(),
        }
    }
}

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

#[derive(Template)]
#[template(path = "forms/form_field.html")]
struct FormFieldTemplate<'a> {
    name: &'a str,
    label: &'a str,
    info: Option<&'a str>,
    input_type: InputType,
    size: InputSize,
    data: &'a FormValue,
}

#[derive(Template)]
#[template(path = "forms/hidden_input.html")]
struct HiddenInputTemplate<'a> {
    name: &'a str,
    data: &'a FormValue,
}

#[derive(Serialize)]
pub struct SelectEntry<'a> {
    pub value: Cow<'a, str>,
    pub text: Cow<'a, str>,
}

#[derive(Template)]
#[template(path = "forms/select.html")]
struct SelectTemplate<'a> {
    name: &'a str,
    entries: &'a Vec<SelectEntry<'a>>,
    label: &'a str,
    info: Option<&'a str>,
    size: InputSize,
    data: &'a FormValue,
}

#[derive(Debug, Default)]
pub struct BoolFormValue {
    value: bool,
    errors: Vec<String>,
}

impl BoolFormValue {
    pub fn get_value(&self) -> bool {
        self.value
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error)
    }

    pub fn create_checkbox(
        &self,
        name: &str,
        label: &str,
        info: Option<&str>,
    ) -> Result<askama::filters::Safe<String>, askama::Error> {
        let template = CheckboxTemplate {
            name,
            label,
            info,
            data: self,
        };
        Ok(askama::filters::Safe(template.render()?))
    }
}

/// Custom serde Deserialize implementation for BoolFormValue:
/// We want to treat the value like an Option<()>: The value shall be `true` when the field is
/// present (with any value) and `false` if the field is not present.
///
/// We achive this by the custom simple Visitor implementation [BoolFormValueVisitor] that only
/// reacts to `visit_some()` and `visit_none()`
impl<'de> serde::Deserialize<'de> for BoolFormValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_option(BoolFormValueVisitor {})
    }
}

/// A very simple serde deserialization visitor for BoolFormValue:
/// It only implements [serde::de::Visitor::visit_some] and [serde::de::Visitor::visit_none] to
/// create a truthy BoolFormValue when the field is present in the input and a falsy BoolFormValue
/// when the field is not present at all.
struct BoolFormValueVisitor;

impl<'de> serde::de::Visitor<'de> for BoolFormValueVisitor {
    type Value = BoolFormValue;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("any value (true) or no such field at all")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(BoolFormValue {
            value: false,
            errors: vec![],
        })
    }

    fn visit_some<D>(self, _deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(BoolFormValue {
            value: true,
            errors: vec![],
        })
    }
}

impl From<bool> for BoolFormValue {
    fn from(value: bool) -> Self {
        Self {
            value,
            errors: vec![],
        }
    }
}

#[derive(Template)]
#[template(path = "forms/checkbox.html")]
struct CheckboxTemplate<'a> {
    name: &'a str,
    label: &'a str,
    info: Option<&'a str>,
    data: &'a BoolFormValue,
}
