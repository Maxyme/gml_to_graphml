/*
GML to graphml converter.

Details:
Keep none as empty attributes (like networkx)
Convert lists to xml lists (unlike networkx which crashes for this step)

Usage: graphconverter input_path output_path

URLS for info
https://stackoverflow.com/questions/45882329/read-large-files-line-by-line-in-rust
https://depth-first.com/articles/2020/07/20/reading-sd-files-in-rust/
 */

use std::ffi::OsStr;
use std::path::PathBuf;
use std::time::Instant;

use clap::{value_parser, Arg, Command};
use graph_converter::{gml_to_graphml, graphml_to_gml};

fn main() {
    let matches = Command::new("Graph converter")
        .version("0.1.3")
        .about("Graph file converter between gml and graphml formats")
        .arg(
            Arg::new("INPUT")
                .help("Sets the input file path to use")
                .required(true)
                .index(1)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("OUTPUT")
                .help("Sets the output file path to use")
                .required(true)
                .index(2)
                .value_parser(value_parser!(PathBuf)),
        )
        .get_matches();

    let input_path = matches
        .get_one::<PathBuf>("INPUT")
        .expect("required by clap");
    let output_path = matches
        .get_one::<PathBuf>("OUTPUT")
        .expect("required by clap");

    println!("Using input file path: {}", input_path.display());

    // Convert files at given paths
    let extension = input_path
        .extension()
        .and_then(OsStr::to_str)
        .expect("Error: File extension could not be detected!");

    let before = Instant::now();
    match extension {
        "gml" => {
            println!("Converting gml file to graphml");
            gml_to_graphml::export_to_graphml(input_path.as_path(), output_path.as_path());
        }
        "graphml" => {
            println!("Converting graphml file to gml");
            graphml_to_gml::export_to_gml(input_path.as_path(), output_path.as_path());
        }
        _ => panic!("Unexpected input file format (only .gml or .graphml files supported)"),
    }
    println!("Elapsed time: {:.2?}", before.elapsed());
}
