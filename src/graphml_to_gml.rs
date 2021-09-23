// Graphml to GML converter
// Note: This takes a decision to remove keys that have no values, ie. <data key="v"></data> will be omitted in the final gml
// Todo: use COW with [u8] instead of converting to string and back when writing

use quick_xml::events::attributes::Attributes;
use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::str;

#[derive(Debug, Clone)]
struct Node {
    id: u32,
    data: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
//struct Edge<'a> {
struct Edge {
    source: u32,
    target: u32, //Cow<'a, [u8]>, //String,
    data: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
struct GraphInfo {
    directed: Option<bool>,
    data: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
enum CurrentState {
    Graph,
    Node,
    Edge,
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
const LINE_BREAK: &[u8] = b"\n";

fn write_graph_start(writer: &mut BufWriter<&File>, graph: &GraphInfo) {
    // write graph specific content first then graph data, like nodes and edges
    writer.write(b"graph [").ok();
    writer.write(LINE_BREAK).ok();
    if let Some(value) = graph.directed {
        writer
            .write(format!("{}directed {}", INDENT_2, value as i8).as_bytes())
            .ok();
        writer.write(LINE_BREAK).ok();
    }
    write_data_items(writer, &graph.data, INDENT_2);
    // Note, not closing yet, as this will be done at the end
}

fn write_graph_end(writer: &mut BufWriter<&File>) {
    // Close graph after all nodes and edges have been written
    writer.write(b"]").ok();
}

fn write_edge(writer: &mut BufWriter<&File>, edge: &Edge) {
    // write edge specific content first then edge data
    writer.write(format!("{}edge [", INDENT_2).as_bytes()).ok();
    writer.write_all(LINE_BREAK).expect("");
    writer
        .write(format!("{}source {}", INDENT_4, edge.source).as_ref())
        .ok();
    writer.write_all(LINE_BREAK).expect("");
    writer
        .write(format!("{}target {}", INDENT_4, edge.target).as_ref())
        .ok();
    writer.write_all(LINE_BREAK).expect("");
    // Add data in a loop
    write_data_items(writer, &edge.data, INDENT_4);
    // Close node
    writer.write(format!("{}]", INDENT_2).as_bytes()).ok();
    writer.write_all(LINE_BREAK).expect("");
}

fn write_node(writer: &mut BufWriter<&File>, node: &Node) {
    // write node specific content first then node data
    writer.write(format!("{}node [", INDENT_2).as_bytes()).ok();
    writer.write_all(LINE_BREAK).expect("");
    writer
        .write(format!("{}id {}", INDENT_4, node.id).as_ref())
        .ok();
    writer.write_all(LINE_BREAK).expect("");
    // Add data in a loop
    write_data_items(writer, &node.data, INDENT_4);
    // Close node
    writer.write(format!("{}]", INDENT_2).as_bytes()).ok();
    writer.write_all(LINE_BREAK).expect("");
}

fn write_data_items(writer: &mut BufWriter<&File>, data: &[(String, String)], indent: &str) {
    // Write data items, while checking if the value is encoded json to unpack into lists and dicts
    for (key, value) in data.iter() {
        // check if the value is a dict or a list, which means it starts with "{ or "[ and ends with }" or ["
        let deserialized_value = {
            if (value.starts_with("\"[") && value.ends_with("]\""))
                || (value.starts_with("\"{") && value.ends_with("}\""))
            {
                // Remove outside quotes if that is the case
                let mut chars = value.chars();
                chars.next();
                chars.next_back();
                chars.as_str()
            } else {
                value
            }
        };

        // try parsing json and if it fails just add the whole string
        match serde_json::from_str(deserialized_value) {
            Result::Ok(map) => {
                write_value(writer, &map, key, indent);
            }
            Result::Err(_) => {
                let new_value = format!("{}{} {}", indent, key, deserialized_value);
                writer.write(new_value.as_bytes()).ok();
                writer.write_all(LINE_BREAK).expect("");
            }
        };
    }
}

fn get_value_with_increment(key: &str, item: &Value) -> String {
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

fn write_value(writer: &mut BufWriter<&File>, json: &Value, label: &str, indent: &str) {
    // Write serde value (list, dict, etc), with possible inside lists
    // todo: this should technically be recursive...
    // a [
    //   y 2
    //   z 1
    //   dict [
    //       a 1
    //       b 2
    //   ]
    //   list_with_name [
    //        i 1
    //        i 2
    //   ]
    //   list 1
    //   list 2
    //   ]

    match json {
        json if json.is_array() => {
            let array = json.as_array().expect("Issue retrieving array");
            if array.is_empty() {
                // Write value is string (ie. "[]" -> "[]" and not a list in GML
                writer
                    .write(format!("{}{} {}", indent, label, json).as_bytes())
                    .ok();
            } else {
                for (index, value_string) in array.iter().enumerate() {
                    // add value and incrementation
                    let key_value = format!("{}{} {}", INDENT_4, label, value_string);
                    writer.write(key_value.as_bytes()).ok();
                    if index < array.len() - 1 {
                        // dont add a line break after the last item
                        writer.write_all(LINE_BREAK).ok();
                    }
                }
            }
        }
        json if json.is_object() => {
            // if the value is a dict
            let dict_key_value = format!("{}{} [", INDENT_4, label);
            writer.write(dict_key_value.as_bytes()).ok();
            writer.write(LINE_BREAK).ok();
            for (key, value) in json.as_object().expect("Value isn't valid a dict object") {
                match value {
                    // Note: This should be recursively calling the top function
                    value if value.is_array() => {
                        // add key [ into the lines
                        for item in value.as_array().expect("").iter() {
                            // add value and incrementation (2 spaces)
                            let value_string = get_value_with_increment(key.as_str(), item);
                            writer
                                .write(format!("{}{}", INDENT_4, value_string).as_bytes())
                                .ok();
                            writer.write(LINE_BREAK).ok();
                        }
                    }
                    _ => {
                        // value is an object
                        let value_string = get_value_with_increment(key.as_str(), value);
                        writer
                            .write(format!("{}{}", INDENT_4, value_string).as_bytes())
                            .ok();
                        writer.write(LINE_BREAK).ok();
                    }
                }
            }
            // Close dict
            writer.write(format!("{}]", INDENT_4).as_bytes()).ok();
        }
        _ => {
            // else value is number or string or an indeterminate json type
            let value = format!("{}{} {}", indent, label, json);
            writer.write(value.as_bytes()).ok();
        }
    }

    writer.write(LINE_BREAK).ok();
}

fn get_attribute(attributes: Attributes, search_term: &[u8]) -> Result<String, String> {
    // Get the select attribute by keyword
    for attr in attributes {
        let val = attr.expect("Attribute");
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
    let mut state = CurrentState::Graph;
    let mut buf = Vec::new();

    let mut graph_info_added = false;

    let mut current_node = Node {
        id: 0,
        data: vec![],
    };

    let mut current_edge = Edge {
        source: Default::default(),
        target: Default::default(),
        data: Default::default(),
    };

    let mut current_graph = GraphInfo {
        directed: None,
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
                                Ok(value) => Some(value == "directed"),
                                Err(_) => None,
                            }
                    }
                    b"node" => {
                        if !graph_info_added {
                            // Add graph info when entering first node
                            write_graph_start(&mut writer, &current_graph);
                            graph_info_added = true;
                        }
                        let mut id: String = get_attribute(e.html_attributes(), b"id").expect("");
                        // Filter non numeric chars out - ie, in graphml id is n1 and should be 1 in gml
                        id = id.chars().filter(|c| c.is_digit(10)).collect();
                        current_node.id = id.parse::<u32>().expect("Issue Parsing id");
                        state = CurrentState::Node;
                    }
                    b"edge" => {
                        let mut target = get_attribute(e.html_attributes(), b"target").expect("");
                        target = target.chars().filter(|c| c.is_digit(10)).collect();
                        current_edge.target = target.parse::<u32>().expect("Issue Parsing id");

                        let mut source = get_attribute(e.html_attributes(), b"source").expect("");
                        source = source.chars().filter(|c| c.is_digit(10)).collect();
                        current_edge.source = source.parse::<u32>().expect("Issue Parsing id");
                        state = CurrentState::Edge;
                    }

                    b"data" => {
                        // get the key value when entering a data tag
                        current_data_key = get_attribute(e.html_attributes(), b"key").expect("");
                        in_data = true;
                    }
                    _ => {
                        panic!(
                            "Unsupported tag value {:?}",
                            str::from_utf8(e.name()).expect("")
                        )
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                match e.name() {
                    b"graph" => {
                        if !graph_info_added {
                            // Add graph info it never added (ie, no nodes present)
                            // TODO: this should be more robust
                            write_graph_start(&mut writer, &current_graph);
                            graph_info_added = true;
                        }
                        write_graph_end(&mut writer);
                    }
                    b"node" => {
                        write_node(&mut writer, &current_node);
                        state = CurrentState::Graph;
                        current_node.data.clear();
                    }
                    b"edge" => {
                        write_edge(&mut writer, &current_edge);
                        state = CurrentState::Graph;
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
                            let val = attr.expect("Attribute");
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
                let mut value = e.unescape_and_decode(&reader).expect("Error getting value");

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
                if [ValueTypes::Float, ValueTypes::Double].contains(&key.attr_type)
                    && !value.contains('.')
                {
                    // Add 1 decimal 0 to a value if it is formatted like an int ie. 1 -> 1.0
                    // This is because GML floats always have a decimal.
                    value = format!("{:.1}", value.parse::<f64>().expect("Issue parsing value"));
                }
                match state {
                    CurrentState::Graph => {
                        current_graph.data.push((key.attr_name.clone(), value));
                    }
                    CurrentState::Node => {
                        current_node.data.push((key.attr_name.clone(), value));
                    }
                    CurrentState::Edge => {
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
    extern crate test;
    use test::Bencher;

    #[test]
    fn test_simple() {
        let input_path = Path::new("./src/data/test_simple.graphml");
        let output_path = Path::new("./src/result_simple.gml");
        export_to_gml(&input_path, &output_path);
    }

    #[test]
    fn test_complex() {
        let input_path = Path::new("./src/data/test_complex.graphml");
        let output_path = Path::new("./src/result_complex.gml");
        export_to_gml(&input_path, &output_path);
    }

    #[bench]
    fn bench_complex(b: &mut Bencher) {
        let input_path = Path::new("./src/data/test_complex.graphml");
        let output_path = Path::new("./src/result_complex.gml");
        b.iter(|| {
            export_to_gml(&input_path, &output_path);
        });
    }
}
