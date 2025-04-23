//! This module provides the `FormValue` helper types that allow creating HTML form input fields
//! with pre-filled values and validation error messages.

use crate::web::ui::error::AppError;
use askama::Template;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct FormValue<T: FormValueRepresentation> {
    value: String,
    #[serde(skip)]
    errors: Vec<String>,
    #[serde(skip)]
    _phantom: PhantomData<T>,
}

/// Implemented by types that can be used as an HTML form string value
///
/// In general this includes two functionalities:
/// * converting the type into a string for form value representation ([into_form_value_string]),
///   **and**
/// * validating a submitted form input string and converting it to this type.
///
/// Validation can either be implemented via the [ValidateFromFormInput] trait (when no additional
/// data is required for validating/converting a value) or by implementing
/// [ValidateDataForFormValue] for one or more additional data type.
pub trait FormValueRepresentation: Debug {
    fn into_form_value_string(self) -> String;
}

/// Trait for [FormValueRepresentation]-implementing types that can be validated and converted
/// directly from  their form string representation, without additional data.
///
/// This allows validating (and converting) the value of a [FormValue] of this type by calling
/// `form_value.validate()` (implemented in [_FormValidSimpleValidate::validate] trait).
pub trait ValidateFromFormInput: FormValueRepresentation + Sized {
    fn from_form_value(value: &'_ str) -> Result<Self, String>;
}

/// Allow validating/converting the [FormValueRepresentation] type `R` with the help of this type.
///
/// Every type `D` implementing this trait, can be used as additional validation data for validating
/// (and converting) the value of a [FormValue] of type `R` via the [FormValue::validate_with]
/// function.
pub trait ValidationDataForFormValue<R: FormValueRepresentation> {
    fn validate_form_value(self, value: &'_ str) -> Result<R, String>;
}

impl FormValueRepresentation for String {
    fn into_form_value_string(self) -> String {
        self
    }
}

impl ValidateFromFormInput for String {
    fn from_form_value(value: &'_ str) -> Result<Self, String> {
        Ok(value.to_owned())
    }
}

impl<T: FormValueRepresentation> FormValue<T> {
    pub fn validate_with<'d, D: ValidationDataForFormValue<T> + 'd>(
        &'_ mut self,
        data: D,
    ) -> Option<T> {
        match data.validate_form_value(&self.value) {
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

impl<T: FormValueRepresentation> Default for FormValue<T>
where
    T: Default,
{
    fn default() -> Self {
        FormValue {
            value: T::default().into_form_value_string(),
            errors: vec![],
            _phantom: Default::default(),
        }
    }
}

impl<T: FormValueRepresentation> From<T> for FormValue<T> {
    fn from(value: T) -> Self {
        FormValue {
            value: value.into_form_value_string(),
            errors: vec![],
            _phantom: Default::default(),
        }
    }
}

/// Helper trait with a simplified version of the [FormValue::validate] method that is added to the
/// [FormValue] type when the data type `T` does not have associated validation data (i.e. the
/// [FormValueRepresentation::ValidationData] type is the unit type (`()`)),
pub trait _FormValidSimpleValidate<T> {
    fn validate(&mut self) -> Option<T>;
}

impl<T: ValidateFromFormInput> _FormValidSimpleValidate<T> for FormValue<T> {
    fn validate(&mut self) -> Option<T> {
        match T::from_form_value(&self.value) {
            Ok(v) => Some(v),
            Err(e) => {
                self.errors.push(e);
                None
            }
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
struct FormFieldTemplate<'a, T: FormValueRepresentation> {
    name: &'a str,
    label: &'a str,
    info: Option<&'a str>,
    input_type: InputType,
    size: InputSize,
    data: &'a FormValue<T>,
}

#[derive(Template)]
#[template(path = "forms/hidden_input.html")]
struct HiddenInputTemplate<'a, T: FormValueRepresentation> {
    name: &'a str,
    data: &'a FormValue<T>,
}

#[derive(Serialize)]
pub struct SelectEntry<'a> {
    pub value: Cow<'a, str>,
    pub text: Cow<'a, str>,
}

#[derive(Template)]
#[template(path = "forms/select.html")]
struct SelectTemplate<'a, T: FormValueRepresentation> {
    name: &'a str,
    entries: &'a Vec<SelectEntry<'a>>,
    label: &'a str,
    info: Option<&'a str>,
    size: InputSize,
    data: &'a FormValue<T>,
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
