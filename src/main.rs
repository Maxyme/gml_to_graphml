/*
GML to graphml converter.

Details:
Keep none as empty attributes (like networkx)
Convert lists to xml lists (unlike networkx which crashes for this step)

URLS for info
https://stackoverflow.com/questions/45882329/read-large-files-line-by-line-in-rust
https://depth-first.com/articles/2020/07/20/reading-sd-files-in-rust/
 */

use std::ffi::OsStr;

use std::time::Instant;

use std::fs::File;
use std::path::Path;
mod gml_to_graphml;
mod graphml_to_gml;

fn main() {
    //let filename = "/home/max/Desktop/GML Data Samples/32161455.gml";
    let filename = "./src/test_complex.gml";
    //let filename = "./src/test_simple.gml";
    //let filename = "./src/test_simple.graphml";
    let input_file = File::open(filename).expect("Issue reading file at path");

    let output_path = "./src/result.graphml";
    let mut output_file = File::create(output_path).expect("Unable to create file");

    let extension = Path::new(filename)
        .extension()
        .and_then(OsStr::to_str)
        .expect("Error: File extension could not be detected!");
    let before = Instant::now();
    match extension {
        "gml" => {
            println!("Converting gml file into graphml");
            gml_to_graphml::export_to_graphml(&input_file, &mut output_file);
        }
        "graphml" => {
            println!("Converting graphml file into.gml");
            graphml_to_gml::export_to_gml(&input_file, &mut output_file);
        }
        _ => panic!("Unexpected file format"),
    }
    println!("Elapsed time: {:.2?}", before.elapsed());
}
