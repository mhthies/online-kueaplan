//! This module provides the `FormValue` helper types that encapsulate string values and validation
//! error messages for rendering HTML form input fields and validating the corresponding user input.

use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct FormValue<T: FormValueRepresentation> {
    value: Option<String>,
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

impl FormValueRepresentation for Uuid {
    fn into_form_value_string(self) -> String {
        self.to_string()
    }
}
impl ValidateFromFormInput for Uuid {
    fn from_form_value(value: &str) -> Result<Self, String> {
        Ok(Uuid::parse_str(value).map_err(|e| e.to_string())?)
    }
}

impl<T: FormValueRepresentation> FormValue<T> {
    /// Create a FormValue without contained value. This will cause an error when trying to validate
    /// it or call [string_value()].
    pub fn empty() -> Self {
        Self {
            value: None,
            errors: vec![],
            _phantom: Default::default(),
        }
    }

    pub fn validate_with<'d, D: ValidationDataForFormValue<T> + 'd>(
        &'_ mut self,
        data: D,
    ) -> Option<T> {
        if let Some(value) = &self.value {
            match data.validate_form_value(value) {
                Ok(v) => Some(v),
                Err(e) => {
                    self.errors.push(e);
                    None
                }
            }
        } else {
            self.errors.push("Wert fehlt in Formular-Daten".to_owned());
            None
        }
    }

    /// Manually add a validation error related to this form field.
    ///
    /// This can be used to attach error messages to a specific input field to inform the user about
    /// higher-level validation errors that were found when checking the consistency of the overall
    /// form/entity.
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error)
    }

    /// Check if validation errors have occurred, related to this form value.
    ///
    /// This should only be used by form input sub-templates for changing the rendering of a form
    /// input (like a text input) representing this form value.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the list of validation errors related to this form value.
    ///
    /// This should only be used by form input sub-templates for rendering the validation errors
    /// near to the input representing this form value.
    pub fn errors(&self) -> &Vec<String> {
        &self.errors
    }

    /// Get the current string representation of the form value to be used as the `value` attribute
    /// when rendering the form input.
    pub fn string_value(&self) -> &str {
        self.value.as_deref().unwrap_or("")
    }
}

impl<T: FormValueRepresentation> Default for FormValue<T>
where
    T: Default,
{
    fn default() -> Self {
        FormValue {
            value: Some(T::default().into_form_value_string()),
            errors: vec![],
            _phantom: Default::default(),
        }
    }
}

impl<T: FormValueRepresentation> From<T> for FormValue<T> {
    fn from(value: T) -> Self {
        FormValue {
            value: Some(value.into_form_value_string()),
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
        if let Some(value) = &self.value {
            match T::from_form_value(value) {
                Ok(v) => Some(v),
                Err(e) => {
                    self.errors.push(e);
                    None
                }
            }
        } else {
            self.errors.push("Wert fehlt in Formular-Daten".to_owned());
            None
        }
    }
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

    /// Check if validation errors have occurred, related to this form value.
    ///
    /// This should only be used by form input sub-templates for changing the rendering of a form
    /// input (like a checkbox) representing this form value.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the list of validation errors related to this form value.
    ///
    /// This should only be used by form input sub-templates for rendering the validation errors
    /// near to the input representing this form value.
    pub fn errors(&self) -> &Vec<String> {
        &self.errors
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
