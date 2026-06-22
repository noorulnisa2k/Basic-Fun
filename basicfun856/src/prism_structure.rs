use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum Agency {
    X12,
    EDIFACT,
    ERROR,
    Unknown(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Output<'a> {
    #[serde(borrow)]
    pub company: Cow<'a, str>,

    #[serde(rename = "Direction", borrow, skip_serializing_if = "Option::is_none")]
    pub direction: Option<Cow<'a, str>>,

    #[serde(rename = "ProcessID", borrow)]
    pub process_id: Cow<'a, str>,

    #[serde(rename = "PlantName", borrow, skip_serializing_if = "Option::is_none")]
    pub plant_name: Option<Cow<'a, str>>,

    #[serde(
        rename = "RawFileName",
        borrow,
        skip_serializing_if = "Option::is_none"
    )]
    pub raw_file_name: Option<Cow<'a, str>>,

    #[serde(rename = "Status", borrow)]
    pub status: Cow<'a, str>,

    #[serde(
        rename = "EDIFileName",
        borrow,
        skip_serializing_if = "Option::is_none"
    )]
    pub edi_file_name: Option<Cow<'a, str>>,

    #[serde(
        rename = "ERPFileName",
        borrow,
        skip_serializing_if = "Option::is_none"
    )]
    pub erp_file_name: Option<Cow<'a, str>>,

    #[serde(
        rename = "ErrorFileName",
        borrow,
        skip_serializing_if = "Option::is_none"
    )]
    pub error_file_name: Option<Cow<'a, str>>,

    #[serde(rename = "ErrorType", borrow, skip_serializing_if = "Option::is_none")]
    pub error_type: Option<Cow<'a, str>>,

    #[serde(
        rename = "ErrorDescription",
        borrow,
        skip_serializing_if = "Option::is_none"
    )]
    pub error_description: Option<Cow<'a, str>>,

    #[serde(rename = "Reference", borrow, skip_serializing_if = "Option::is_none")]
    pub reference: Option<Cow<'a, str>>,

    #[serde(rename = "TransactionCode", skip_serializing_if = "Option::is_none")]
    pub transaction_code: Option<Cow<'a, str>>,

    #[serde(rename = "Agency")]
    pub agency: Agency,

    #[serde(rename = "B2BI_Timestamp", borrow)]
    pub b2bi_timestamp: Option<Cow<'a, str>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction<'a> {
    #[serde(default, rename = "ControlNumber", borrow)]
    control_number: Option<Cow<'a, str>>,

    #[serde(default, rename = "Type", borrow)]
    transaction_type: Cow<'a, str>,
}

// TODO MAKE OPTIONAL
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Delimiters<'a> {
    #[serde(rename = "ElementDelimiter", borrow)]
    element_delimiter: Option<Cow<'a, str>>,

    #[serde(rename = "SubElementDelimiter", borrow)]
    sub_element_delimiter: Option<Cow<'a, str>>,

    #[serde(rename = "SegmentDelimiter", borrow)]
    segment_delimiter: Option<Cow<'a, str>>,
}

impl FromStr for Agency {
    type Err = ParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_uppercase().as_str() {
            "X12" => Ok(Agency::X12),
            "EDIFACT" => Ok(Agency::EDIFACT),
            "ERROR" => Ok(Agency::ERROR),
            other => Ok(Agency::Unknown(other.to_string())),
        }
    }
}

impl Serialize for Agency {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Agency::X12 => serializer.serialize_str("X12"),
            Agency::EDIFACT => serializer.serialize_str("EDIFACT"),
            Agency::ERROR => serializer.serialize_str("ERROR"),
            Agency::Unknown(value) => serializer.serialize_str(value), // Serialize the stored unknown value
        }
    }
}

impl<'de> Deserialize<'de> for Agency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?; // Deserialize into a String
        Agency::from_str(&s).map_err(serde::de::Error::custom) // Use FromStr implementation
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    // #[error("Invalid enum variant: {0}")]
    // InvalidVariant(String),
    #[error(transparent)]
    SerdeXml(#[from] quick_xml::Error),
}

impl Transaction<'_> {
    fn is_empty(&self) -> bool {
        self.control_number.is_none() && self.transaction_type.is_empty()
    }
}
