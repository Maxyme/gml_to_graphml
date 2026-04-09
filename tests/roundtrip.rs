use std::fs;
use std::io;
use std::path::Path;

use graph_converter::{gml_to_graphml::export_to_graphml, graphml_to_gml::export_to_gml};
use tempfile::NamedTempFile;

fn assert_output_matches_fixture(
    output_path: &Path,
    expected_path: &Path,
    label: &str,
) -> io::Result<()> {
    let actual = fs::read_to_string(output_path)?;
    let expected = fs::read_to_string(expected_path)?;
    assert_eq!(&actual, &expected,
        "{label} output should match the golden fixture"
    );
    Ok(())
}

#[test]
fn converts_simple_gml_to_graphml() -> io::Result<()> {
    let input_path = Path::new("tests/data/simple.gml");
    let output_file = NamedTempFile::new()?;
    export_to_graphml(input_path, output_file.path());
    
    let expected_path = Path::new("tests/data/simple.graphml");
    assert_output_matches_fixture(output_file.path(), expected_path, "graphml")
}

#[test]
fn converts_simple_graphml_to_gml() -> io::Result<()> {
    let input_path = Path::new("tests/data/simple.graphml");
    let output_file = NamedTempFile::new()?;
    export_to_gml(input_path, output_file.path());
    
    let expected_path = Path::new("tests/data/simple.gml");
    assert_output_matches_fixture(output_file.path(), expected_path, "gml")
}
