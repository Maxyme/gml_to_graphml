// Graphml to GML converter

use std::fs::File;
use tempfile::NamedTempFile;
use std::io::BufWriter;
use quick_xml::Reader;
use quick_xml::events::Event;
use std::path::Path;
use std::ffi::OsStr;

pub fn export_to_gml(input_gml: &File, output_file: &mut File) {
    let tmp_file = NamedTempFile::new().expect("Error creating temp file");
    let writer = BufWriter::new(tmp_file.as_file());

    let file_path = Path::new("./src/test_simple.graphml");
    let mut reader = Reader::from_file(file_path).expect("Issue reading from path");


    let mut count = 0;
    let mut txt = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf) {
            // for triggering namespaced events, use this instead:
            // match reader.read_namespaced_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                // for namespaced:
                // Ok((ref namespace_value, Event::Start(ref e)))
                match e.name() {
                    b"tag1" => println!("attributes values: {:?}",
                                        e.attributes().map(|a| a.unwrap().value)
                                            .collect::<Vec<_>>()),
                    b"tag2" => count += 1,
                    _ => (),
                }
            },
            // unescape and decode the text event using the reader encoding
            Ok(Event::Text(e)) => txt.push(e.unescape_and_decode(&reader).unwrap()),
            Ok(Event::Eof) => break, // exits the loop when reaching end of file
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // There are several other `Event`s we do not consider here
        }

        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }
    println!("{:?}", txt);
}