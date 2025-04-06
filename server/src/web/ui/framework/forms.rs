use askama::Template;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, Deserialize, Default)]
#[serde(transparent)]
pub struct FormValue {
    value: String,
    #[serde(skip)]
    errors: Vec<String>,
}

impl FormValue {
    pub fn validate<'a, T: FromFormValue<'a>>(&'a mut self) -> Option<T> {
        match T::from_form_value(&mut self.value) {
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
        match T::from_form_value(&mut self.value, data) {
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
    SMALL,
    NORMAL,
    LARGE,
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

#[derive(Debug, Deserialize, Default)]
#[serde(transparent)]
pub struct BoolFormValue {
    // This is a bit hacky: In the end, we only want to store a boolean here (checked/not checked
    // aka. value is present in the encoded form data or not). However, we need to trick serde into
    // accepting both, a missing value or any string. Using the derive(Deserialize) macro with an
    // Option<String> field is the easiest way to do so.
    // TODO replace with custom Deserialize implementation
    value: Option<String>,
    #[serde(skip, default)]
    errors: Vec<String>,
}

impl BoolFormValue {
    pub fn get_value(&self) -> bool {
        self.value.is_some()
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

impl From<bool> for BoolFormValue {
    fn from(value: bool) -> Self {
        Self {
            value: if value {
                Some("true".to_string())
            } else {
                None
            },
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
