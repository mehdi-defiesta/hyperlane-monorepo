use anyhow::{Result, Context};
use ethers::prelude::*;
use hyperlane_core::H256;
use serde::{Deserialize, Deserializer, Serialize, de::Error, de::Visitor};
use std::fmt::{self, Display, Formatter};

/// A filter that can match against wildcard "*", single values, or multiple values
#[derive(Debug, Clone, PartialEq)]
pub enum Filter<T> {
    Wildcard,
    Single(T),
    Multiple(Vec<T>),
}

impl<T> Default for Filter<T> {
    fn default() -> Self {
        Self::Wildcard
    }
}

impl<T: PartialEq> Filter<T> {
    pub fn matches(&self, value: &T) -> bool {
        match self {
            Filter::Wildcard => true, // Wildcard always matches
            Filter::Single(v) => v == value,
            Filter::Multiple(list) => list.iter().any(|v| v == value),
        }
    }
}

impl<T: Display> Display for Filter<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Filter::Wildcard => write!(f, "*"),
            Filter::Single(v) => write!(f, "{}", v),
            Filter::Multiple(list) => {
                write!(f, "[")?;
                for (i, v) in list.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
        }
    }
}

// Serialize implementations for specific types
impl Serialize for Filter<u32> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Filter::Wildcard => serializer.serialize_str("*"),
            Filter::Single(v) => v.serialize(serializer),
            Filter::Multiple(list) => list.serialize(serializer),
        }
    }
}

impl Serialize for Filter<String> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Filter::Wildcard => serializer.serialize_str("*"),
            Filter::Single(v) => v.serialize(serializer),
            Filter::Multiple(list) => list.serialize(serializer),
        }
    }
}

// Custom deserializer for Filter<u32>
impl<'de> Deserialize<'de> for Filter<u32> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FilterVisitor;
        
        impl<'de> Visitor<'de> for FilterVisitor {
            type Value = Filter<u32>;
            
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string '*', number, or array of numbers")
            }
            
            fn visit_str<E>(self, value: &str) -> Result<Filter<u32>, E>
            where
                E: Error,
            {
                if value == "*" {
                    Ok(Filter::Wildcard)
                } else {
                    let num = value.parse::<u32>().map_err(E::custom)?;
                    Ok(Filter::Single(num))
                }
            }
            
            fn visit_u32<E>(self, value: u32) -> Result<Filter<u32>, E>
            where
                E: Error,
            {
                Ok(Filter::Single(value))
            }
            
            fn visit_u64<E>(self, value: u64) -> Result<Filter<u32>, E>
            where
                E: Error,
            {
                if value <= u32::MAX as u64 {
                    Ok(Filter::Single(value as u32))
                } else {
                    Err(E::custom("Number too large for u32"))
                }
            }
            
            fn visit_seq<A>(self, mut seq: A) -> Result<Filter<u32>, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<u32>()? {
                    values.push(value);
                }
                Ok(Filter::Multiple(values))
            }
        }
        
        deserializer.deserialize_any(FilterVisitor)
    }
}

// Custom deserializer for Filter<String>
impl<'de> Deserialize<'de> for Filter<String> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FilterVisitor;
        
        impl<'de> Visitor<'de> for FilterVisitor {
            type Value = Filter<String>;
            
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string '*', string, or array of strings")
            }
            
            fn visit_str<E>(self, value: &str) -> Result<Filter<String>, E>
            where
                E: Error,
            {
                if value == "*" {
                    Ok(Filter::Wildcard)
                } else {
                    Ok(Filter::Single(value.to_string()))
                }
            }
            
            fn visit_seq<A>(self, mut seq: A) -> Result<Filter<String>, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<String>()? {
                    values.push(value);
                }
                Ok(Filter::Multiple(values))
            }
        }
        
        deserializer.deserialize_any(FilterVisitor)
    }
}

/// A single matching rule that can filter messages based on origin domain, sender address,
/// destination domain, and recipient address
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MatchingRule {
    #[serde(default, rename = "originDomain")]
    pub origin_domain: Filter<u32>,
    
    #[serde(default, rename = "senderAddress")]  
    pub sender_address: Filter<String>,
    
    #[serde(default, rename = "destinationDomain")]
    pub destination_domain: Filter<u32>,
    
    #[serde(default, rename = "recipientAddress")]
    pub recipient_address: Filter<String>,
}

impl Display for MatchingRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{originDomain: {}, senderAddress: {}, destinationDomain: {}, recipientAddress: {}}}",
            self.origin_domain,
            self.sender_address,
            self.destination_domain,
            self.recipient_address
        )
    }
}

/// A list of matching rules for filtering Hyperlane messages
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MatchingList {
    pub rules: Vec<MatchingRule>,
}

impl MatchingList {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn from_json(json_str: &str) -> Result<Self> {
        let rules: Vec<MatchingRule> = serde_json::from_str(json_str).context("Failed to parse MatchingList JSON")?;
        Ok(MatchingList { rules })
    }

    /// Check if a message matches any of the rules
    pub fn matches_message(
        &self,
        origin_domain: u32,
        sender_address: &Address,
        destination_domain: u32,
        recipient_address: &H256,
    ) -> bool {
        if self.rules.is_empty() {
            return true; // Empty list matches everything
        }

        let sender_str = format!("{:#x}", sender_address); // Use lowercase hex with 0x
        let recipient_str = format!("{:#x}", recipient_address);

        self.rules.iter().any(|rule| {
            rule.origin_domain.matches(&origin_domain)
                && rule.sender_address.matches(&sender_str)
                && rule.destination_domain.matches(&destination_domain) 
                && rule.recipient_address.matches(&recipient_str)
        })
    }

    /// Create a simple rule that matches specific values
    pub fn create_simple_rule(
        origin_domain: Option<u32>,
        sender_address: Option<&str>,
        destination_domain: Option<u32>,
        recipient_address: Option<&str>,
    ) -> MatchingRule {
        MatchingRule {
            origin_domain: match origin_domain {
                Some(d) => Filter::Single(d),
                None => Filter::Wildcard,
            },
            sender_address: match sender_address {
                Some(a) => Filter::Single(a.to_string()),
                None => Filter::Wildcard,
            },
            destination_domain: match destination_domain {
                Some(d) => Filter::Single(d),
                None => Filter::Wildcard,
            },
            recipient_address: match recipient_address {
                Some(a) => Filter::Single(a.to_string()),
                None => Filter::Wildcard,
            },
        }
    }

    pub fn add_rule(&mut self, rule: MatchingRule) {
        self.rules.push(rule);
    }
}

impl Display for MatchingList {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.rules.is_empty() {
            write!(f, "[]")
        } else {
            write!(f, "[")?;
            for (i, rule) in self.rules.iter().enumerate() {
                if i > 0 { write!(f, ", ")?; }
                write!(f, "{}", rule)?;
            }
            write!(f, "]")
        }
    }
}

impl Default for MatchingList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::types::Address;

    #[test]
    fn test_matching_list_creation() {
        let mut list = MatchingList::new();
        assert!(list.rules.is_empty());

        let rule = MatchingRule {
            origin_domain: Filter::Single(1),
            sender_address: Filter::Wildcard("*".to_string()),
            destination_domain: Filter::Single(2),
            recipient_address: Filter::Wildcard("*".to_string()),
        };

        list.add_rule(rule);
        assert_eq!(list.rules.len(), 1);
    }

    #[test]
    fn test_filter_matching() {
        let wildcard: Filter<u32> = Filter::Wildcard("*".to_string());
        let single = Filter::Single(42u32);
        let multiple = Filter::Multiple(vec![1u32, 2u32, 3u32]);

        assert!(wildcard.matches(&100));
        assert!(single.matches(&42));
        assert!(!single.matches(&43));
        assert!(multiple.matches(&2));
        assert!(!multiple.matches(&4));
    }

    #[test]
    fn test_json_parsing() {
        let json = r#"[
            {
                "originDomain": 1,
                "senderAddress": "*",
                "destinationDomain": 2,
                "recipientAddress": "0x1234567890123456789012345678901234567890"
            }
        ]"#;

        let list: Vec<MatchingRule> = serde_json::from_str(json).unwrap();
        assert_eq!(list.len(), 1);
        
        let rule = &list[0];
        assert_eq!(rule.origin_domain, Filter::Single(1));
        assert_eq!(rule.destination_domain, Filter::Single(2));
        assert_eq!(rule.sender_address, Filter::Wildcard("*".to_string()));
    }
}