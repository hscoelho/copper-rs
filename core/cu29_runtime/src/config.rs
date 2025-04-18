//! This module defines the configuration of the copper runtime.
//! The configuration is a directed graph where nodes are tasks and edges are connections between tasks.
//! The configuration is serialized in the RON format.
//! The configuration is used to generate the runtime code at compile time.

use cu29_traits::{CuError, CuResult};
use html_escape::encode_text;
use petgraph::adj::NodeIndex;
use petgraph::stable_graph::{EdgeIndex, StableDiGraph};
use petgraph::visit::EdgeRef;
use ron::extensions::Extensions;
use ron::value::Value as RonValue;
use ron::{Number, Options};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use std::fs::read_to_string;

/// NodeId is the unique identifier of a node in the configuration graph for petgraph
/// and the code generation.
pub type NodeId = u32;

/// This is the configuration of a component (like a task config or a monitoring config):w
/// It is a map of key-value pairs.
/// It is given to the new method of the task implementation.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ComponentConfig(pub HashMap<String, Value>);

impl Display for ComponentConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        let ComponentConfig(config) = self;
        write!(f, "{{")?;
        for (key, value) in config.iter() {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{key}: {value}")?;
            first = false;
        }
        write!(f, "}}")
    }
}

// forward map interface
impl ComponentConfig {
    #[allow(dead_code)]
    pub fn new() -> Self {
        ComponentConfig(HashMap::new())
    }

    #[allow(dead_code)]
    pub fn get<T: From<Value>>(&self, key: &str) -> Option<T> {
        let ComponentConfig(config) = self;
        config.get(key).map(|v| T::from(v.clone()))
    }

    #[allow(dead_code)]
    pub fn set<T: Into<Value>>(&mut self, key: &str, value: T) {
        let ComponentConfig(config) = self;
        config.insert(key.to_string(), value.into());
    }
}

// The configuration Serialization format is as follows:
// (
//   tasks : [ (id: "toto", type: "zorglub::MyType", config: {...}),
//             (id: "titi", type: "zorglub::MyType2", config: {...})]
//   cnx : [ (src: "toto", dst: "titi", msg: "zorglub::MyMsgType"),...]
// )

/// Wrapper around the ron::Value to allow for custom serialization.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Value(RonValue);

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value(RonValue::Number(value.into()))
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Value(RonValue::Number((value as u64).into()))
    }
}

impl From<u16> for Value {
    fn from(value: u16) -> Self {
        Value(RonValue::Number((value as u64).into()))
    }
}

impl From<u8> for Value {
    fn from(value: u8) -> Self {
        Value(RonValue::Number((value as u64).into()))
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value(RonValue::Number(value.into()))
    }
}

impl From<Value> for bool {
    fn from(value: Value) -> Self {
        if let Value(RonValue::Bool(v)) = value {
            v
        } else {
            panic!("Expected a Boolean variant but got {value:?}")
        }
    }
}
macro_rules! impl_from_value_for_int {
    ($($target:ty),* $(,)?) => {
        $(
            impl From<Value> for $target {
                fn from(value: Value) -> Self {
                    if let Value(RonValue::Number(num)) = value {
                        match num {
                            Number::I8(n) => n as $target,
                            Number::I16(n) => n as $target,
                            Number::I32(n) => n as $target,
                            Number::I64(n) => n as $target,
                            Number::U8(n) => n as $target,
                            Number::U16(n) => n as $target,
                            Number::U32(n) => n as $target,
                            Number::U64(n) => n as $target,
                            Number::F32(_) | Number::F64(_) => {
                                panic!("Expected an integer Number variant but got {num:?}")
                            }
                        }
                    } else {
                        panic!("Expected a Number variant but got {value:?}")
                    }
                }
            }
        )*
    };
}

impl_from_value_for_int!(u8, i8, u16, i16, u32, i32, u64, i64);

impl From<Value> for f64 {
    fn from(value: Value) -> Self {
        if let Value(RonValue::Number(num)) = value {
            num.into_f64()
        } else {
            panic!("Expected a Number variant but got {value:?}")
        }
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value(RonValue::String(value))
    }
}

impl From<Value> for String {
    fn from(value: Value) -> Self {
        if let Value(RonValue::String(s)) = value {
            s
        } else {
            panic!("Expected a String variant")
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Value(value) = self;
        match value {
            RonValue::Number(n) => {
                let s = match n {
                    Number::I8(n) => n.to_string(),
                    Number::I16(n) => n.to_string(),
                    Number::I32(n) => n.to_string(),
                    Number::I64(n) => n.to_string(),
                    Number::U8(n) => n.to_string(),
                    Number::U16(n) => n.to_string(),
                    Number::U32(n) => n.to_string(),
                    Number::U64(n) => n.to_string(),
                    Number::F32(n) => n.0.to_string(),
                    Number::F64(n) => n.0.to_string(),
                    _ => panic!("Expected a Number variant but got {value:?}"),
                };
                write!(f, "{s}")
            }
            RonValue::String(s) => write!(f, "{s}"),
            RonValue::Bool(b) => write!(f, "{b}"),
            RonValue::Map(m) => write!(f, "{m:?}"),
            RonValue::Char(c) => write!(f, "{c:?}"),
            RonValue::Unit => write!(f, "unit"),
            RonValue::Option(o) => write!(f, "{o:?}"),
            RonValue::Seq(s) => write!(f, "{s:?}"),
            RonValue::Bytes(bytes) => write!(f, "{bytes:?}"),
        }
    }
}

/// A node in the configuration graph.
/// A node represents a Task in the system Graph.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    id: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    type_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    config: Option<ComponentConfig>,
}

impl Node {
    #[allow(dead_code)]
    pub fn new(id: &str, ptype: &str) -> Self {
        Node {
            id: id.to_string(),
            type_: Some(ptype.to_string()),
            // base_period_ns: None,
            config: None,
        }
    }

    #[allow(dead_code)]
    pub fn get_id(&self) -> String {
        self.id.clone()
    }

    #[allow(dead_code)]
    pub fn set_type(mut self, name: Option<String>) -> Self {
        self.type_ = name;
        self
    }

    pub fn get_type(&self) -> &str {
        self.type_.as_ref().unwrap()
    }

    #[allow(dead_code)]
    pub fn get_instance_config(&self) -> Option<&ComponentConfig> {
        self.config.as_ref()
    }

    #[allow(dead_code)]
    pub fn get_param<T: From<Value>>(&self, key: &str) -> Option<T> {
        let pc = self.config.as_ref()?;
        let ComponentConfig(pc) = pc;
        let v = pc.get(key)?;
        Some(T::from(v.clone()))
    }

    #[allow(dead_code)]
    pub fn set_param<T: Into<Value>>(&mut self, key: &str, value: T) {
        if self.config.is_none() {
            self.config = Some(ComponentConfig(HashMap::new()));
        }
        let ComponentConfig(config) = self.config.as_mut().unwrap();
        config.insert(key.to_string(), value.into());
    }
}

/// This represents a connection between 2 tasks (nodes) in the configuration graph.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Cnx {
    /// Source node id.
    src: String,

    // Destination node id.
    dst: String,

    /// Message type exchanged between src and dst.
    pub msg: String,

    /// Tells Copper to batch messages before sending the buffer to the next node.
    /// If None, Copper will just send 1 message at a time.
    /// If Some(n), Copper will batch n messages before sending the buffer.
    pub batch: Option<u32>,

    /// Tells Copper if it needs to log the messages.
    pub store: Option<bool>,
}

/// CuConfig is the programmatic representation of the configuration graph.
/// It is a directed graph where nodes are tasks and edges are connections between tasks.
#[derive(Debug, Clone)]
pub struct CuConfig {
    // This is not what is directly serialized, see the custom serialization below.
    pub graph: StableDiGraph<Node, Cnx, NodeId>,
    pub monitor: Option<MonitorConfig>,
    pub logging: Option<LoggingConfig>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct MonitorConfig {
    #[serde(rename = "type")]
    type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    config: Option<ComponentConfig>,
}

impl MonitorConfig {
    #[allow(dead_code)]
    pub fn get_type(&self) -> &str {
        &self.type_
    }

    #[allow(dead_code)]
    pub fn get_config(&self) -> Option<&ComponentConfig> {
        self.config.as_ref()
    }
}

fn default_as_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct LoggingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slab_size_mib: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_size_mib: Option<u64>,
    #[serde(default = "default_as_true", skip_serializing_if = "Clone::clone")]
    pub enable_task_logging: bool,
}

/// The config is a list of tasks and their connections.
#[derive(Serialize, Deserialize, Default)]
struct CuConfigRepresentation {
    tasks: Vec<Node>,
    cnx: Vec<Cnx>,
    monitor: Option<MonitorConfig>,
    logging: Option<LoggingConfig>,
}

impl<'de> Deserialize<'de> for CuConfig {
    /// This is a custom serialization to make this implementation independent of petgraph.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let representation =
            CuConfigRepresentation::deserialize(deserializer).map_err(serde::de::Error::custom)?;

        let mut cuconfig = CuConfig::default();
        for task in representation.tasks {
            cuconfig.add_node(task);
        }

        for c in representation.cnx {
            let src = cuconfig
                .graph
                .node_indices()
                .find(|i| cuconfig.graph[*i].id == c.src)
                .expect("Source node not found");
            let dst = cuconfig
                .graph
                .node_indices()
                .find(|i| cuconfig.graph[*i].id == c.dst)
                .unwrap_or_else(|| panic!("Destination {} node not found", c.dst));
            cuconfig.connect_ext(
                src.index() as NodeId,
                dst.index() as NodeId,
                &c.msg,
                c.batch,
                c.store,
            );
        }
        cuconfig.monitor = representation.monitor;
        cuconfig.logging = representation.logging;
        Ok(cuconfig)
    }
}

impl Serialize for CuConfig {
    /// This is a custom serialization to make this implementation independent of petgraph.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let tasks: Vec<Node> = self
            .graph
            .node_indices()
            .map(|idx| self.graph[idx].clone())
            .collect();

        let cnx: Vec<Cnx> = self
            .graph
            .edge_indices()
            .map(|edge| self.graph[edge].clone())
            .collect();

        CuConfigRepresentation {
            tasks,
            cnx,
            monitor: self.monitor.clone(),
            logging: self.logging.clone(),
        }
        .serialize(serializer)
    }
}

impl Default for CuConfig {
    fn default() -> Self {
        CuConfig {
            graph: StableDiGraph::new(),
            monitor: None,
            logging: None,
        }
    }
}

/// The implementation has a lot of convenience methods to manipulate
/// the configuration to give some flexibility into programmatically creating the configuration.
impl CuConfig {
    /// Add a new node to the configuration graph.
    pub fn add_node(&mut self, node: Node) -> NodeId {
        self.graph.add_node(node).index() as NodeId
    }

    /// Get the node with the given id.
    #[allow(dead_code)] // Used in proc macro
    pub fn get_node(&self, node_id: NodeId) -> Option<&Node> {
        self.graph.node_weight(node_id.into())
    }

    /// Get the node with the given id mutably.
    #[allow(dead_code)] // Used in proc macro
    pub fn get_node_mut(&mut self, node_id: NodeId) -> Option<&mut Node> {
        self.graph.node_weight_mut(node_id.into())
    }

    /// this is more like infer from the connections of this node.
    #[allow(dead_code)] // Used in proc macro
    pub fn get_node_output_msg_type(&self, node_id: &str) -> Option<String> {
        self.graph.node_indices().find_map(|node_index| {
            if let Some(node) = self.get_node(node_index.index() as u32) {
                if node.id != node_id {
                    return None;
                }
                let edges = self.get_src_edges(node_index.index() as u32);
                if edges.is_empty() {
                    panic!("A CuSrcTask is configured with no task connected to it.")
                }
                let cnx = self
                    .graph
                    .edge_weight(EdgeIndex::new(edges[0]))
                    .expect("Found an cnx id but could not retrieve it back");
                return Some(cnx.msg.clone());
            }
            None
        })
    }

    /// this is more like infer from the connections of this node.
    #[allow(dead_code)] // Used in proc macro
    pub fn get_node_input_msg_type(&self, node_id: &str) -> Option<String> {
        self.graph.node_indices().find_map(|node_index| {
            if let Some(node) = self.get_node(node_index.index() as u32) {
                if node.id != node_id {
                    return None;
                }
                let edges = self.get_dst_edges(node_index.index() as u32);
                if edges.is_empty() {
                    panic!("A CuSinkTask is configured with no task connected to it.")
                }
                let cnx = self
                    .graph
                    .edge_weight(EdgeIndex::new(edges[0]))
                    .expect("Found an cnx id but could not retrieve it back");
                return Some(cnx.msg.clone());
            }
            None
        })
    }

    /// Get the list of edges that are connected to the given node as a source.
    pub fn get_src_edges(&self, node_id: NodeId) -> Vec<usize> {
        self.graph
            .edges_directed(node_id.into(), petgraph::Direction::Outgoing)
            .map(|edge| edge.id().index())
            .collect()
    }

    /// Get the list of edges that are connected to the given node as a destination.
    pub fn get_dst_edges(&self, node_id: NodeId) -> Vec<usize> {
        self.graph
            .edges_directed(node_id.into(), petgraph::Direction::Incoming)
            .map(|edge| edge.id().index())
            .collect()
    }

    #[allow(dead_code)]
    pub fn get_edge_weight(&self, index: usize) -> Option<Cnx> {
        self.graph.edge_weight(EdgeIndex::new(index)).cloned()
    }

    /// Convenience method to get all nodes in the configuration graph.
    pub fn get_all_nodes(&self) -> Vec<(NodeIndex, &Node)> {
        self.graph
            .node_indices()
            .map(|index| (index.index() as u32, &self.graph[index]))
            .collect()
    }

    /// Adds an edge between two nodes/tasks in the configuration graph.
    /// msg_type is the type of message exchanged between the two nodes/tasks.
    /// batch is the number of messages to batch before sending the buffer.
    /// store tells Copper if it needs to log the messages.
    pub fn connect_ext(
        &mut self,
        source: NodeId,
        target: NodeId,
        msg_type: &str,
        batch: Option<u32>,
        store: Option<bool>,
    ) {
        self.graph.add_edge(
            source.into(),
            target.into(),
            Cnx {
                src: self
                    .get_node(source)
                    .expect("Source node not found")
                    .id
                    .clone(),
                dst: self
                    .get_node(target)
                    .expect("Target node not found")
                    .id
                    .clone(),
                msg: msg_type.to_string(),
                batch,
                store,
            },
        );
    }

    /// Adds an edge between two nodes/tasks in the configuration graph.
    /// msg_type is the type of message exchanged between the two nodes/tasks.
    #[allow(dead_code)]
    pub fn connect(&mut self, source: NodeId, target: NodeId, msg_type: &str) {
        self.connect_ext(source, target, msg_type, None, None);
    }

    fn get_options() -> Options {
        Options::default()
            .with_default_extension(Extensions::IMPLICIT_SOME)
            .with_default_extension(Extensions::UNWRAP_NEWTYPES)
            .with_default_extension(Extensions::UNWRAP_VARIANT_NEWTYPES)
    }

    #[allow(dead_code)]
    pub fn serialize_ron(&self) -> String {
        let ron = Self::get_options();
        let pretty = ron::ser::PrettyConfig::default();
        ron.to_string_pretty(&self, pretty).unwrap()
    }

    pub fn deserialize_ron(ron: &str) -> Self {
        match Self::get_options().from_str(ron) {
            Ok(ron) => ron,
            Err(e) => panic!(
                "Syntax Error in config: {} at position {}",
                e.code, e.position
            ),
        }
    }

    /// Render the configuration graph in the dot format.
    pub fn render(&self, output: &mut dyn std::io::Write) {
        writeln!(output, "digraph G {{").unwrap();

        for index in self.graph.node_indices() {
            let node = &self.graph[index];
            let config_str = match &node.config {
                Some(config) => {
                    let config_str = config
                        .0
                        .iter()
                        .map(|(k, v)| format!("<B>{k}</B> = {v}<BR ALIGN=\"LEFT\"/>"))
                        .collect::<Vec<String>>()
                        .join("\n");
                    format!("____________<BR/><BR ALIGN=\"LEFT\"/>{config_str}")
                }
                None => String::new(),
            };
            writeln!(output, "{} [", index.index()).unwrap();
            writeln!(output, "shape=box,").unwrap();
            writeln!(output, "style=\"rounded, filled\",").unwrap();
            writeln!(output, "fontname=\"Noto Sans\"").unwrap();

            let is_src = self.get_dst_edges(index.index() as NodeId).is_empty();
            let is_sink = self.get_src_edges(index.index() as NodeId).is_empty();
            if is_src {
                writeln!(output, "fillcolor=lightgreen,").unwrap();
            } else if is_sink {
                writeln!(output, "fillcolor=lightblue,").unwrap();
            } else {
                writeln!(output, "fillcolor=lightgrey,").unwrap();
            }
            writeln!(output, "color=grey,").unwrap();

            writeln!(output, "labeljust=l,").unwrap();
            writeln!(
                output,
                "label=< <FONT COLOR=\"red\"><B>{}</B></FONT> <FONT COLOR=\"dimgray\">[{}]</FONT><BR ALIGN=\"LEFT\"/>{} >",
                node.id,
                node.get_type(),
                config_str
            )
                .unwrap();

            writeln!(output, "];").unwrap();
        }
        for edge in self.graph.edge_indices() {
            let (src, dst) = self.graph.edge_endpoints(edge).unwrap();

            let cnx = &self.graph[edge];
            let msg = encode_text(&cnx.msg);
            writeln!(
                output,
                "{} -> {} [label=< <B><FONT COLOR=\"gray\">{}</FONT></B> >];",
                src.index(),
                dst.index(),
                msg
            )
            .unwrap();
        }
        writeln!(output, "}}").unwrap();
    }

    #[allow(dead_code)]
    pub fn get_all_instances_configs(&self) -> Vec<Option<&ComponentConfig>> {
        self.get_all_nodes()
            .iter()
            .map(|(_, node)| node.get_instance_config())
            .collect()
    }

    #[allow(dead_code)]
    pub fn get_monitor_config(&self) -> Option<&MonitorConfig> {
        self.monitor.as_ref()
    }

    /// Validate the logging configuration to ensure section pre-allocation sizes do not exceed slab sizes.
    /// This method is wrapper around [LoggingConfig::validate]
    pub fn validate_logging_config(&self) -> CuResult<()> {
        if let Some(logging) = &self.logging {
            return logging.validate();
        }
        Ok(())
    }
}

impl LoggingConfig {
    /// Validate the logging configuration to ensure section pre-allocation sizes do not exceed slab sizes.
    pub fn validate(&self) -> CuResult<()> {
        if let Some(section_size_mib) = self.section_size_mib {
            if let Some(slab_size_mib) = self.slab_size_mib {
                if section_size_mib > slab_size_mib {
                    return Err(CuError::from(format!("Section size ({} MiB) cannot be larger than slab size ({} MiB). Adjust the parameters accordingly.", section_size_mib, slab_size_mib)));
                }
            }
        }

        Ok(())
    }
}

/// Read a copper configuration from a file.
pub fn read_configuration(config_filename: &str) -> CuResult<CuConfig> {
    let config_content = read_to_string(config_filename).map_err(|e| {
        CuError::from(format!(
            "Failed to read configuration file: {:?}",
            &config_filename
        ))
        .add_cause(e.to_string().as_str())
    })?;
    read_configuration_str(config_content)
}

/// Read a copper configuration from a String.
pub fn read_configuration_str(config_content: String) -> CuResult<CuConfig> {
    let cuconfig = CuConfig::deserialize_ron(&config_content);
    cuconfig.validate_logging_config()?;

    Ok(cuconfig)
}

// tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_serialize() {
        let mut config = CuConfig::default();
        let n1 = config.add_node(Node::new("test1", "package::Plugin1"));
        let n2 = config.add_node(Node::new("test2", "package::Plugin2"));
        config.connect(n1, n2, "msgpkg::MsgType");
        let serialized = config.serialize_ron();
        let deserialized = CuConfig::deserialize_ron(&serialized);
        assert_eq!(config.graph.node_count(), deserialized.graph.node_count());
        assert_eq!(config.graph.edge_count(), deserialized.graph.edge_count());
    }

    #[test]
    fn test_serialize_with_params() {
        let mut config = CuConfig::default();
        let mut camera = Node::new("copper-camera", "camerapkg::Camera");
        camera.set_param::<Value>("resolution-height", 1080.into());
        config.add_node(camera);
        let serialized = config.serialize_ron();
        let deserialized = CuConfig::deserialize_ron(&serialized);
        assert_eq!(
            deserialized
                .get_node(0)
                .unwrap()
                .get_param::<i32>("resolution-height")
                .unwrap(),
            1080
        );
    }

    #[test]
    #[should_panic(expected = "Syntax Error in config: Expected opening `[` at position 1:10")]
    fn test_deserialization_error() {
        // Task needs to be an array, but provided tuple wrongfully
        let txt = r#"( tasks: (), cnx: [], monitor: (type: "ExampleMonitor", ) ) "#;
        CuConfig::deserialize_ron(txt);
    }

    #[test]
    fn test_monitor() {
        let txt = r#"( tasks: [], cnx: [], monitor: (type: "ExampleMonitor", ) ) "#;
        let config = CuConfig::deserialize_ron(txt);
        assert_eq!(config.monitor.as_ref().unwrap().type_, "ExampleMonitor");

        let txt =
            r#"( tasks: [], cnx: [], monitor: (type: "ExampleMonitor", config: { "toto": 4, } )) "#;
        let config = CuConfig::deserialize_ron(txt);
        assert_eq!(
            config.monitor.as_ref().unwrap().config.as_ref().unwrap().0["toto"].0,
            4u8.into()
        );
    }

    #[test]
    fn test_logging_parameters() {
        // Test with `enable_task_logging: false`
        let txt = r#"( tasks: [], cnx: [], logging: ( slab_size_mib: 1024, section_size_mib: 100, enable_task_logging: false ),) "#;

        let config = CuConfig::deserialize_ron(txt);
        assert!(config.logging.is_some());
        let logging_config = config.logging.unwrap();
        assert_eq!(logging_config.slab_size_mib.unwrap(), 1024);
        assert_eq!(logging_config.section_size_mib.unwrap(), 100);
        assert!(!logging_config.enable_task_logging);

        // Test with `enable_task_logging` not provided
        let txt =
            r#"( tasks: [], cnx: [], logging: ( slab_size_mib: 1024, section_size_mib: 100, ),) "#;
        let config = CuConfig::deserialize_ron(txt);
        assert!(config.logging.is_some());
        let logging_config = config.logging.unwrap();
        assert_eq!(logging_config.slab_size_mib.unwrap(), 1024);
        assert_eq!(logging_config.section_size_mib.unwrap(), 100);
        assert!(logging_config.enable_task_logging);
    }

    #[test]
    fn test_validate_logging_config() {
        // Test with valid logging configuration
        let txt =
            r#"( tasks: [], cnx: [], logging: ( slab_size_mib: 1024, section_size_mib: 100 ) )"#;
        let config = CuConfig::deserialize_ron(txt);
        assert!(config.validate_logging_config().is_ok());

        // Test with invalid logging configuration
        let txt =
            r#"( tasks: [], cnx: [], logging: ( slab_size_mib: 100, section_size_mib: 1024 ) )"#;
        let config = CuConfig::deserialize_ron(txt);
        assert!(config.validate_logging_config().is_err());
    }

    // this test makes sure the edge id is suitable to be used to sort the inputs of a task
    #[test]
    fn test_deserialization_edge_id_assignment() {
        // note here that the src1 task is added before src2 in the tasks array,
        // however, src1 connection is added AFTER src2 in the cnx array
        let txt = r#"( 
            tasks: [(id: "src1", type: "a"), (id: "src2", type: "b"), (id: "sink", type: "c")],
            cnx: [(src: "src2", dst: "sink", msg: "msg1"), (src: "src1", dst: "sink", msg: "msg2")]
        )"#;
        let config = CuConfig::deserialize_ron(txt);
        assert!(config.validate_logging_config().is_ok());

        // the node id depends on the order in which the tasks are added
        let src1_id = 0;
        assert_eq!(config.get_node(src1_id).unwrap().id, "src1");
        let src2_id = 1;
        assert_eq!(config.get_node(src2_id).unwrap().id, "src2");

        // the edge id depends on the order the connection is created
        // the src2 was added second in the tasks, but the connection was added first
        let src1_edge_id = *config.get_src_edges(src1_id).first().unwrap();
        assert_eq!(src1_edge_id, 1);
        let src2_edge_id = *config.get_src_edges(src2_id).first().unwrap();
        assert_eq!(src2_edge_id, 0);
    }
}
