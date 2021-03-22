// Graphml to GML converter
// Note: This takes a decision to remove keys that have no values, ie. <data key="v"></data> will be omitted in the final gml
// Todo: use COW with [u8] instead of converting to string and back when writing

use quick_xml::events::attributes::Attributes;
use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::str;

#[derive(Debug, Clone)]
struct Node {
    id: String,
    data: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
//struct Edge<'a> {
struct Edge {
    source: String,
    target: String, //Cow<'a, [u8]>, //String,
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
    InEdge,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ValueTypes {
    String,
    Double,
    Float,
    Int,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ForTypes {
    Edge,
    Node,
    Graph,
}

#[derive(Debug, Clone)]
struct Key {
    attr_name: String,
    attr_type: ValueTypes,
    for_type: ForTypes,
}

const INDENT_2: &str = "  ";
const INDENT_4: &str = "    ";
const LINE_BREAK: &[u8] = "\r\n".as_bytes();

fn write_graph_start(writer: &mut BufWriter<&File>, graph: &GraphInfo) {
    // write graph specific content first then graph data, like nodes and edges
    writer.write("graph [".as_bytes()).ok();
    writer.write("\r\n".as_bytes()).ok();
    writer
        .write(format!("{}directed 0", INDENT_2).as_bytes())
        .ok();
    writer.write(LINE_BREAK).ok();
    write_data_items(writer, &graph.data, INDENT_2);
    // Note, not closing yet, as this will be done at the end
}

fn write_graph_end(writer: &mut BufWriter<&File>) {
    // Close graph after all nodes and edges have been written
    writer.write("]".as_bytes()).ok();
}

fn write_edge(writer: &mut BufWriter<&File>, edge: &Edge) {
    // write edge specific content first then edge data
    writer.write(format!("{}edge [", INDENT_2).as_bytes()).ok();
    writer.write("\r\n".as_bytes()).expect("");
    writer
        .write(format!("{}source {}", INDENT_4, edge.source).as_ref())
        .ok();
    writer.write(LINE_BREAK).expect("");
    writer
        .write(format!("{}target {}", INDENT_4, edge.target).as_ref())
        .ok();
    writer.write(LINE_BREAK).expect("");
    // Add data in a loop
    write_data_items(writer, &edge.data, INDENT_4);
    // Close node
    writer.write(format!("{}]", INDENT_2).as_bytes()).ok();
    writer.write(LINE_BREAK).expect("");
}

fn write_node(writer: &mut BufWriter<&File>, node: &Node) {
    // write node specific content first then node data
    writer.write(format!("{}node [", INDENT_2).as_bytes()).ok();
    writer.write(LINE_BREAK).expect("");
    writer
        .write(format!("{}id {}", INDENT_4, node.id).as_ref())
        .ok();
    writer.write(LINE_BREAK).expect("");
    // Add data in a loop
    write_data_items(writer, &node.data, INDENT_4);
    // Close node
    writer.write(format!("{}]", INDENT_2).as_bytes()).ok();
    writer.write(LINE_BREAK).expect("");
}

fn write_data_items(writer: &mut BufWriter<&File>, data: &Vec<(String, String)>, indent: &str) {
    // Write data items, while checking if the value is encoded json to unpack into lists and dicts
    for (key, value) in data.iter() {
        // check if the value is a dict or a list, which means it starts with "{ and ends with }"
        //if re_is_dict.is_match(&value) {
        if value.contains("{") {
            // todo, try parsing as json and if it fails, just add as string? Why would { be in a value?!
            let new_value = value.replace("\"{", "{").replace("}\"", "}");
            let map: Value =
                serde_json::from_str(new_value.as_str()).expect("Issue parsing into json");
            write_dict(writer, &map, key);
        } else {
            writer
                .write(format!("{}{} {}", indent, key, value).as_ref())
                .ok();
            writer.write(LINE_BREAK).expect("");
        }
    }
}

fn get_value_with_increment(key: &String, item: &Value) -> String {
    // Generate a string with the values in the correct format with an indent
    if item.is_f64() {
        format!("{}{} {:?}", INDENT_4, key, item.as_f64().expect(""))
    } else if item.is_i64() {
        format!("{}{} {:?}", INDENT_4, key, item.as_i64().expect(""))
    } else if item.is_string() {
        format!("{}{} {:?}", INDENT_4, key, item.as_str().expect(""))
    } else {
        panic!("Could not decipher value type");
    }
}

fn write_dict(writer: &mut BufWriter<&File>, json: &Value, dict_key: &String) {
    // Write gml dicts, with possible inside lists
    // a [
    //   y 2
    //   z 1
    //   list [
    //        i 1
    //        i 2
    //   ]
    // ]
    let dict_increment = " ".repeat(4);
    writer
        .write(format!("{}{} [", dict_increment, dict_key).as_bytes())
        .ok();
    writer.write("\r\n".as_bytes()).expect("");
    // todo: there should only be one value?
    for (key, value) in json.as_object().expect("Value isn't valid a dict object") {
        match value {
            value if value.is_array() => {
                // add key [ into the lines
                for item in value.as_array().expect("").iter() {
                    // add value and incrementation (2 spaces)
                    let value_string = get_value_with_increment(key, item);
                    writer
                        .write(format!("{}{}", dict_increment, value_string).as_bytes())
                        .ok();
                    writer.write(LINE_BREAK).expect("");
                }
            }
            _ => {
                // value is an object
                let value_string = get_value_with_increment(key, value);
                writer
                    .write(format!("{}{}", dict_increment, value_string).as_bytes())
                    .ok();
                writer.write(LINE_BREAK).expect("");
            }
        }
    }
    // Close dict
    writer.write(format!("{}]", dict_increment).as_bytes()).ok();
    writer.write("\r\n".as_bytes()).expect("");
}

fn get_attribute(attributes: Attributes, search_term: &[u8]) -> Result<String, String> {
    // Get the select attribute by keyword
    for attr in attributes {
        let val = attr.ok().expect("Attribute");
        if val.key == search_term {
            let value_as_string = str::from_utf8(val.value.as_ref()).expect("").to_string();
            return Ok(value_as_string);
        }
    }
    Err("Error: attribute not found".to_string())
}

pub fn export_to_gml(input_graphml: &Path, output_path: &Path) {
    // Export graphml from given path to a gml graph at output path
    let mut in_data = false;
    let mut current_data_key = String::new(); //&[u8]; // = "";
    let mut keys: HashMap<String, Key> = HashMap::new();

    let mut reader = Reader::from_file(input_graphml).expect("Issue reading from path");
    let output_file = File::create(output_path).expect("Unable to create file");
    let mut writer = BufWriter::new(&output_file);
    let mut state = CurrentState::InGraph;
    let mut buf = Vec::new();

    let mut graph_info_added = false;

    let mut current_node = Node {
        id: "".to_string(),
        data: vec![],
    };

    let mut current_edge = Edge {
        source: Default::default(),
        target: Default::default(),
        data: Default::default(),
    };

    let mut current_graph = GraphInfo {
        directed: false,
        data: vec![],
    };

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Decl(_)) => {
                // Ignore the xml declaration
            }
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    b"graphml" => {
                        // ignore graphml tag attributes
                    }
                    b"graph" => {
                        // Get the directed value, with default to false
                        current_graph.directed =
                            match get_attribute(e.html_attributes(), b"edgedefault") {
                                Ok(value) => value == "directed",
                                Err(_) => false,
                            }
                    }
                    b"node" => {
                        if !graph_info_added {
                            // Add graph info when entering first node
                            write_graph_start(&mut writer, &current_graph);
                            graph_info_added = true;
                        }
                        current_node.id = get_attribute(e.html_attributes(), b"id").ok().expect("");
                        state = CurrentState::InNode;
                    }
                    b"edge" => {
                        current_edge.target = get_attribute(e.html_attributes(), b"target")
                            .ok()
                            .expect("");
                        current_edge.source = get_attribute(e.html_attributes(), b"source")
                            .ok()
                            .expect("");
                        state = CurrentState::InEdge;
                    }

                    b"data" => {
                        // get the key value when entering a data tag
                        current_data_key =
                            get_attribute(e.html_attributes(), b"key").ok().expect("");
                        in_data = true;
                    }
                    _ => {
                        panic!(
                            "Unsupported tag value {:?}",
                            str::from_utf8(e.name().as_ref()).expect("")
                        )
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                match e.name() {
                    b"graph" => {
                        write_graph_end(&mut writer);
                    }
                    b"node" => {
                        write_node(&mut writer, &current_node);
                        state = CurrentState::InGraph;
                        current_node.data.clear();
                    }
                    b"edge" => {
                        write_edge(&mut writer, &current_edge);
                        state = CurrentState::InGraph;
                        current_edge.data.clear();
                    }
                    b"data" => {
                        // Exit data state
                        in_data = false;
                    }
                    _ => (),
                }
            }
            Ok(Event::Empty(ref e)) => {
                match e.name() {
                    b"key" => {
                        let mut new_key = Key {
                            attr_name: "".to_string(),
                            attr_type: ValueTypes::String,
                            for_type: ForTypes::Edge,
                        };
                        let mut key_id = "".to_string();
                        for attr in e.html_attributes() {
                            let val = attr.ok().expect("Attribute");
                            match val.key {
                                b"attr.name" => {
                                    new_key.attr_name =
                                        str::from_utf8(val.value.as_ref()).expect("").to_string()
                                }
                                b"id" => {
                                    key_id =
                                        str::from_utf8(val.value.as_ref()).expect("").to_string()
                                }
                                b"attr.type" => match val.value.as_ref() {
                                    b"string" => new_key.attr_type = ValueTypes::String,
                                    b"double" => new_key.attr_type = ValueTypes::Double,
                                    b"float" => new_key.attr_type = ValueTypes::Float,
                                    b"int" => new_key.attr_type = ValueTypes::Int,
                                    _ => panic!("Error: Unrecognized value type!"),
                                },
                                b"for" => match val.value.as_ref() {
                                    b"edge" => new_key.for_type = ForTypes::Edge,
                                    b"node" => new_key.for_type = ForTypes::Node,
                                    b"graph" => new_key.for_type = ForTypes::Graph,
                                    _ => panic!("This for type is unsupported!"),
                                },
                                _ => (),
                            };
                        }
                        keys.insert(key_id.clone(), new_key.clone());
                    }
                    b"data" => {
                        // Ignore empty data tags
                    }
                    b"node" => {
                        // Ignore empty node tags
                    }
                    b"edge" => {
                        // Ignore empty edge tags
                    }
                    _ => (),
                }
            }
            // unescape and decode the text event using the reader encoding
            Ok(Event::Text(e)) => {
                // Extract the string data if in a data tag inside a node, edge or graph only
                if !in_data {
                    // Ignore text when not in data tag
                    continue;
                }
                let mut value = e
                    .unescape_and_decode(&reader)
                    .ok()
                    .expect("Error getting value");

                if value.is_empty() || value == "\"\"" {
                    // Skip empty values
                    continue;
                }
                // Get the attribute name and type from the current data key
                let key = keys.get(&*current_data_key).expect("Issue getting key");
                if key.attr_type == ValueTypes::String {
                    // Add quotes around value if it's a string
                    value = format!("\"{}\"", value);
                }
                match state {
                    CurrentState::InGraph => {
                        current_graph.data.push((key.attr_name.clone(), value));
                    }
                    CurrentState::InNode => {
                        current_node.data.push((key.attr_name.clone(), value));
                    }
                    CurrentState::InEdge => {
                        current_edge.data.push((key.attr_name.clone(), value));
                    }
                };
            }
            Ok(Event::Eof) => break, // exit the loop when reaching end of file
            Err(e) => {
                // Propagate error
                panic!("Error at position {}: {:?}", reader.buffer_position(), e)
            }
            _ => (), // Ignore other Events
        }

        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphml_to_gml;

    #[test]
    fn test_simple() {
        let filename = "./src/data/test_simple.graphml";
        let input_path = Path::new(filename);
        let output_path = Path::new("./src/result_simple.graphml");
        graphml_to_gml::export_to_gml(&input_path, &output_path);
    }

    #[test]
    fn test_complex() {
        let filename = "./src/data/test_complex.graphml";
        let input_path = Path::new(filename);
        let output_path = Path::new("./src/result_complex.graphml");
        graphml_to_gml::export_to_gml(&input_path, &output_path);
    }
}
