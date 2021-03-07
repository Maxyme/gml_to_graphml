/*
GML to graphml converter.

Details:
Keep none as empty attributes (like networkx)
Convert lists to xml lists (unlike networkx which crashes for this step)

TODO
Need to check if large files can be loaded into a string as to not take a lot of memory, otherwise use a bufreader

Nice to have:
Btreemap for keys so that they are displayed in order
Keys at the top of the file

Nom: this is very interesting:
https://github.com/Geal/nom/blob/master/doc/choosing_a_combinator.md

URLS for info
https://stackoverflow.com/questions/45882329/read-large-files-line-by-line-in-rust
https://depth-first.com/articles/2020/07/20/reading-sd-files-in-rust/
 */

use itertools::Itertools;
use std::time::Instant;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter};

#[derive(Debug, Clone)]
struct Node<'a> {
    id: &'a str,
    attributes: HashMap<&'a str, &'a str>,
}

#[derive(Debug, Clone)]
struct Edge<'a> {
    id: &'a str,
    source: &'a str,
    target: &'a str,
    attributes: HashMap<&'a str, &'a str>,
}

#[derive(Debug, Clone)]
struct Graph<'a> {
    name: &'a str,
    version: f32,
    directed: bool,
    nodes: Vec<u8>,
    edges: Vec<u8>,
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
    Double,
    String,
}
impl GraphmlAttributeTypes {
    fn value(&self) -> &str {
        match *self {
            GraphmlAttributeTypes::Int => "int",
            GraphmlAttributeTypes::Double => "double",
            GraphmlAttributeTypes::String => "string",
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


fn add_header(writer: &mut Writer<BufWriter<&File>>, directed: bool) {
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

    // Open a graph node. Todo: add directed variable for graph
    let mut elem = BytesStart::borrowed_name("graph".as_bytes());
    let directed = match directed {
        true => "directed",
        false => "undirected",
    };
    elem.push_attribute(("edgedefault", directed));

    writer
        .write_event(Event::Start(elem))
        .expect("Unable to write data");
}

fn add_graph_info(writer: &mut Writer<BufWriter<&File>>, graph_name: &str, key_name: &str) {
    // Add the graph name: <data key="d0">Test gml file</data>

    let mut elem = BytesStart::borrowed_name("data".as_bytes());
    elem.push_attribute(("key", key_name));
    let text = BytesText::from_plain_str(graph_name);
    writer
        .write_event(Event::Start(elem))
        .expect("Unable to write data");
    writer
        .write_event(Event::Text(text))
        .expect("Unable to write data");
    writer.write_event(Event::End(BytesEnd::borrowed(b"data"))).ok();
}

fn add_footer(writer: &mut Writer<BufWriter<&File>>) {
    // Close the graph and graphml xml nodes
    writer.write_event(Event::End(BytesEnd::borrowed(b"graph"))).ok();
    writer.write_event(Event::End(BytesEnd::borrowed(b"graphml"))).ok();
}

fn add_node(
    writer: &mut Writer<BufWriter<&File>>,
    node_id: u32,
    node_data: &Vec<(String, String)>,
) {
    // Add a new xml node: <node id="1"><data key="d0">1.0</data></node>
    let name = "node".as_bytes();
    let mut node = BytesStart::borrowed_name(name);
    node.push_attribute(("id", node_id.to_string().as_str())); // todo, double conversion ???
    add_elem_with_keys(writer, node_data, node, name);
}

fn add_edge(
    writer: &mut Writer<BufWriter<&File>>,
    source: u32,
    target: u32,
    edge_data: &Vec<(String, String)>,
) {
    // Add a new xml edge: <edge source="1" target="2"><data key="d1">1.1</data></edge>
    let name = "edge".as_bytes();
    let mut edge = BytesStart::borrowed_name(name);
    edge.push_attribute(("source", source.to_string().as_str())); // todo, double conversion ???
    edge.push_attribute(("target", target.to_string().as_str())); // todo, double conversion ???
    add_elem_with_keys(writer, edge_data, edge, name);
}

fn add_elem_with_keys(
    writer: &mut Writer<BufWriter<&File>>,
    elem_data: &Vec<(String, String)>,
    elem: BytesStart,
    elem_name: &[u8],
) {
    // Add a xml element with data keys
    writer
        .write_event(Event::Start(elem))
        .expect("Unable to write data");
    for (key, value) in elem_data {
        let mut data = BytesStart::borrowed_name("data".as_bytes());
        data.push_attribute(("key", key.as_str()));
        writer
            .write_event(Event::Start(data))
            .expect("Unable to write data");
        let text = BytesText::from_plain_str(value.as_str());
        writer
            .write_event(Event::Text(text))
            .expect("Unable to write data");
        writer.write_event(Event::End(BytesEnd::borrowed(b"data"))).ok();
    }
    writer.write_event(Event::End(BytesEnd::borrowed(elem_name))).ok();
}

fn add_keys(writer: &mut Writer<BufWriter<&File>>, keys: &HashMap<KeyAttributes, KeyValues>) {
    // Write the list of xml keys
    // <key id="d10" for="edge" attr.name="list" attr.type="string" />
    for (key, value) in keys.iter() {
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

fn export_to_graphml(input_gml: &File, output_path: &File) {
    // export the graph to graphml at the output path destination
    // Todo: check if instantiating a bufwriter with a bigger capacity makes it faster for large files

    let writer = BufWriter::new(output_path);
    let mut xml_writer = Writer::new_with_indent(writer, ' ' as u8, 2);

    let buf_reader = BufReader::new(input_gml);

    // Current node info - todo: use struct
    let mut node_id: u32 = 1;
    let mut node_data: Vec<(String, String)> = Vec::new();

    // Current edge info - todo: use struct
    let mut edge_source: u32 = 1;
    let mut edge_target: u32 = 1;
    let mut edge_data: Vec<(String, String)> = Vec::new();

    // Key info
    let mut keys: HashMap<KeyAttributes, KeyValues> = HashMap::new();
    let mut key_count: u32 = 0;

    // Current graph info - todo: use struct
    let mut graph_tile: String = "Graph Title".to_string();
    let mut directed = false;
    let graph_key = "d0".to_string();

    // Current state - Todo: use match with enum or state machine
    let mut in_graph = false;
    let mut in_node = false;
    let mut in_edge = false;
    let mut in_dict = false;
    let mut in_list = false;

    for line in buf_reader.lines() {
        let line = line.expect("Unable to read line");
        // todo: match enum
        if line.trim().starts_with("#") {
            // skip comments - Note, comments could be added directly?
            continue;
        }

        if line.contains("node") {
            //entering node data
            node_data.clear();
            if in_graph {
                // Push the graph data first before parsing nodes - todo, can also be pushed last, but weird?
                in_graph = false;
                add_header(&mut xml_writer, directed);
                add_graph_info(&mut xml_writer, graph_tile.as_str(), &graph_key);
                // Add graph keys
                let key_values = KeyValues {
                    id: format!("d{}", key_count),
                    attr_type: GraphmlAttributeTypes::String,
                };
                let key_att = KeyAttributes {
                    attr_name: "label".to_string(),
                    for_elem: GraphmlElems::Graph,
                };
                keys.insert(key_att, key_values);
                key_count += 1;
            }
            in_node = true;
            in_edge = false;
        } else if line.contains("graph") {
            // entering graph
            in_graph = true;
        } else if line.contains("edge") {
            // entering edge
            edge_data.clear();
            in_node = false;
            in_edge = true;
        } else if line.contains("[") {
            // ignore opening lines for now (could be list item, so need to fix)
            continue;
        } else if line.contains("]") {
            // End previous open item (node, edge, graph, or in-items dict or list)
            // if type in [node, edge, graph], add the data
            if in_list {
                in_list = false;
            } else if in_dict {
                in_dict = false;
            }
            // not needed?
            // else if in_graph {
            //     // !in_node && !in_edge && !in_dict && !in_list {
            //     // Exiting the
            //     in_graph = false;
            //     continue;
            //    }
            else if in_edge {
                // Add edge when exiting an edge
                add_edge(&mut xml_writer, edge_source, edge_target, &edge_data);
                in_edge = false;
            } else if in_node {
                // Add node and increment node id when exiting node
                add_node(&mut xml_writer, node_id, &node_data);
                node_id += 1;
                in_node = false;
            }
            continue;
        } else if in_graph {
            // Add graph attributes
            if line.contains(" label ") {
                let (_, title) = parse_data_line(&line);
                graph_tile = title;
            } else if line.contains(" directed ") {
                let (_, value) = parse_data_line(&line);
                directed = value == "1";
            }
            continue;
        } else if in_node {
            // Add attributes to a node
            if line.contains(" id ") {
                node_id = line
                    .trim()
                    .splitn(2, char::is_whitespace)
                    .last()
                    .expect("")
                    .parse()
                    .expect("");
                continue;
            }

            let (node_attribute_label, value) = parse_data_line(&line);

            // Add or update keys for attribute
            let key_id = get_key_id(
                &mut keys,
                &mut key_count,
                node_attribute_label,
                &value,
                GraphmlElems::Node,
            );

            // Add node attributes
            node_data.push((key_id, value))
        } else if in_edge {
            // Add attributes to an edge
            if line.contains("source") {
                edge_source = line
                    .trim()
                    .splitn(2, char::is_whitespace)
                    .last()
                    .expect("")
                    .parse()
                    .expect("Cannot parse source into int");
                continue;
            }
            if line.contains("target") {
                edge_target = line
                    .trim()
                    .splitn(2, char::is_whitespace)
                    .last()
                    .expect("")
                    .parse()
                    .expect("Cannot parse target into int");
                continue;
            }
            let (edge_attribute_label, value) = line
                .trim()
                .splitn(2, char::is_whitespace)
                .map(|x| x.to_string())
                .collect_tuple()
                .expect("Issues...");

            // Add or update keys for attribute
            let key_id = get_key_id(
                &mut keys,
                &mut key_count,
                edge_attribute_label,
                &value,
                GraphmlElems::Edge,
            );
            edge_data.push((key_id, value)) // todo: copy instead?
        }
    }

    add_keys(&mut xml_writer, &keys);
    add_footer(&mut xml_writer);
}

fn parse_data_line(line: &String) -> (String, String) {
    // Parse a single data line to extract name a value
    // TODO extract dicts, list too...
    let (name, value): (String, String) = line
        .trim()
        .splitn(2, char::is_whitespace)
        .map(|x| x.to_string())
        .collect_tuple()
        .expect("Issues...");

    (name, value)
}

fn get_key_id(
    keys: &mut HashMap<KeyAttributes, KeyValues>,
    key_count: &mut u32,
    node_attribute_label: String,
    value: &String,
    for_elem: GraphmlElems,
) -> String {
    // Get the key id if it already exists otherwise create a new key id
    let key_attr = KeyAttributes {
        attr_name: node_attribute_label,
        for_elem,
    };

    let key_id = {
        match keys.get(&key_attr) {
            Some(values) => values.id.clone(),
            None => {
                // Check to see if it's a number
                let mut attribute_type = GraphmlAttributeTypes::String;
                let is_num = value.parse::<f64>().is_ok();
                if is_num {
                    // Todo: should we check for int too? Also, should we assume that future values of int like are ints?
                    // Ie: if a =1 then later b=1.1, ??
                    attribute_type = GraphmlAttributeTypes::Double;
                }

                let values = KeyValues {
                    id: format!("d{}", key_count),
                    attr_type: attribute_type,
                };
                let id = values.id.clone();
                keys.insert(key_attr, values);
                *key_count += 1;
                id
            }
        }
    };
    key_id
}

fn main() {
    //let filename = "/home/max/Desktop/GML Data Samples/32140213_v5.gml";
    // let filename = "./src/test_complex.gml";
    let filename = "./src/test_simple.gml";
    let input_file = File::open(filename).expect("Issue reading file at path");

    let output_path = "./src/result.graphml";
    let output_file = File::create(output_path).expect("Unable to create file");

    let extension = Path::new(filename)
        .extension()
        .and_then(OsStr::to_str)
        .expect("Error: File extension could not be detected!");

    match extension {
        "gml" => {
            println!("Converting gml file into graphml");
            let before = Instant::now();
            export_to_graphml(&input_file, &output_file);
            println!("Elapsed time: {:.2?}", before.elapsed());
        }
        "graphml" => {
            println!("Converting graphml file into.gml");
        }
        _ => panic!("Unexpected file format"),
    }
}
