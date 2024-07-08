use std::borrow::Cow;

const TOPIC_SEP: &'static str = "/";
const TOPIC_WILDCARD: &'static str = "#";
const TOPIC_ANY: &'static str = "+";

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
}
