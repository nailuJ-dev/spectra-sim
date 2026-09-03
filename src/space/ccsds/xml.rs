use std::collections::BTreeMap;

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use quick_xml::XmlVersion;

use crate::{Result, SimError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlNode {
    pub name: String,
    pub attributes: BTreeMap<String, String>,
    pub text: String,
    pub children: Vec<XmlNode>,
}

impl XmlNode {
    pub fn child(&self, name: &str) -> Option<&XmlNode> {
        self.children
            .iter()
            .find(|child| child.name.eq_ignore_ascii_case(name))
    }

    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a XmlNode> {
        self.children
            .iter()
            .filter(move |child| child.name.eq_ignore_ascii_case(name))
    }

    pub fn descendant(&self, name: &str) -> Option<&XmlNode> {
        if self.name.eq_ignore_ascii_case(name) {
            return Some(self);
        }
        self.children
            .iter()
            .find_map(|child| child.descendant(name))
    }

    pub fn descendants<'a>(&'a self, name: &'a str, output: &mut Vec<&'a XmlNode>) {
        if self.name.eq_ignore_ascii_case(name) {
            output.push(self);
        }
        for child in &self.children {
            child.descendants(name, output);
        }
    }

    pub fn collect_leaf_values(&self, output: &mut BTreeMap<String, String>) {
        if self.children.is_empty() && !self.text.trim().is_empty() {
            output.insert(self.name.to_ascii_uppercase(), self.text.trim().to_string());
        }
        for child in &self.children {
            child.collect_leaf_values(output);
        }
    }

    pub fn leaf_map(&self) -> BTreeMap<String, String> {
        let mut result = BTreeMap::new();
        self.collect_leaf_values(&mut result);
        result
    }
}

pub fn parse_xml_tree(input: &str) -> Result<XmlNode> {
    if input.len() > 32 * 1024 * 1024 {
        return Err(SimError::DimensionLimit {
            actual: input.len(),
            max: 32 * 1024 * 1024,
        });
    }
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);
    reader.config_mut().expand_empty_elements = true;
    let mut stack: Vec<XmlNode> = Vec::new();
    let mut root: Option<XmlNode> = None;
    let mut buf = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|error| SimError::InvalidArgument(format!("invalid CCSDS XML: {error}")))?
        {
            Event::Start(start) => {
                let name = String::from_utf8_lossy(start.local_name().as_ref()).into_owned();
                let mut attributes = BTreeMap::new();
                for attr in start.attributes().with_checks(true) {
                    let attr = attr.map_err(|error| {
                        SimError::InvalidArgument(format!("invalid CCSDS XML attribute: {error}"))
                    })?;
                    let key = String::from_utf8_lossy(attr.key.local_name().as_ref()).into_owned();
                    let value = attr
                        .decoded_and_normalized_value(XmlVersion::Explicit1_0, reader.decoder())
                        .map_err(|error| {
                            SimError::InvalidArgument(format!(
                                "invalid CCSDS XML attribute value: {error}"
                            ))
                        })?
                        .into_owned();
                    attributes.insert(key, value);
                }
                stack.push(XmlNode {
                    name,
                    attributes,
                    text: String::new(),
                    children: Vec::new(),
                });
            }
            Event::Text(text) => {
                if let Some(current) = stack.last_mut() {
                    let decoded = text.decode().map_err(|error| {
                        SimError::InvalidArgument(format!("invalid CCSDS XML text: {error}"))
                    })?;
                    current.text.push_str(&decoded);
                }
            }
            Event::CData(text) => {
                if let Some(current) = stack.last_mut() {
                    let decoded = text.decode().map_err(|error| {
                        SimError::InvalidArgument(format!("invalid CCSDS XML CDATA: {error}"))
                    })?;
                    current.text.push_str(&decoded);
                }
            }
            Event::GeneralRef(reference) => {
                if let Some(current) = stack.last_mut() {
                    let name = String::from_utf8_lossy(reference.as_ref());
                    let resolved = match name.as_ref() {
                        "amp" => "&",
                        "lt" => "<",
                        "gt" => ">",
                        "quot" => "\"",
                        "apos" => "'",
                        _ => {
                            return Err(SimError::InvalidArgument(format!(
                                "unsupported XML entity &{name};"
                            )))
                        }
                    };
                    current.text.push_str(resolved);
                }
            }
            Event::End(_) => {
                let node = stack.pop().ok_or_else(|| {
                    SimError::InvalidArgument("unbalanced CCSDS XML end tag".into())
                })?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else if root.replace(node).is_some() {
                    return Err(SimError::InvalidArgument(
                        "CCSDS XML contains multiple roots".into(),
                    ));
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    if !stack.is_empty() {
        return Err(SimError::InvalidArgument(
            "CCSDS XML ended with unclosed elements".into(),
        ));
    }
    root.ok_or_else(|| SimError::InvalidArgument("CCSDS XML is empty".into()))
}
