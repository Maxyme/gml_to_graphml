// GML to graphml converter

use std::collections::HashMap;
use std::fs::File;
use std::io::{copy, BufRead, BufReader, BufWriter, Write};

use itertools::Itertools;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use regex::Regex;
use serde_json::{json, Map, Number, Value};
use tempfile::NamedTempFile;

use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone)]
struct Node {
    id: String,
    data: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
struct Edge {
    source: String,
    target: String,
    data: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
struct GraphInfo {
    directed: bool,
    data: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
enum CurrentState {
    InGraph,
    InNode,
    InNodeObject,
    InEdge,
    InEdgeObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum GraphmlElems {
    Node,
    Edge,
    Graph,
}

impl GraphmlElems {
    fn value(&self) -> &str {
        match *self {
            GraphmlElems::Node => "node",
            GraphmlElems::Edge => "edge",
            GraphmlElems::Graph => "graph",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum GraphmlAttributeTypes {
    Int,
    Float,
    String,
}

impl GraphmlAttributeTypes {
    fn value(&self) -> &str {
        match *self {
            GraphmlAttributeTypes::Int => "int",
            GraphmlAttributeTypes::Float => "float",
            GraphmlAttributeTypes::String => "string",
            // Add double if necessary
            //GraphmlAttributeTypes::Double => "double",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct KeyAttributes {
    attr_name: String,
    for_elem: GraphmlElems,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct KeyValues {
    id: String,
    attr_type: GraphmlAttributeTypes,
}

fn add_header(writer: &mut Writer<BufWriter<File>>) {
    // Write the Graphml header

    // Add the xml declaration
    let header = BytesDecl::new(
        "1.0".as_bytes(),
        Some("UTF-8".as_bytes()),
        Some("yes".as_bytes()),
    );
    writer
        .write_event(Event::Decl(header))
        .expect("Unable to write data");

    // Open the graphml node and add the boilerplate attributes
    let mut elem = BytesStart::borrowed_name("graphml".as_bytes());
    elem.push_attribute(("xmlns", "http://graphml.graphdrawing.org/xmlns"));
    elem.push_attribute(("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"));
    elem.push_attribute(("xsi:schemaLocation", "http://graphml.graphdrawing.org/xmlns http://graphml.graphdrawing.org/xmlns/1.0/graphml.xsd"));
    writer
        .write_event(Event::Start(elem))
        .expect("Unable to write data");
}

fn add_graph_info(writer: &mut Writer<BufWriter<&File>>, graph: &GraphInfo) {
    // Add the graph node: <data key="d0">Test gml file</data>
    let name = "graph".as_bytes();
    let mut elem = BytesStart::borrowed_name(name);
    let direction = if graph.directed {
        "directed"
    } else {
        "undirected"
    };
    elem.push_attribute(("edgedefault", direction));
    add_elem_with_keys(writer, &graph.data, elem, name, false);
}

fn add_footer(writer: &mut Writer<BufWriter<&File>>) {
    // Close the graph and graphml xml nodes
    writer
        .write_event(Event::End(BytesEnd::borrowed(b"graph")))
        .ok();
    writer
        .write_event(Event::End(BytesEnd::borrowed(b"graphml")))
        .ok();
}

fn add_node(writer: &mut Writer<BufWriter<&File>>, node: &Node) {
    // Add a new xml node: <node id="1"><data key="d0">1.0</data></node>
    let name = "node".as_bytes();
    let mut node_elem = BytesStart::borrowed_name(name);
    node_elem.push_attribute(("id", node.id.as_str()));
    add_elem_with_keys(writer, &node.data, node_elem, name, true);
}

fn add_edge(writer: &mut Writer<BufWriter<&File>>, edge: &Edge) {
    // Add a new xml edge: <edge source="1" target="2"><data key="d1">1.1</data></edge>
    let name = "edge".as_bytes();
    let mut edge_elem = BytesStart::borrowed_name(name);
    edge_elem.push_attribute(("source", edge.source.as_str()));
    edge_elem.push_attribute(("target", edge.target.as_str()));
    add_elem_with_keys(writer, &edge.data, edge_elem, name, true);
}

fn add_elem_with_keys(
    writer: &mut Writer<BufWriter<&File>>,
    elem_data: &Vec<(String, String)>,
    elem: BytesStart,
    elem_name: &[u8],
    close: bool,
) {
    // Add a xml element with data keys. Note close is false for graph as it is closed at the end
    writer
        .write_event(Event::Start(elem))
        .expect("Unable to write data");
    for (key, value) in elem_data {
        let mut data = BytesStart::borrowed_name("data".as_bytes());
        data.push_attribute(("key", key.as_str()));
        writer
            .write_event(Event::Start(data))
            .expect("Unable to write data");
        // Note: use from_plain_str instead to escape double quotes if present
        //let text = BytesText::from_escaped_str(value.as_str());
        let text = BytesText::from_plain_str(value.as_str());
        writer
            .write_event(Event::Text(text))
            .expect("Unable to write data");
        writer
            .write_event(Event::End(BytesEnd::borrowed(b"data")))
            .ok();
    }
    if close {
        writer
            .write_event(Event::End(BytesEnd::borrowed(elem_name)))
            .ok();
    }
}

fn add_keys(writer: &mut Writer<BufWriter<File>>, keys: &HashMap<KeyAttributes, KeyValues>) {
    // Write the list of xml keys
    // <key id="d10" for="edge" attr.name="list" attr.type="string" />

    // Sort the keys by id values
    use std::iter::FromIterator;
    let mut v = Vec::from_iter(keys);
    v.sort_by(|&(_, a), &(_, b)| {
        (a.id[1..])
            .parse::<u32>()
            .expect("")
            .cmp(&(b.id[1..]).parse::<u32>().expect(""))
    });

    for (key, value) in v {
        let mut elem = BytesStart::borrowed_name("key".as_bytes());
        elem.push_attribute(("id", value.id.as_str()));
        elem.push_attribute(("for", key.for_elem.value()));
        elem.push_attribute(("attr.name", key.attr_name.as_str()));
        elem.push_attribute(("attr.type", value.attr_type.value()));
        writer
            .write_event(Event::Empty(elem))
            .expect("Unable to write data");
    }
}

fn parse_data_line(line: &String) -> (String, String) {
    // Parse a single data line to extract name a value, remove superfluous quotes around a string
    // Todo: use regex to only replace quotes if at start and end of string
    let (name, value): (String, String) = line
        .trim()
        .splitn(2, char::is_whitespace)
        .map(|x| x.replace("\"", ""))
        .collect_tuple()
        .expect("Error parsing data");

    (name, value)
}

fn get_or_add_key_id(
    keys: &mut HashMap<KeyAttributes, KeyValues>,
    attribute: String,
    value: &String,
    for_elem: GraphmlElems,
) -> String {
    // Get the key id if it already exists otherwise create a new key id
    let key_attr = KeyAttributes {
        attr_name: attribute,
        for_elem,
    };

    match keys.get(&key_attr) {
        Some(values) => values.id.clone(),
        None => {
            // Check to see if it's a number
            let mut attribute_type = GraphmlAttributeTypes::String;
            if value.parse::<u32>().is_ok() {
                // Note: it is assumed that future values of same key will be int!
                attribute_type = GraphmlAttributeTypes::Int;
            } else if value.parse::<f64>().is_ok() {
                attribute_type = GraphmlAttributeTypes::Float;
            }

            let key_count = keys.len();
            let values = KeyValues {
                id: format!("d{}", key_count),
                attr_type: attribute_type,
            };
            let key_id = values.id.clone();
            keys.insert(key_attr, values);
            key_id
        }
    }
}

pub fn export_to_graphml(input_gml: &Path, output_path: &Path) {
    // Convert the import file to graphml using a bufreader and xml bufwriter
    // Write to a temp file and copy the file with header information added after at the output path destination
    // Todo: check if instantiating a bufwriter with a bigger capacity makes it faster for large files
    let tmp_file = NamedTempFile::new().expect("Error creating temp file");

    let mut output_file = File::create(output_path).expect("Unable to create file");

    let writer = BufWriter::new(tmp_file.as_file());
    let mut xml_writer = Writer::new_with_indent(writer, ' ' as u8, 2);

    let input_file = File::open(input_gml).expect("Issue reading file at path");
    let buf_reader = BufReader::new(input_file);

    // Current node info - Todo initialize empty
    let mut node = Node {
        id: "".to_string(),
        data: Vec::new(),
    };

    // Current edge info
    let mut edge = Edge {
        source: "".to_string(),
        target: "".to_string(),
        data: Vec::new(),
    };

    // Current graph info
    let mut graph = GraphInfo {
        directed: false,
        data: Vec::new(),
    };

    let mut graph_info_added = false;

    // Key info
    let mut keys: HashMap<KeyAttributes, KeyValues> = HashMap::new();

    // Current dict info (inside an edge or a node)
    let mut dict_key_value: String = "".to_string(); // key value name for the dict;
    let mut inner_dict: Map<String, Value> = Map::new();
    let mut list_item_staging: String = "".to_string(); // staging item for possible lists

    let mut state = CurrentState::InGraph;

    let re_node_start = Regex::new(r"node \[").unwrap();
    let re_edge_start = Regex::new(r"edge \[").unwrap();
    let re_graph_start = Regex::new(r"graph \[").unwrap();
    let re_closing_bracket = Regex::new(r"\]").unwrap();

    for line in buf_reader.lines() {
        let line = line.expect("Unable to read line");
        if line.trim().starts_with("#") {
            // skip comments - Note, comments could be added directly?
            continue;
        }

        match line.as_str() {
            line if re_graph_start.is_match(line) => {
                // entering graph - least likely to happen so it can be last
                state = CurrentState::InGraph;
            }
            line if re_node_start.is_match(line) => {
                // entering node data
                if !graph_info_added {
                    // Add graph data when entering the first node
                    add_graph_info(&mut xml_writer, &graph);
                    graph_info_added = true;
                }
                state = CurrentState::InNode;
            }
            line if re_edge_start.is_match(line) => {
                // entering edge
                state = CurrentState::InEdge;
            }
            line if re_closing_bracket.is_match(line) && line.trim().len() == 1 => {
                // End previous open item (node, edge, graph, or in-items dict or list)
                // if in [node, edge, graph] add write the data
                // Note, only if the line contains a closing bracket and nothing else ie ("[]" would not work)
                match state {
                    CurrentState::InNodeObject | CurrentState::InEdgeObject => {
                        // add dict and clear
                        let graph_elem = {
                            if state == CurrentState::InEdgeObject {
                                GraphmlElems::Edge
                            } else {
                                GraphmlElems::Node
                            }
                        };

                        let serialized_dict = json!(inner_dict).to_string();
                        let key_id = get_or_add_key_id(
                            &mut keys,
                            dict_key_value.clone(),
                            &serialized_dict,
                            graph_elem,
                        );
                        // Add attributes
                        if state == CurrentState::InNodeObject {
                            node.data.push((key_id, serialized_dict));
                            state = CurrentState::InNode
                        } else if state == CurrentState::InEdgeObject {
                            edge.data.push((key_id, serialized_dict));
                            state = CurrentState::InEdge
                        }
                        inner_dict.clear();
                        list_item_staging.clear();
                    }
                    CurrentState::InEdge => {
                        // Add edge when exiting an edge
                        add_edge(&mut xml_writer, &edge);
                        state = CurrentState::InGraph;
                        edge.data.clear();
                    }
                    CurrentState::InNode => {
                        // Add node and increment node id when exiting node
                        add_node(&mut xml_writer, &node);
                        state = CurrentState::InGraph;
                        node.data.clear();
                    }
                    CurrentState::InGraph => continue, // graph completed, ignore
                };
            }
            _ => {
                // Parse the name and value
                let (name, value) = parse_data_line(&line);
                match state {
                    CurrentState::InGraph => {
                        // Add graph attributes
                        match name.as_str() {
                            "directed" => {
                                graph.directed = value == "1";
                            }
                            _ => {
                                let key_id =
                                    get_or_add_key_id(&mut keys, name, &value, GraphmlElems::Graph);
                                // Add graph attributes
                                graph.data.push((key_id, value))
                            }
                        };
                    }
                    CurrentState::InNode => {
                        if name == "id" {
                            node.id = value;
                        } else if value.contains("[") {
                            // Start dict attribute
                            state = CurrentState::InNodeObject;
                            dict_key_value = name;
                        } else {
                            let key_id =
                                get_or_add_key_id(&mut keys, name, &value, GraphmlElems::Node);
                            // Add node attributes
                            node.data.push((key_id, value))
                        }
                    }
                    CurrentState::InEdge => {
                        if name == "source" {
                            edge.source = value;
                        } else if name == "target" {
                            edge.target = value;
                        } else if value.contains("[") {
                            // Start dict attribute
                            state = CurrentState::InEdgeObject;
                            dict_key_value = name;
                        } else {
                            // Add or update keys for attribute
                            let key_id =
                                get_or_add_key_id(&mut keys, name, &value, GraphmlElems::Edge);
                            edge.data.push((key_id, value));
                        }
                    }
                    CurrentState::InNodeObject | CurrentState::InEdgeObject => {
                        if name == list_item_staging {
                            // when all the names are the same it's a list
                            let value_object = {
                                if value.parse::<f64>().is_ok() {
                                    Value::Number(Number::from_str(value.as_str()).expect("Error"))
                                } else {
                                    Value::from(value)
                                }
                            };
                            // Parse the value into the correct format
                            let key_value = inner_dict
                                .get_mut(name.as_str())
                                .expect("Issue retrieving key");
                            if key_value.is_array() {
                                // if the previous value was a string, add the new value to a list
                                let mut new_vect = key_value.as_array().unwrap().clone();
                                new_vect.push(value_object);
                                *key_value = json!(new_vect);
                            } else {
                                // otherwise create a list from the old value and append the new value
                                *key_value = json!([key_value, value_object]);
                            }
                        } else {
                            // add attributes to the dict currently being built
                            list_item_staging = name.clone();
                            // check if it can be parsed as a number, otherwise use string
                            match value.parse::<f64>() {
                                Ok(_) => inner_dict.insert(
                                    name,
                                    serde_json::Value::Number(
                                        Number::from_str(value.as_str()).expect("Error"),
                                    ),
                                ),
                                Err(_) => inner_dict.insert(name, Value::String(value)),
                            };
                        }
                    }
                }
            }
        }
    }
    // Add remaining elements
    add_footer(&mut xml_writer);
    // Flush the remaining buffer - could also close the scope
    xml_writer.inner().flush().ok();

    // Write the  header, keys and graph info into another file and merge the result into the final file
    let writer = BufWriter::new(output_file.try_clone().expect(""));
    let mut new_xml_writer = Writer::new_with_indent(writer, ' ' as u8, 2);
    add_header(&mut new_xml_writer);
    add_keys(&mut new_xml_writer, &keys);
    new_xml_writer.write("\n".as_ref()).ok();
    new_xml_writer.inner().flush().ok();

    // Merge the previous file
    let mut src = File::open(&tmp_file).expect("Error opening source file");
    copy(&mut src, &mut output_file).expect("Error copying file");
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::gml_to_graphml;

    #[test]
    fn test_simple() {
        let filename = "./src/data/test_simple.gml";
        let input_path = Path::new(filename);
        let output_path = Path::new("./src/result_simple.graphml");
        gml_to_graphml::export_to_graphml(&input_path, &output_path);
    }

    #[test]
    fn test_complex() {
        let filename = "./src/data/test_complex.gml";
        let input_path = Path::new(filename);
        let output_path = Path::new("./src/result_complex.graphml");
        gml_to_graphml::export_to_graphml(&input_path, &output_path);
    }
}