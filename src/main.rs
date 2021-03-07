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

use std::time::Instant;

use std::borrow::Cow;


use quick_xml::Writer;
use quick_xml::Reader;
use quick_xml::events::{Event, BytesEnd, BytesStart, BytesDecl, BytesText};
use std::io::Cursor;

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::ffi::OsStr;
use std::path::PathBuf;
use graph_io_gml::parse_gml;

use std::fs::File;
use std::io::{BufReader, BufRead, BufWriter, Write, LineWriter};
use std::borrow::Borrow;
use nom::AsChar;

#[derive(Debug, Clone)]
struct Node<'a> {
    id : &'a str,
    attributes: HashMap<&'a str, &'a str>
}

#[derive(Debug, Clone)]
struct Edge<'a> {
    id : &'a str,
    source: &'a str,
    target: &'a str,
    attributes: HashMap<&'a str, &'a str>
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

fn add_header(writer: &mut Writer<BufWriter<&File>>) {
    // Write the Graphml header
    // Add the xml declaration
    let header = BytesDecl::new("1.0".as_bytes(), Some("UTF-8".as_bytes()), Some("yes".as_bytes()));
    writer.write_event(Event::Decl(header)).expect("Unable to write data");

    // Open the graphml node and add the boilerplate attributes
    let mut elem = BytesStart::borrowed_name("graphml".as_bytes()); //(b"graphml".to_vec(), "graphml".len());
    elem.push_attribute(("xmlns", "http://graphml.graphdrawing.org/xmlns"));
    elem.push_attribute(("xmlns", "http://www.w3.org/2001/XMLSchema-instance"));
    elem.push_attribute(("xsi:schemaLocation", "http://graphml.graphdrawing.org/xmlns http://graphml.graphdrawing.org/xmlns/1.0/graphml.xsd"));
    writer.write_event(Event::Start(elem)).expect("Unable to write data");

    // Open a graph node. Todo: add directed variable for graph
    let mut elem = BytesStart::borrowed_name("graph".as_bytes()); //b"graph".to_vec(), "graph".len());
    elem.push_attribute(("edgedefault", "undirected"));
    writer.write_event(Event::Start(elem)).expect("Unable to write data");
}

fn add_graph_info(writer: &mut Writer<BufWriter<&File>>, graph_name: &str, key_name: &str) {
    // Add the graph name and key information tag ie. <data key="d0">Test gml file</data>
    let mut elem = BytesStart::borrowed_name("data".as_bytes()); //b"data".to_vec(), "data".len());
    elem.push_attribute(("key", key_name));
    let mut text = BytesText::from_plain_str(graph_name);//b"data".to_vec(), "data".len());
    writer.write_event(Event::Start(elem)).expect("Unable to write data");
    writer.write_event(Event::Text(text)).expect("Unable to write data");
    writer.write_event(Event::End(BytesEnd::borrowed(b"data")));
}

fn add_footer(writer: &mut Writer<BufWriter<&File>>) {
    // Close the graph and graphml xml nodes
    writer.write_event(Event::End(BytesEnd::borrowed(b"graph")));
    writer.write_event(Event::End(BytesEnd::borrowed(b"graphml")));
}

fn add_node(writer: &mut Writer<BufWriter<&File>>, node_id: &str, keys: &Vec<(&str, &str)>) {
    // Add a xml node
    //    <node id="1">
    //       <data key="d0">1.0</data>
    //     </node>
    let mut node = BytesStart::borrowed_name("node".as_bytes());
    node.push_attribute(("id", node_id));
    writer.write_event(Event::Start(node)).expect("Unable to write data");
    for (key, value) in keys {
        let mut data = BytesStart::borrowed_name("data".as_bytes());
        data.push_attribute(("key", *key));
        writer.write_event(Event::Start(data)).expect("Unable to write data");
        let mut text = BytesText::from_plain_str(value);
        writer.write_event(Event::Text(text)).expect("Unable to write data");
        writer.write_event(Event::End(BytesEnd::borrowed(b"data")));
    }
    writer.write_event(Event::End(BytesEnd::borrowed(b"node")));
}


fn add_edge(writer: &mut Writer<BufWriter<&File>>, source: &str, target: &str, keys: &Vec<(&str, &str)>) {
    // Add an xml edge
    //    <edge source="1" target="2">
    //       <data key="d1">1.1</data>
    //     </edge>
    let mut edge = BytesStart::borrowed_name("edge".as_bytes());
    edge.push_attribute(("source", source));
    edge.push_attribute(("target", target));
    writer.write_event(Event::Start(edge)).expect("Unable to write data");
    for (key, value) in keys {
        let mut data = BytesStart::borrowed_name("data".as_bytes());
        data.push_attribute(("key", *key));
        writer.write_event(Event::Start(data)).expect("Unable to write data");
        let mut text = BytesText::from_plain_str(value);
        writer.write_event(Event::Text(text)).expect("Unable to write data");
        writer.write_event(Event::End(BytesEnd::borrowed(b"data")));
    }
    writer.write_event(Event::End(BytesEnd::borrowed(b"edge")));
}

fn add_keys_struct(writer: &mut Writer<BufWriter<&File>>, keys: &Vec<Key>) {
    // Add the list of xml keys
    // Need to check if it's substantially slower to COW ?
    for (key ) in keys.into_iter() {
        //let key = &key;
        let mut elem = BytesStart::borrowed_name("key".as_bytes());
        elem.push_attribute(("id", key.id.borrow()));
        elem.push_attribute(("for", key.for_type.borrow()));
        elem.push_attribute(("attr.name", key.attr_name.borrow()));
        elem.push_attribute(("attr.type", key.attr_type.borrow()));
        writer.write_event(Event::Empty(elem)).expect("Unable to write data");
    }
}

fn add_keys(writer: &mut Writer<BufWriter<&File>>, keys: &Vec<(&str, &str, &str, &str)>) {
    // Add the list of xml keys
    // <key id="d10" for="edge" attr.name="list" attr.type="string" />
    // Todo: pass a list of key structs
    for (id, for_type, attr_name, attr_type ) in keys {
    //for (key ) in keys {
        let mut elem = BytesStart::borrowed_name("key".as_bytes()); //b"data".to_vec(), "data".len());
        elem.push_attribute(("id", *id));
        elem.push_attribute(("for", *for_type));
        elem.push_attribute(("attr.name", *attr_name));
        elem.push_attribute(("attr.type", *attr_type));
        writer.write_event(Event::Empty(elem)).expect("Unable to write data");
        //writer.write_event(Event::End(BytesEnd::borrowed(b"key")));
    }
}

fn export_to_graphml(input_gml: &File, output_path: &File) {
    // export the graph to graphml at the output path destination
    // Todo: check if instantiating a bufwriter with a bigger capacity makes it faster for large files

    let mut writer = BufWriter::new(output_path);
    let mut xml_writer = Writer::new_with_indent(writer, ' ' as u8, 2);
    add_header(&mut xml_writer);

    let before = Instant::now();
    let buf_reader = BufReader::new(input_gml);
    //let count = reader.lines().fold(0, |sum, _| sum + 1);
    let mut count: usize = 0;
    let mut in_graph = true;
    add_graph_info(&mut xml_writer, &"title", &"d0");

    //let mut keys: Vec<Key> = Vec::new();
    let mut keys: Vec<(&str, &str, &str, &str)> = Vec::new();  //vec![("s", "u", "v", "w"), ("s", "u", "v", "w"), ("s", "u", "v", "w")];
    for line in buf_reader.lines() {
        let line = line.expect("Unable to read line");
        if line.contains("node [") {
            count +1;
        }
        // if in_graph && line.contains("label") {
        //     //add_boilerplate_header(&mut xml_writer);
        //     add_graph_info(&mut xml_writer, &"title", &"d0");
        //     in_graph = false;
        // }
        // buf_writer.write_all(line.as_bytes()).expect("Unable to write data");
        // writer.write_all(line.as_bytes()).expect("Unable to write data");
        // writer.write_all(b"\n");
        //println!("Line: {}", line);
        // Todo: use node and edge objects
        let node_attributes = vec![("s", "u")];
        add_node(&mut xml_writer, "testr", &node_attributes);
        let edge_attributes = vec![("s", "u")];
        add_edge(&mut xml_writer, "d0", "d1", &edge_attributes);
        //let new_key
        // keys.push(Key{
        //     id: Cow::Borrowed("baz"),
        //     attr_name: Cow::Borrowed("baz"),
        //     for_type: Cow::Borrowed("baz"),
        //     attr_type: Cow::Borrowed("baz"),
        // })
        keys.push(("s", "u", "v", "w"))
    }

    add_keys(&mut xml_writer, &keys);
    add_footer(&mut xml_writer);
    println!("line count: {}", count);
    println!("Elapsed time: {:.2?}", before.elapsed());


}

fn gml_to_graphml(input: &String) -> Graph {
    // convert a gml to graphml
    let mut g = Graph {
        name: "",
        version: 0.0,
        directed: false,
        nodes: vec![],
        edges: vec![]
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


    let name = "/home/max/Desktop/GML Data Samples/32140213_v5.gml";
    let filename = "./src/test_simple.gml";
    let input_file = File::open(filename).expect("Issue reading file at path");

    let output_path = "./src/result.graphml";
    let output_file = File::create(output_path).expect("Unable to create file");

    let input = fs::read_to_string(filename).expect("Error reading file");
    let extension = Path::new(filename).extension().and_then(OsStr::to_str).expect("Error: File extension could not be detected!");

    let f = File::open(filename).expect("Issue reading file at path");
    let mut reader = BufReader::new(f);

    let mut line = String::new();
    let len = reader.read_line(&mut line).expect("Issue reading line");
    println!("First line is {} bytes long", len);


    match extension {
        "gml" => {
            println!("Converting gml file into graphml");
            // convert_gml_to_graphml(&input, output_path)
            let graph: Graph = gml_to_graphml(&input);
            export_to_graphml(&input_file, &output_file);
        },
        "graphml" => {
            println!("Converting graphml file into.gml");
        },
        _ => panic!("Unexpected file format")
    }

}
