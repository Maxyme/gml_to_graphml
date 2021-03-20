/*
GML to graphml converter.

Details:
Keep none as empty attributes (like networkx)
Convert lists to xml lists (unlike networkx which crashes for this step)

URLS for info
https://stackoverflow.com/questions/45882329/read-large-files-line-by-line-in-rust
https://depth-first.com/articles/2020/07/20/reading-sd-files-in-rust/
 */

use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::time::Instant;

mod gml_to_graphml;
mod graphml_to_gml;

fn main() {
    // use executable like this: gml_to_graphml input_path output_path
    let args: Vec<String> = env::args().collect();
    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);

    let extension = input_path
        .extension()
        .and_then(OsStr::to_str)
        .expect("Error: File extension could not be detected!");

    let before = Instant::now();
    match extension {
        "gml" => {
            println!("Converting gml file to graphml");
            gml_to_graphml::export_to_graphml(&input_path, &output_path);
        }
        "graphml" => {
            println!("Converting graphml file to gml");
            graphml_to_gml::export_to_gml(&input_path, &output_path);
        }
        _ => panic!("Unexpected input file format (only .gml or .graphml files supported)"),
    }
    println!("Elapsed time: {:.2?}", before.elapsed());
}
