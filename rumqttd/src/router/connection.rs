use slab::Slab;

use crate::protocol::LastWillProperties;
use crate::{protocol::LastWill, Topic};
use crate::{Acl, Filter};
use std::collections::{HashMap, HashSet};

use super::ConnectionEvents;

pub(crate) const TENANTS_PREFIX: &'static str = "/tenants/";

/// Used to register a new connection with the router
/// Connection messages encompasses a handle for router to
/// communicate with this connection
#[derive(Debug)]
pub struct Connection {
    pub client_id: String,
    /// Id of client's organisation/tenant
    pub tenant_id: Option<String>,
    /// Username after successful authentication
    pub username: Option<String>,
    /// Dynamically create subscription filters incase they didn't exist during a publish
    pub dynamic_filters: bool,
    /// ACLs with substitued variables for this connection
    pub acls: Vec<Acl>,
    /// Clean session
    pub clean: bool,
    /// Subscriptions
    pub subscriptions: HashSet<Filter>,
    /// Last will of this connection
    pub last_will: Option<LastWill>,
    /// Properties of Last will
    pub last_will_properties: Option<LastWillProperties>,
    /// Connection events
    pub events: ConnectionEvents,
    /// MQTT version
    pub(crate) protocol_level: u8,
    /// Topic aliases set by clients
    pub(crate) topic_aliases: HashMap<u16, Topic>,
    /// Topic aliases used by broker
    pub(crate) broker_topic_aliases: Option<BrokerAliases>,
    /// subscription IDs for a connection
    pub(crate) subscription_ids: HashMap<Filter, usize>,
}

impl Connection {
    /// Create connection state to hold identifying information of connecting device
    pub fn new(
        tenant_id: Option<String>,
        username: Option<String>,
        client_id: String,
        clean: bool,
        dynamic_filters: bool,
        acls: &[Acl],
    ) -> Connection {
        // Change client id to -> tenant_id.client_id and derive topic path prefix
        // to validate topics
        let client_id = if let Some(ref tenant_id) = tenant_id {
            let client_id = tenant_id.to_owned() + "." + &client_id;
            client_id
        } else {
            client_id
        };

        let tenant_id_var = tenant_id
            .as_ref()
            .map(|tenant_id| ("%t", tenant_id.as_str()));
        let username_var = username.as_ref().map(|username| ("%u", username.as_str()));
        let variables = [
            Some(("%c", client_id.as_str())),
            tenant_id_var,
            username_var,
        ];
        let acls = acls
            .into_iter()
            .map(|acl| acl.substitute_variables(variables.into_iter().filter_map(|var| var)))
            .collect();

        Connection {
            client_id,
            tenant_id,
            username,
            dynamic_filters,
            clean,
            subscriptions: HashSet::default(),
            last_will: None,
            last_will_properties: None,
            events: ConnectionEvents::default(),
            topic_aliases: HashMap::new(),
            broker_topic_aliases: None,
            subscription_ids: HashMap::new(),
            acls,
        }
    }

    pub fn topic_alias_max(&mut self, max: u16) -> &mut Connection {
        // if topic_alias_max is 0, that means client doesn't want to use / support topic alias
        if max > 0 {
            self.broker_topic_aliases = Some(BrokerAliases::new(max));
        }
        self
    }

    pub fn last_will(
        &mut self,
        will: Option<LastWill>,
        props: Option<LastWillProperties>,
    ) -> &mut Connection {
        self.last_will = will;
        self.last_will_properties = props;
        self
    }
}

#[derive(Debug)]
pub(crate) struct BrokerAliases {
    pub(crate) broker_topic_aliases: HashMap<Filter, u16>,
    pub(crate) used_aliases: Slab<()>,
    pub(crate) topic_alias_max: u16,
}

impl BrokerAliases {
    fn new(topic_alias_max: u16) -> BrokerAliases {
        let mut used_aliases = Slab::new();
        // occupy 0th index as 0 is invalid topic alias
        assert_eq!(0, used_aliases.insert(()));

        let broker_topic_aliases = HashMap::new();

        BrokerAliases {
            broker_topic_aliases,
            used_aliases,
            topic_alias_max,
        }
    }

    // unset / remove the alias for topic
    pub fn remove_alias(&mut self, topic: &str) {
        if let Some(alias) = self.broker_topic_aliases.remove(topic) {
            self.used_aliases.remove(alias as usize);
        }
    }

    // Get alias used for the topic, if it exists
    pub fn get_alias(&self, topic: &str) -> Option<u16> {
        self.broker_topic_aliases.get(topic).copied()
    }

    // Set new alias for a topic and return the alias
    // returns None if can't set new alias
    pub fn set_new_alias(&mut self, topic: &str) -> Option<u16> {
        let alias_to_use = self.used_aliases.insert(());

        // NOTE: maybe we can use self.used_aliases.len()
        // to check for availability of alias
        if alias_to_use > self.topic_alias_max as usize {
            self.used_aliases.remove(alias_to_use);
            return None;
        }

        let alias_to_use = alias_to_use as u16;
        self.broker_topic_aliases
            .insert(topic.to_owned(), alias_to_use);
        Some(alias_to_use)
    }
}
