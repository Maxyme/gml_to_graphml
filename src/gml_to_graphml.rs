// GML to graphml converter

use std::collections::HashMap;
use std::fs::File;
use std::io::{copy, BufRead, BufReader, BufWriter, Write};

use itertools::Itertools;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use regex::Regex;
use serde_json::{json, Map, Number, Value};
use uuid::Uuid;

use std::path::Path;
use std::str::FromStr;

use std::hash::Hash;
use std::{env, fs};

#[derive(Debug, Clone)]
struct Node {
    id: String,
    data: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
struct Edge {
    source: String,
    target: String,
    data: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
struct GraphInfo {
    directed: Option<bool>,
    data: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
enum CurrentState {
    Graph,
    Node,
    NodeObject,
    Edge,
    EdgeObject,
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

const LINE_BREAK: &[u8] = b"\n";

fn add_header(writer: &mut Writer<BufWriter<File>>) {
    // Write the Graphml header

    // Add the xml declaration
    let header = BytesDecl::new(b"1.0", Some(b"UTF-8"), Some(b"yes"));
    writer.write_event(Event::Decl(header)).ok();

    // Open the graphml node and add the boilerplate attributes
    let mut elem = BytesStart::borrowed_name(b"graphml");
    elem.push_attribute(("xmlns", "http://graphml.graphdrawing.org/xmlns"));
    elem.push_attribute(("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"));
    elem.push_attribute(("xsi:schemaLocation", "http://graphml.graphdrawing.org/xmlns http://graphml.graphdrawing.org/xmlns/1.0/graphml.xsd"));
    writer.write_event(Event::Start(elem)).ok();
}

fn add_graph_info(writer: &mut Writer<BufWriter<&File>>, graph: &GraphInfo) {
    // Add the graph node: <data key="d0">Test gml file</data>
    let name = b"graph";
    let mut elem = BytesStart::borrowed_name(name);

    match graph.directed {
        Some(true) => {
            elem.push_attribute(("edgedefault", "directed"));
        }
        Some(false) => {
            elem.push_attribute(("edgedefault", "undirected"));
        }
        _ => {}
    }
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
    let name = b"node";
    let mut node_elem = BytesStart::borrowed_name(name);
    node_elem.push_attribute(("id", node.id.as_str()));
    add_elem_with_keys(writer, &node.data, node_elem, name, true);
}

fn add_edge(writer: &mut Writer<BufWriter<&File>>, edge: &Edge) {
    // Add a new xml edge: <edge source="1" target="2"><data key="d1">1.1</data></edge>
    let name = b"edge";
    let mut edge_elem = BytesStart::borrowed_name(name);
    edge_elem.push_attribute(("source", edge.source.as_str()));
    edge_elem.push_attribute(("target", edge.target.as_str()));
    add_elem_with_keys(writer, &edge.data, edge_elem, name, true);
}

fn add_elem_with_keys(
    writer: &mut Writer<BufWriter<&File>>,
    elem_data: &HashMap<String, Vec<String>>,
    elem: BytesStart,
    elem_name: &[u8],
    close: bool,
) {
    // Add a xml element with data keys. Note close is false for graph as it is closed at the end
    writer.write_event(Event::Start(elem)).ok();
    for (key, value) in elem_data {
        let mut data = BytesStart::borrowed_name(b"data");
        data.push_attribute(("key", key.as_str()));
        writer.write_event(Event::Start(data)).ok();

        // Generate the text element from the value.
        // Note: use from_plain_str instead of from_escaped_str to escape double quotes from json values

        // Make a list from the value if it has a size > 1
        let formatted_value = {
            if value.len() > 1 {
                format!("[{}]", value.join(","))
            } else {
                value.first().expect("").to_owned()
            }
        };

        let text = {
            if formatted_value.starts_with('\"') || formatted_value.ends_with('\"') {
                // Remove outside quotes if there are any
                BytesText::from_plain_str(&formatted_value[1..formatted_value.len() - 1])
            } else {
                BytesText::from_plain_str(formatted_value.as_str())
            }
        };

        writer.write_event(Event::Text(text)).ok();
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
    let mut v = Vec::from_iter(keys);
    v.sort_by(|&(_, a), &(_, b)| {
        (a.id[1..])
            .parse::<usize>()
            .expect("")
            .cmp(&(b.id[1..]).parse::<usize>().expect(""))
    });

    for (key, value) in v {
        let mut elem = BytesStart::borrowed_name(b"key");
        elem.push_attribute(("id", value.id.as_str()));
        elem.push_attribute(("for", key.for_elem.value()));
        elem.push_attribute(("attr.name", key.attr_name.as_str()));
        elem.push_attribute(("attr.type", value.attr_type.value()));
        writer.write_event(Event::Empty(elem)).ok();
    }
}

fn get_or_add_key_id(
    keys: &mut HashMap<KeyAttributes, KeyValues>,
    key_attr: &KeyAttributes,
    value: &str,
) -> String {
    // Get the key id if it already exists otherwise create a new key id
    match keys.get(key_attr) {
        Some(values) => values.id.to_string(),
        None => {
            let attribute_type = {
                // Note: it is assumed that future values of same key will be same type!
                if value.starts_with('\"') && value.ends_with('\"') {
                    GraphmlAttributeTypes::String
                }
                // Otherwise check to see if it's a number
                else if value.parse::<u32>().is_ok() {
                    GraphmlAttributeTypes::Int
                } else if value.parse::<f64>().is_ok() {
                    GraphmlAttributeTypes::Float
                } else {
                    // everything else is a string type
                    GraphmlAttributeTypes::String
                }
            };
            let id = format!("d{}", keys.len());
            let values = KeyValues {
                id: id.clone(),
                attr_type: attribute_type,
            };
            keys.insert(key_attr.clone(), values);
            id
        }
    }
}

fn update_element(
    elem_map: &mut HashMap<String, Vec<String>>,
    keys: &mut HashMap<KeyAttributes, KeyValues>,
    value: &str,
    element: GraphmlElems,
    name: &str,
) {
    // Add value to element (graph, node or edge) values since the values can be a single item or a list

    let key_attr = KeyAttributes {
        attr_name: name.to_string(),
        for_elem: element,
    };
    let key_id = get_or_add_key_id(keys, &key_attr, value);

    // Get the original data or an empty vec if no data exists for the element
    let elem_data = elem_map.entry(key_id).or_insert_with(Vec::new);

    // Check if the value has been seen for the same node which would mean a list type instead
    // Update the key attribute value to string as it will contain a serialized list and the
    // previous type could be int or float
    if elem_data.len() > 1 {
        let elem = keys
            .get_mut(&key_attr)
            .expect("Error retrieving previous key!");
        (*elem).attr_type = GraphmlAttributeTypes::String;
    }
    // Add the value to the global node hashmap
    elem_data.push(value.to_string());
}

pub fn export_to_graphml(input_gml: &Path, output_path: &Path) {
    // Convert the import file to graphml using a bufreader and xml bufwriter
    // Write to a temp file and copy the file with header information added after at the output path destination
    // Todo: check if instantiating a bufwriter with a bigger capacity makes it faster for large files
    let tmp_dir = env::temp_dir();
    let tmp_name = Uuid::new_v4().to_string();
    let tmp_path = tmp_dir.join(tmp_name);
    let tmp_file = File::create(&tmp_path).expect("Unable to create file");
    let mut output_file = File::create(output_path).expect("Unable to create file");

    let writer = BufWriter::new(&tmp_file);
    let mut xml_writer = Writer::new_with_indent(writer, b' ', 2);

    let input_file = File::open(input_gml).expect("Issue reading file at path");
    let buf_reader = BufReader::new(input_file);

    // Current node info - Todo initialize empty
    let mut node = Node {
        id: "".to_string(),
        data: Default::default(),
    };

    // Current edge info
    let mut edge = Edge {
        source: String::new(),
        target: String::new(),
        data: Default::default(),
    };

    // Current graph info
    let mut graph = GraphInfo {
        directed: None,
        data: Default::default(),
    };

    let mut graph_info_added = false;

    // Key info
    let mut keys: HashMap<KeyAttributes, KeyValues> = HashMap::new();

    // Current dict info (inside an edge or a node)
    let mut dict_key_value = String::new(); // key value name for the dict;
    let mut inner_dict: Map<String, Value> = Map::new();
    let mut list_item_staging = String::new(); // staging item for possible lists

    let mut state = CurrentState::Graph;

    let re_node_start = Regex::new(r"node \[").unwrap();
    let re_edge_start = Regex::new(r"edge \[").unwrap();
    let re_graph_start = Regex::new(r"graph \[").unwrap();
    let re_closing_bracket = Regex::new(r"^\]$").unwrap();

    for line in buf_reader.lines() {
        let line = line.expect("Unable to read line");
        if line.trim().starts_with('#') {
            // skip comments - Note, comments could be added directly?
            continue;
        }

        match line.trim() {
            line if re_graph_start.is_match(line) => {
                // entering graph - least likely to happen so it can be last
                state = CurrentState::Graph;
            }
            line if re_node_start.is_match(line) => {
                // entering node data
                if !graph_info_added {
                    // Add graph data when entering the first node
                    add_graph_info(&mut xml_writer, &graph);
                    graph_info_added = true;
                }
                state = CurrentState::Node;
            }
            line if re_edge_start.is_match(line) => {
                // entering edge
                state = CurrentState::Edge;
            }
            line if re_closing_bracket.is_match(line) && line.len() == 1 => {
                // End previous open item (node, edge, graph, or in-items dict or list)
                // if in [node, edge, graph] add write the data
                // Note, only if the line contains a closing bracket and nothing else ie ("[]" would not work)
                match state {
                    CurrentState::Edge => {
                        // Add edge when exiting an edge
                        add_edge(&mut xml_writer, &edge);
                        state = CurrentState::Graph;
                        edge.data.clear();
                    }
                    CurrentState::Node => {
                        // Add node and increment node id when exiting node
                        add_node(&mut xml_writer, &node);
                        state = CurrentState::Graph;
                        node.data.clear();
                    }
                    CurrentState::Graph => continue, // graph completed, ignore
                    CurrentState::NodeObject => {
                        let serialized_value = json!(inner_dict).to_string();
                        update_element(
                            &mut node.data,
                            &mut keys,
                            serialized_value.as_str(),
                            GraphmlElems::Node,
                            dict_key_value.as_str(),
                        );
                        state = CurrentState::Node;
                        inner_dict.clear();
                        list_item_staging.clear();
                    }
                    CurrentState::EdgeObject => {
                        let serialized_value = json!(inner_dict).to_string();
                        update_element(
                            &mut edge.data,
                            &mut keys,
                            serialized_value.as_str(),
                            GraphmlElems::Edge,
                            dict_key_value.as_str(),
                        );
                        state = CurrentState::Edge;
                        inner_dict.clear();
                        list_item_staging.clear();
                    }
                };
            }
            _ => {
                // Parse a single data line to extract name and value
                let (name, value) = line
                    .trim()
                    .splitn(2, char::is_whitespace)
                    .collect_tuple()
                    .expect("Error parsing data");

                match state {
                    CurrentState::Graph => {
                        // Add graph attributes
                        match name {
                            "directed" => {
                                graph.directed = Some(value == "1");
                            }
                            _ => {
                                // Update the global graph object with the new value
                                update_element(
                                    &mut graph.data,
                                    &mut keys,
                                    value,
                                    GraphmlElems::Graph,
                                    name,
                                );
                            }
                        };
                    }
                    CurrentState::Node => {
                        if name == "id" {
                            // add a default n in front of the id
                            node.id = format!("n{}", value);
                        } else if value.trim().ends_with('[') {
                            // Start dict attribute
                            state = CurrentState::NodeObject;
                            dict_key_value = name.to_string();
                        } else {
                            // Update the global node object with the new value
                            update_element(
                                &mut node.data,
                                &mut keys,
                                value,
                                GraphmlElems::Node,
                                name,
                            );
                        }
                    }
                    CurrentState::Edge => {
                        if name == "source" {
                            // add a default n in front of the source to match the node id
                            edge.source = format!("n{}", value);
                        } else if name == "target" {
                            // add a default n in front of the target to match the node id
                            edge.target = format!("n{}", value);
                        } else if value.trim().ends_with('[') {
                            // Start dict attribute
                            state = CurrentState::EdgeObject;
                            dict_key_value = name.to_string();
                        } else {
                            // Update global edge object with the new value
                            update_element(
                                &mut edge.data,
                                &mut keys,
                                value,
                                GraphmlElems::Edge,
                                name,
                            );
                        }
                    }
                    CurrentState::NodeObject | CurrentState::EdgeObject => {
                        if name == list_item_staging {
                            // todo: use a dict here instead, and check if key in dict
                            // when all the names are the same it's a list
                            let value_object = {
                                if value.parse::<f64>().is_ok() {
                                    Value::Number(Number::from_str(value).expect("Error"))
                                } else {
                                    Value::from(value)
                                }
                            };
                            // Parse the value into the correct format
                            let key_value = inner_dict.get_mut(name).expect("Issue retrieving key");
                            if key_value.is_array() {
                                // if the previous value was a string, add the new value to a list
                                let new_vect = key_value.as_array_mut().unwrap();
                                new_vect.push(value_object);
                                *key_value = json!(new_vect);
                            } else {
                                // otherwise create a list from the old value and append the new value
                                *key_value = json!([key_value, value_object]);
                            }
                        } else {
                            // add attributes to the dict currently being built
                            list_item_staging = name.to_string();
                            // check if it can be parsed as a number, otherwise use string
                            match value.parse::<f64>() {
                                Ok(_) => inner_dict.insert(
                                    name.to_string(),
                                    serde_json::Value::Number(
                                        Number::from_str(value).expect("Error parsing string"),
                                    ),
                                ),
                                Err(_) => inner_dict.insert(
                                    name.to_string(),
                                    Value::from_str(value).expect("Error parsing string"),
                                ),
                            };
                        }
                    }
                }
            }
        }
    }

    if !graph_info_added {
        // Add graph data if not added (ie, when no nodes are present)
        add_graph_info(&mut xml_writer, &graph);
    }

    // Add remaining elements
    add_footer(&mut xml_writer);
    // Flush the remaining buffer - could also close the scope
    xml_writer.inner().flush().ok();

    // Write the  header, keys and graph info into another file and merge the result into the final file
    let writer = BufWriter::new(output_file.try_clone().expect(""));
    let mut new_xml_writer = Writer::new_with_indent(writer, b' ', 2);
    add_header(&mut new_xml_writer);
    add_keys(&mut new_xml_writer, &keys);
    new_xml_writer.write(LINE_BREAK).ok();
    new_xml_writer.inner().flush().ok();

    // Merge the previous file
    let mut src = File::open(&tmp_path).expect("Error opening source file");
    copy(&mut src, &mut output_file).expect("Error copying file");

    // Remove the temp file
    fs::remove_file(&tmp_path).expect("Issue deleting temp file");
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate test;
    use test::Bencher;

    #[test]
    fn test_simple() {
        let input_path = Path::new("./src/data/test_simple.gml");
        let output_path = Path::new("./src/result_simple.graphml");
        export_to_graphml(&input_path, &output_path);
    }

    #[test]
    fn test_complex() {
        let input_path = Path::new("./src/data/test_complex.gml");
        let output_path = Path::new("./src/result_complex.graphml");
        export_to_graphml(&input_path, &output_path);
    }

    #[bench]
    fn bench_complex(b: &mut Bencher) {
        let input_path = Path::new("./src/data/test_complex.gml");
        let output_path = Path::new("./src/result_complex.graphml");
        b.iter(|| {
            export_to_graphml(&input_path, &output_path);
        });
    }
}
