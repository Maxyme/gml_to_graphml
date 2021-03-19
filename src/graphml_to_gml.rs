// Graphml to GML converter
// Note: This takes a decision to remove keys that have no values, ie. <data key="v"></data> will be omitted in the final gml
// Todo: use COW with [u8] instead of converting to string and back when writing

use std::fs::File;
use tempfile::NamedTempFile;
use std::io::{BufWriter, Write};
use quick_xml::Reader;
use quick_xml::events::Event;
use std::path::Path;
use std::ffi::OsStr;
use std::collections::HashMap;
use std::str;
use std::borrow::Cow;

// Todo: share these objects between the 2 classes?
#[derive(Debug, Clone)]
struct Node {
    id: String,
    data: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
//struct Edge<'a> {
struct Edge{
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
    Int
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ForTypes {
    Edge,
    Node
}

#[derive(Debug, Clone)]
struct Key {
    attr_name: String,
    attr_type: ValueTypes,
    for_type: ForTypes
}

fn write_graph_start(writer: &mut BufWriter<&File>, graph: &GraphInfo) {
    // write graph specific content first then graph data
    let graph_increment = " ".repeat(2);
    // Add graph data before adding nodes and edges
    writer.write("graph [".as_bytes()).ok();
    writer.write("\r\n".as_bytes()).ok();
    writer.write(format!("{}directed 0", graph_increment).as_bytes()).ok();  // todo: parse edgedefault="undirected"
    writer.write("\r\n".as_bytes()).ok();
    // Add data in a loop
    for (key, value) in graph.data.iter() {
        writer.write(format!("{}{} {}", graph_increment, key, value).as_ref()).ok();
        writer.write("\r\n".as_bytes()).expect("");
    }
}

fn write_graph_end(writer: &mut BufWriter<&File>) {
    // Close graph after all nodes and edges have been written
    writer.write("]".as_bytes()).ok();
}

fn write_edge(writer: &mut BufWriter<&File>, edge: &Edge) {
    // write edge specific content first then edge data
    let edge_increment = " ".repeat(2);
    let data_increment = " ".repeat(4);
    writer.write(format!("{}edge [", edge_increment).as_bytes()).ok();
    writer.write("\r\n".as_bytes()).expect("");
    writer.write(format!("{}source {}", data_increment, edge.source).as_ref()).ok();
    writer.write("\r\n".as_bytes()).expect("");
    // let v = String::from_utf8_lossy(edge.target.as_ref());
    // writer.write(format!("{:?}target {}", data_increment, v).as_ref()).ok();
    writer.write(format!("{}target {}", data_increment, edge.target).as_ref()).ok();
    writer.write("\r\n".as_bytes()).expect("");
    // Add data in a loop
    for (key, value) in edge.data.iter() {
        writer.write(format!("{}{} {}", data_increment, key, value).as_ref()).ok();
        writer.write("\r\n".as_bytes()).expect("");
    }
    // Close node
    writer.write(format!("{}]", edge_increment).as_bytes()).ok();
    writer.write("\r\n".as_bytes()).expect("");
}

fn write_node(writer: &mut BufWriter<&File>, node: &Node) {
    // write node specific content first then node data
    let node_increment = " ".repeat(2);
    let data_increment = " ".repeat(4);
    writer.write(format!("{}node [", node_increment).as_bytes()).ok();
    writer.write("\r\n".as_bytes()).expect("");
    writer.write(format!("{}id {}", data_increment, node.id).as_ref()).ok();
    writer.write("\r\n".as_bytes()).expect("");
    // Add data in a loop
    for (key, value) in node.data.iter() {
        writer.write(format!("{}{} {}", data_increment, key, value).as_ref()).ok();
        writer.write("\r\n".as_bytes()).expect("");
    }
    // Close node
    writer.write(format!("{}]", node_increment).as_bytes()).ok();
    writer.write("\r\n".as_bytes()).expect("");
}

pub fn export_to_gml(input_graphml: &Path, output_path: &Path) {
    // Export graphml from given path to a gml graph at output path
    let mut in_data = false;
    let mut current_data_key = String::new(); //&[u8]; // = "";
    let mut keys : HashMap<String, Key> = HashMap::new();


    let mut reader = Reader::from_file(input_graphml).expect("Issue reading from path");
    let mut output_file = File::create(output_path).expect("Unable to create file");
    let mut writer = BufWriter::new(&output_file);
    let mut state = CurrentState::InGraph;
    let mut buf = Vec::new();

    let mut graph_info_added = false;

    let mut current_node = Node {
        id: "".to_string(),
        data: vec![]
    };

    let mut current_edge = Edge {
        source: Default::default(), //"".to_string(),
        target: Default::default(),
        data: Default::default(), //vec![],
        //target: "".to_string()
    };

    let mut current_graph = GraphInfo {
        directed: false,
        data: vec![],
    };

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Decl(ref e)) => {
                // Ignore the xml declaration
                continue
            },
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    b"graph" => {
                        println!("Inside graph");
                        for attr in e.attributes() {
                            println!("{:?}", attr.ok());
                        }
                    },
                    b"node" => {
                        if !graph_info_added {
                            // Add graph info when entering first node
                            write_graph_start(&mut writer, &current_graph);
                            graph_info_added = true;
                        }
                        state = CurrentState::InNode;
                        // get current node id
                        for attr in e.html_attributes() {
                            let val = attr.ok().expect("Attribute");
                            if val.key == b"id" {
                                current_node.id = str::from_utf8(val.value.as_ref()).expect("").to_string();
                            }
                        }
                    },
                    b"edge" => {
                        println!("start edge");
                        for attr in e.attributes() {
                            println!("{:?}", attr.ok());
                        }
                        state = CurrentState::InEdge;
                        for attr in e.html_attributes() {
                            let val = attr.ok().expect("Attribute");
                            if val.key == b"target" {
                                current_edge.target = str::from_utf8(val.value.as_ref()).expect("").to_string();
                                //current_edge.target = val.value;
                            }
                            if val.key == b"source" {
                                current_edge.source = str::from_utf8(val.value.as_ref()).expect("").to_string();
                            }
                        }
                    },

                    b"data" => {
                        println!("Enter data {:?}", e);
                        //let mut key: &[u8];
                        for attr in e.html_attributes() {
                            let val = attr.ok().expect("Attribute");
                            if val.key == b"key" {
                                //current_data_key = val.value.as_ref();
                                current_data_key = str::from_utf8(val.value.as_ref()).expect("").to_string();
                            }
                        }
                        //current_data_key = key;
                        in_data = true;
                    },
                    _ => (),
                }
            },
            Ok(Event::End(ref e)) => {
                match e.name() {
                    b"graph" => {
                        write_graph_end(&mut writer);
                    },
                    b"node" => {
                        write_node(&mut writer, &current_node);
                        state = CurrentState::InGraph;
                        current_node.data.clear();
                    },
                    b"edge" => {
                        write_edge(&mut writer, &current_edge);
                        state = CurrentState::InGraph;
                        current_edge.data.clear();
                    },
                    b"data" => {
                        // Exit data state // todo: use a in_data state and go back to previous?
                        in_data = false;
                    },
                    _ => ()
                }
            },
            Ok(Event::Empty(ref e)) => {
                match e.name() {
                    b"key" => {
                        let mut new_key = Key {
                            attr_name: "".to_string(),
                            attr_type: ValueTypes::String,
                            for_type: ForTypes::Edge
                        };
                        let mut key_id = "".to_string();
                        for attr in e.html_attributes() {
                            let val = attr.ok().expect("Attribute");
                            let key = val.key;
                            // todo: keep str instead of string?

                            match key {
                                b"attr.name" => new_key.attr_name = str::from_utf8(val.value.as_ref()).expect("").to_string(),
                                b"id" => key_id = str::from_utf8(val.value.as_ref()).expect("").to_string(),
                                b"attr.type" => {
                                    match val.value.as_ref() {
                                        b"string" => new_key.attr_type = ValueTypes::String,
                                        b"double" => new_key.attr_type = ValueTypes::Double,
                                        b"float" => new_key.attr_type = ValueTypes::Float,
                                        b"int" => new_key.attr_type = ValueTypes::Int,
                                        _ => panic!("Error: Unrecognized value type!")
                                    }
                                },
                                _ => ()
                            };
                        }
                        keys.insert(key_id.clone(), new_key.clone());
                    },
                    b"data" => {
                        // Todo: ignore empty
                        panic!("Empty node or edge values not supported at the moment")
                    },
                    b"node" => {
                        panic!("Empty node or edge values not supported at the moment")
                    },
                    b"edge" => {
                        panic!("Empty node or edge values not supported at the moment")
                    }
                    _ => ()
                }
            },
            // unescape and decode the text event using the reader encoding
            Ok(Event::Text(e)) => {
                //let v = reader.decode(e).ok();
                let mut value = e.unescape_and_decode(&reader).ok().expect("Error getting value");

                if !in_data {
                    // Ignore text when not in data tag
                    continue
                    //println!("Data is {:?}", text.expect(""));
                }
                // Get the attribute name from the current data key
                let cur_key = str::from_utf8(current_data_key.clone().as_ref()).expect("").to_string();
                let key = keys.get(&*cur_key).expect("issue getting key");
                if key.attr_type == ValueTypes::String {
                    // Add quotes around value if it's a string
                    // todo: check for json here first, as this won't work
                    value = format!("\"{}\"", value);
                }
                match state {
                    CurrentState::InGraph => {
                        current_graph.data.push((key.attr_name.clone(), value));
                    },
                    CurrentState::InNode => {
                        current_node.data.push((key.attr_name.clone(), value));
                    },
                    CurrentState::InEdge => {
                        current_edge.data.push((key.attr_name.clone(), value));
                    },
                };
            },
            Ok(Event::Eof) => break, // exits the loop when reaching end of file
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // Ignore other Events
        }

        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }

    println!("{:?}", keys);
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
