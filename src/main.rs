#![feature(test)]

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
use std::path::Path;
use std::time::Instant;

mod gml_to_graphml;
mod graphml_to_gml;

use clap::{App, Arg};

fn main() {
    let matches = App::new("Graph converter")
        .version("0.1.3")
        .about("Graph file converter between gml and graphml formats")
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file path to use")
                .short("i")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("Sets the output file path to use")
                .short("o")
                .required(true)
                .index(2),
        )
        .get_matches();

    let input = matches.value_of("INPUT").unwrap();
    let input_path = Path::new(input);
    let output = matches.value_of("OUTPUT").unwrap();
    let output_path = Path::new(output);

    println!("Using input file path: {}", input);

    // Convert files at given paths
    let extension = input_path
        .extension()
        .and_then(OsStr::to_str)
        .expect("Error: File extension could not be detected!");

    let before = Instant::now();
    match extension {
        "gml" => {
            println!("Converting gml file to graphml");
            gml_to_graphml::export_to_graphml(input_path, output_path);
        }
        "graphml" => {
            println!("Converting graphml file to gml");
            graphml_to_gml::export_to_gml(input_path, output_path);
        }
        _ => panic!("Unexpected input file format (only .gml or .graphml files supported)"),
    }
    println!("Elapsed time: {:.2?}", before.elapsed());
}
