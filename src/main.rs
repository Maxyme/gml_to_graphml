/*
GML to graphml converter.

Details:
Keep none as empty attributes (like networkx)
Convert lists to xml lists (unlike networkx which crashes for this step)

TODO
Need to check if large files can be loaded into a string as to not take a lot of memory, otherwise use a bufreader

Nom: this is very interesting:
https://github.com/Geal/nom/blob/master/doc/choosing_a_combinator.md

URLS for info
https://stackoverflow.com/questions/45882329/read-large-files-line-by-line-in-rust
https://depth-first.com/articles/2020/07/20/reading-sd-files-in-rust/
 */

use itertools::Itertools;
use std::time::Instant;

use std::borrow::Cow;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Reader;
use quick_xml::Writer;
use std::io::Cursor;

use graph_io_gml::parse_gml;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use nom::AsChar;
use std::borrow::Borrow;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, LineWriter, Write};

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

struct Foo<'a> {
    baz: Cow<'a, str>,
}

#[derive(Debug, Clone)]
struct Key<'a> {
    id: Cow<'a, str>,
    attr_name: Cow<'a, str>,
    for_type: Cow<'a, str>,
    attr_type: Cow<'a, str>,
    // <key id="d10" for="edge" attr.name="list" attr.type="string" />
    // <key id="d9" for="edge" attr.name="dictionary" attr.type="string" />
    // <key id="d8" for="edge" attr.name="value" attr.type="double" />
    // <key id="d7" for="node" attr.name="nodeitems" attr.type="string" />
}


// possible types are "int", "string", "double", "

enum GraphmlElems {
    node,
    edge,
    graph
}

enum GraphmlAttributeTypes {
    int,
    double,
    string
}

fn add_header(writer: &mut Writer<BufWriter<&File>>) {
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
    let mut elem = BytesStart::borrowed_name("graphml".as_bytes()); //(b"graphml".to_vec(), "graphml".len());
    elem.push_attribute(("xmlns", "http://graphml.graphdrawing.org/xmlns"));
    elem.push_attribute(("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"));
    elem.push_attribute(("xsi:schemaLocation", "http://graphml.graphdrawing.org/xmlns http://graphml.graphdrawing.org/xmlns/1.0/graphml.xsd"));
    writer
        .write_event(Event::Start(elem))
        .expect("Unable to write data");

    // Open a graph node. Todo: add directed variable for graph
    let mut elem = BytesStart::borrowed_name("graph".as_bytes()); //b"graph".to_vec(), "graph".len());
    elem.push_attribute(("edgedefault", "undirected"));
    writer
        .write_event(Event::Start(elem))
        .expect("Unable to write data");
}

fn add_graph_info(writer: &mut Writer<BufWriter<&File>>, graph_name: &str, key_name: &str) {
    // Add the graph name and key information tag ie. <data key="d0">Test gml file</data>
    let mut elem = BytesStart::borrowed_name("data".as_bytes()); //b"data".to_vec(), "data".len());
    elem.push_attribute(("key", key_name));
    let mut text = BytesText::from_plain_str(graph_name); //b"data".to_vec(), "data".len());
    writer
        .write_event(Event::Start(elem))
        .expect("Unable to write data");
    writer
        .write_event(Event::Text(text))
        .expect("Unable to write data");
    writer.write_event(Event::End(BytesEnd::borrowed(b"data")));
}

fn add_footer(writer: &mut Writer<BufWriter<&File>>) {
    // Close the graph and graphml xml nodes
    writer.write_event(Event::End(BytesEnd::borrowed(b"graph")));
    writer.write_event(Event::End(BytesEnd::borrowed(b"graphml")));
}

fn add_node(writer: &mut Writer<BufWriter<&File>>, node_id: u32, keys: &Vec<(String, String)>) {
    // Add a xml node
    //    <node id="1">
    //       <data key="d0">1.0</data>
    //     </node>
    let mut node = BytesStart::borrowed_name("node".as_bytes());
    node.push_attribute(("id", node_id.to_string().as_str())); // todo, double conversion ???
    writer
        .write_event(Event::Start(node))
        .expect("Unable to write data");
    for (key, value) in keys{
        let mut data = BytesStart::borrowed_name("data".as_bytes());
        data.push_attribute(("key", key.as_str()));
        writer
            .write_event(Event::Start(data))
            .expect("Unable to write data");
        let mut text = BytesText::from_plain_str(value.as_str());
        writer
            .write_event(Event::Text(text))
            .expect("Unable to write data");
        writer.write_event(Event::End(BytesEnd::borrowed(b"data")));
    }
    writer.write_event(Event::End(BytesEnd::borrowed(b"node")));
}

fn add_edge(
    writer: &mut Writer<BufWriter<&File>>,
    source: u32,
    target: u32,
    keys: &Vec<(String, String)>,
) {
    // Add an xml edge
    //    <edge source="1" target="2">
    //       <data key="d1">1.1</data>
    //     </edge>
    let mut edge = BytesStart::borrowed_name("edge".as_bytes());
    edge.push_attribute(("source", source.to_string().as_str())); // todo, double conversion ???
    edge.push_attribute(("target", target.to_string().as_str())); // todo, double conversion ???
    writer
        .write_event(Event::Start(edge))
        .expect("Unable to write data");
    for (key, value) in keys {
        let mut data = BytesStart::borrowed_name("data".as_bytes());
        data.push_attribute(("key", key.as_str()));
        writer
            .write_event(Event::Start(data))
            .expect("Unable to write data");
        let mut text = BytesText::from_plain_str(value.as_str());
        writer
            .write_event(Event::Text(text))
            .expect("Unable to write data");
        writer.write_event(Event::End(BytesEnd::borrowed(b"data")));
    }
    writer.write_event(Event::End(BytesEnd::borrowed(b"edge")));
}

fn add_keys_struct(writer: &mut Writer<BufWriter<&File>>, keys: &Vec<Key>) {
    // Add the list of xml keys
    // Need to check if it's substantially slower to COW ?
    for (key) in keys.into_iter() {
        //let key = &key;
        let mut elem = BytesStart::borrowed_name("key".as_bytes());
        elem.push_attribute(("id", key.id.borrow()));
        elem.push_attribute(("for", key.for_type.borrow()));
        elem.push_attribute(("attr.name", key.attr_name.borrow()));
        elem.push_attribute(("attr.type", key.attr_type.borrow()));
        writer
            .write_event(Event::Empty(elem))
            .expect("Unable to write data");
    }
}

fn add_keys(writer: &mut Writer<BufWriter<&File>>, keys: &Vec<(String, String, String, String)>) {
    // Add the list of xml keys
    // <key id="d10" for="edge" attr.name="list" attr.type="string" />
    // Todo: pass a list of key structs - for type, attr_type can be enums items
    for (id, for_type, attr_name, attr_type) in keys {
        //for (key ) in keys {
        let mut elem = BytesStart::borrowed_name("key".as_bytes());
        elem.push_attribute(("id", id.as_str()));
        elem.push_attribute(("for", for_type.as_str()));
        elem.push_attribute(("attr.name", attr_name.as_str()));
        elem.push_attribute(("attr.type", attr_type.as_str()));
        writer
            .write_event(Event::Empty(elem))
            .expect("Unable to write data");
    }
}

fn export_to_graphml(input_gml: &File, output_path: &File) {
    // export the graph to graphml at the output path destination
    // Todo: check if instantiating a bufwriter with a bigger capacity makes it faster for large files

    let mut writer = BufWriter::new(output_path);
    let mut xml_writer = Writer::new_with_indent(writer, ' ' as u8, 2);
    add_header(&mut xml_writer);
    let buf_reader = BufReader::new(input_gml);

    let mut node_id: u32 = 1;
    let mut node_data: Vec<(String, String)> = Vec::new();

    let mut edge_source: u32 = 1;
    let mut edge_target: u32 = 1;
    let mut edge_data: Vec<(String, String)> = Vec::new();
    // Keys are (id, for_type, attr_name, attr_type) #  <key id="d10" for="edge" attr.name="list" attr.type="string" />

    let mut keys: Vec<(String, String, String, String)> = Vec::new(); //vec![("s", "u", "v", "w"), ("s", "u", "v", "w"), ("s", "u", "v", "w")];

    // let mut keys: HashMap<(String, String)>

    let mut graph_tile: String = "Graph Title".to_string();
    let graph_key = "d0".to_string();

    let mut in_graph = false;
    let mut in_node = false;
    let mut in_edge = false;
    let mut in_dict = false;
    let mut in_list = false;
    for line in buf_reader.lines() {
        let line = line.expect("Unable to read line");
        // todo: match enum
        if line.trim().starts_with("#") {
            // skip comments
            continue;
        }
        if line.contains(" node ") {
            //entering node data
            node_data.clear();
            if in_graph {
                // Push the graph data first before parsing nodes
                in_graph = false;
                add_graph_info(&mut xml_writer, graph_tile.as_str(), &graph_key);
                // Add to keys (&str, &str, &str, &str) -> (id, for_type, attr_name, attr_type) -> id="d0" for="graph" attr.name="label" attr.type="string"
                let graph_key = (graph_key.to_owned(), "graph".to_string(), "label".to_string(), "string".to_string());
                keys.push(graph_key);
            }
            in_node = true;
            in_edge = false;
        } else if line.contains(" graph ") {
            // entering graph
            in_graph = true;
        } else if line.contains(" edge ") {
            // entering edge
            edge_data.clear();
            in_node = false;
            in_edge = true;

        } else if line.contains(" [ ") {
            // ignore opening lines for now (could be list item, so need to fix)
            continue;
        } else if line.contains("]") {
            // End previous open item (node, edge, graph, or in-items dict or list)
            // if type in [node, edge, graph], add the data
            if in_list {
                in_list = false;
            }
            if in_dict {
                in_dict = false;
            }
            if !in_node && !in_edge && !in_dict && !in_list {
                // Exiting the graph
                continue;
            }
            if in_edge {
                // Add edge when exiting an edge
                //let edge_attributes = vec![("s", "u")];
                add_edge(&mut xml_writer, edge_source, edge_target, &edge_data);
                in_edge = false;
            }
            if in_node {
                // exiting node
                add_node(&mut xml_writer, node_id, &node_data);
                node_id += 1;
                in_node = false;
            }
            continue;
        } else if in_graph {
            // Add graph attributes
            if line.contains(" label ") {
                graph_tile = line
                    .trim()
                    .splitn(2, char::is_whitespace)
                    .last()
                    .expect("")
                    .to_string();
                continue;
            }
        } else if in_node {
            // Add attributes to a node
            if line.contains(" id ") {
                node_id = line.trim().splitn(2, char::is_whitespace).last().expect("").parse().expect("");
                continue;
            }
            // println!("{:?}", &line);
            let (node_attribute, value) = line
                .trim()
                .splitn(2, char::is_whitespace)
                .map(|x| x.to_string())
                .collect_tuple()
                .expect("Issues...");

            node_data.push((node_attribute, value)) // todo: copy instead?

        } else if in_edge {
            // Add attributes to an edge
            if line.contains("source") {
                edge_source = line.trim().splitn(2, char::is_whitespace).last().expect("").parse().expect("Cannot parse source into int");
                continue;
            }
            if line.contains("target") {
                edge_target = line.trim().splitn(2, char::is_whitespace).last().expect("").parse().expect("Cannot parse target into int");
                continue;
            }
            let (edge_attribue, value) = line
                .trim()
                .splitn(2, char::is_whitespace)
                .map(|x| x.to_string())
                .collect_tuple()
                .expect("Issues...");

            edge_data.push((edge_attribue, value)) // todo: copy instead?
        }
    }

    add_keys(&mut xml_writer, &keys);
    add_footer(&mut xml_writer);
}

fn gml_to_graphml(input: &String) -> Graph {
    // convert a gml to graphml
    let mut g = Graph {
        name: "",
        version: 0.0,
        directed: false,
        nodes: vec![],
        edges: vec![],
    };
    g

    // Get the graph information
}

// fn convert_gml_to_graphml_sortofworks(file: &String, output_path: &str) {
//     let graph = parse_gml(
//         file.as_str(),
//         //&|s| -> Option<f64> { Some(s.and_then(Sexp::get_float).unwrap_or(0.0)) },
//         &|_| -> Option<()> { Some(()) },
//         &|_| -> Option<()> { Some(()) }
//     );
//     // match with ok()
//
//     let pet_graph = graph.unwrap();
//     println!("{:?}", pet_graph.raw_nodes());
//     let graphml = GraphMl::new(&pet_graph).pretty_print(true); //.export_node_weights_display().export_edge_weights_display();
//     //println!("{:?}", graphml.to_string());
//     fs::write(output_path, graphml.to_string()).expect("Unable to write file");
// }

fn main() {
    // let file_path = PathBuf::from("./src/test_complex.gml"); // use small.txt to practice
    // let x = file_path.extension().expect("Error: File extension could not be detected!");
    // let f = fs::read_to_string(&file_path).expect("Error reading file");

    let filename = "/home/max/Desktop/GML Data Samples/32140213_v5.gml";
    // let filename = "./src/test_complex.gml";
    //let filename = "./src/test_simple.gml";
    let input_file = File::open(filename).expect("Issue reading file at path");

    let output_path = "./src/result.graphml";
    let output_file = File::create(output_path).expect("Unable to create file");

    let input = fs::read_to_string(filename).expect("Error reading file");
    let extension = Path::new(filename)
        .extension()
        .and_then(OsStr::to_str)
        .expect("Error: File extension could not be detected!");

    let f = File::open(filename).expect("Issue reading file at path");
    let mut reader = BufReader::new(f);

    let mut line = String::new();
    let len = reader.read_line(&mut line).expect("Issue reading line");
    println!("First line is {} bytes long", len);

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
