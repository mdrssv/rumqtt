use std::{borrow::Cow, fmt::Display, str::FromStr};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const TOPIC_SEP: &'static str = "/";
const TOPIC_WILDCARD: &'static str = "#";
const TOPIC_ANY: &'static str = "+";

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Acl {
    /// Rule, describing which topic this ACL applies to
    pub rule: AclRule,
    /// Indicates whether the topic in question can be subscribed to
    pub read: bool,
    /// Indicated whether to topic in question can be published to
    pub write: bool,
}

impl Acl {
    /// Creates an new `Acl` from an given rule.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumqttd::Acl;
    /// let acl = Acl::new("test/#", true, false);
    /// ```
    ///
    /// From string
    /// ```
    /// use rumqttd::Acl;
    /// let acl: Acl = "test/#:rw".parse().unwrap();
    /// # assert_eq!(acl, Acl { rule: "test/#".into(), read: true, write: true });
    /// # assert_eq!("test/#".parse::<Acl>(), Err(AclError::NoFlags));
    /// ```
    pub fn new(rule: impl Into<AclRule>, read: bool, write: bool) -> Self {
        Self {
            rule: rule.into(),
            read,
            write,
        }
    }

    #[doc(alias = "AclRule::substitute_variables")]
    pub fn substitute_variables<'a, V: IntoIterator<Item = (&'a str, S)>, S: AsRef<str>>(
        &self,
        variables: V,
    ) -> Self {
        let rule = self.rule.substitute_variables(variables);
        Self {
            rule,
            ..self.clone()
        }
    }
}

impl Display for Acl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let flag = |flag, char| {
            if flag {
                char
            } else {
                ""
            }
        };
        write!(
            f,
            "{}:{}{}",
            self.rule,
            flag(self.read, "r"),
            flag(self.write, "w")
        )
    }
}

#[derive(Debug, Eq, PartialEq, Error)]
pub enum AclError {
    #[error("acl does not contain an ':'")]
    NoFlags,
}

impl FromStr for Acl {
    type Err = AclError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let last_colon = s.rfind(":").ok_or(AclError::NoFlags)?;
        let rule = &s[..last_colon];
        let flags = &s[last_colon..][1..];
        Ok(Self {
            rule: rule.to_owned().into(),
            read: flags.contains("r"),
            write: flags.contains("w"),
        })
    }
}

impl TryFrom<&str> for Acl {
    type Error = <Self as FromStr>::Err;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl TryFrom<String> for Acl {
    type Error = <Self as FromStr>::Err;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl Into<String> for Acl {
    fn into(self) -> String {
        self.to_string()
    }
}

/// Represents an Access Control List (ACL) rule.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AclRule(Cow<'static, str>);

impl Default for AclRule {
    fn default() -> Self {
        Self(TOPIC_WILDCARD.into())
    }
}

impl From<&'static str> for AclRule {
    /// Creates an `AclRule` from a static string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumqttd::AclRule;
    /// let rule = AclRule::from("test/#");
    /// ```
    fn from(value: &'static str) -> Self {
        Self(Cow::Borrowed(value))
    }
}

impl From<String> for AclRule {
    /// Creates an `AclRule` from a `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumqttd::AclRule;
    /// let rule = AclRule::from(String::from("test/#"));
    /// ```
    fn from(value: String) -> Self {
        Self(Cow::Owned(value))
    }
}

impl Display for AclRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl AsRef<str> for AclRule {
    /// Returns a reference to the string slice of the `AclRule`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumqttd::AclRule;
    /// let rule = AclRule::from("test/#");
    /// assert_eq!(rule.as_ref(), "test/#");
    /// ```
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl AclRule {
    fn matches(&self, path: &str, _filter: bool) -> bool {
        for (tc, rc) in path
            .split(TOPIC_SEP)
            .map(Some)
            .chain([None])
            .zip(self.0.as_ref().split(TOPIC_SEP).map(Some).chain([None]))
        {
            match (tc, rc) {
                (Some(_), Some(TOPIC_WILDCARD)) => return true,
                (Some(_), Some(TOPIC_ANY)) => continue,
                (tc, rc) if tc == rc => continue,
                _ => return false,
            }
        }
        true
    }

    /// Checks if the ACL rule matches a given topic.
    ///
    /// # Parameters
    ///
    /// - `topic`: The topic to match against.
    ///
    /// # Returns
    ///
    /// `true` if the topic matches the ACL rule, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumqttd::AclRule;
    /// let rule = AclRule::from("test/#");
    /// assert!(rule.matches_topic("test/abc"));
    /// assert!(rule.matches_topic("test/abc/def"));
    ///
    /// let rule = AclRule::from("test/+");
    /// assert!(rule.matches_topic("test/abc"));
    /// assert!(!rule.matches_topic("test/abc/def"));
    /// ```
    pub fn matches_topic(&self, topic: impl AsRef<str>) -> bool {
        self.matches(topic.as_ref(), false)
    }

    /// Checks if the ACL rule matches a given filter.
    ///
    /// # Parameters
    ///
    /// - `filter`: The filter to match against.
    ///
    /// # Returns
    ///
    /// `true` if the filter matches the ACL rule, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumqttd::AclRule;
    /// let rule = AclRule::from("test/#");
    /// assert!(rule.matches_filter("test/abc"));
    /// assert!(rule.matches_filter("test/abc/def"));
    ///
    /// let rule = AclRule::from("test/+");
    /// assert!(rule.matches_filter("test/abc"));
    /// assert!(!rule.matches_filter("test/abc/def"));
    /// ```
    pub fn matches_filter(&self, filter: impl AsRef<str>) -> bool {
        self.matches(filter.as_ref(), true)
    }

    /// Substitutes variables in the ACL rule with provided values.
    ///
    /// This method clones the current instance and then iterates over the provided variables,
    /// replacing occurrences of each variable name with its corresponding value. If the value
    /// contains any illegal characters (`TOPIC_SEP`, `TOPIC_WILDCARD`, `TOPIC_ANY`), that
    /// substitution is skipped.
    ///
    /// # Type Parameters
    ///
    /// - `'a`: Lifetime parameter for the variable names.
    /// - `V`: An iterator of items, where each item is a tuple of `(&'a str, S)`.
    /// - `S`: A type that can be referenced as a string slice (`&str`).
    ///
    /// # Parameters
    ///
    /// - `variables`: An iterator of key-value pairs representing the variables and their corresponding
    ///   substitution values.
    ///
    /// # Returns
    ///
    /// A new instance of `Self` with the variables substituted.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumqttd::AclRule;
    /// let rule = AclRule::from("device/%u/version/+");
    /// let user = "0xff";
    /// assert_eq!(rule.substitute_variables([("%u", user)]).as_ref(), "device/0xff/version/+");
    /// assert_eq!(rule.substitute_variables([("%u", "client1/a")]).as_ref(), "device/%u/version/+");
    /// ```
    ///
    /// # Note
    ///
    /// This function skips any substitution where the replacement value contains characters defined
    /// by `TOPIC_SEP`, `TOPIC_WILDCARD`, or `TOPIC_ANY`.
    pub fn substitute_variables<'a, V: IntoIterator<Item = (&'a str, S)>, S: AsRef<str>>(
        &self,
        variables: V,
    ) -> Self {
        let mut substituted = self.clone();
        for (name, value) in variables {
            let value = value.as_ref();
            if [TOPIC_SEP, TOPIC_WILDCARD, TOPIC_ANY]
                .into_iter()
                .any(|illegal| value.contains(illegal))
            {
                continue;
            }
            if self.as_ref().contains(name) {
                substituted.0 = Cow::Owned(substituted.as_ref().replace(name, value));
            }
        }
        substituted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_topic_wildcard() {
        let rule = AclRule::from("test/#");
        assert!(rule.matches_topic("test/abc"));
        assert!(rule.matches_topic("test/abc/def"));
    }
    #[test]
    fn matches_topic_any() {
        let rule = AclRule::from("test/+");
        assert!(rule.matches_topic("test/abc"));
        assert!(!rule.matches_topic("test/abc/def"));
        let rule = AclRule::from("test/+/sub/+");
        assert!(rule.matches_topic("test/abc/sub/def"));
        assert!(!rule.matches_topic("test/abc/bub/def"));
    }

    #[test]
    fn matches_filter_wildcard() {
        let rule = AclRule::from("test/#");
        assert!(rule.matches_filter("test/abc"));
        assert!(rule.matches_filter("test/abc/def"));
        assert!(!rule.matches_filter("#"));
    }
    #[test]
    fn matches_filter_any() {
        let rule = AclRule::from("test/+");
        assert!(rule.matches_filter("test/abc"));
        assert!(!rule.matches_filter("test/abc/def"));
        let rule = AclRule::from("test/+/sub/+");
        assert!(rule.matches_filter("test/+/sub/def"));
        assert!(!rule.matches_filter("test/abc/+/def"));
    }

    #[test]
    fn substitute() {
        let rule = AclRule::from("device/%u/version/+");
        let user = "0xff";
        assert_eq!(
            rule.substitute_variables([("%u", user)]).as_ref(),
            *&format!("device/{user}/version/+")
        );
        assert_eq!(
            rule.substitute_variables([("%u", "client1/a")]).as_ref(),
            "device/%u/version/+"
        );
    }

    #[test]
    fn substitute_fail_does_not_allocate() {
        let rule = AclRule::from("device/%u/version/+");
        assert!(matches!(rule.0, Cow::Borrowed(_)));
        let id = "8e7798ed-cf5e-472d-93aa-c7e794bd6aaa";
        assert!(matches!(
            rule.substitute_variables([("%c", id)]).0,
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn string_parse() {
        let rule: Acl = "test/+:r".parse().unwrap();
        assert_eq!(rule.to_string().parse::<Acl>().unwrap(), rule);
    }
}
