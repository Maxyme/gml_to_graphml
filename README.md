## GML - Graphml file converter

### Idea:
Goal of this project is to convert files to and from GML/GraphML file formats. 
Uses a bufreader to read files line by line and build objects on disk to prevent OOM for big files.

### Development:
Normal build (the host must have libc):  
`cargo build --release`  

Statically linked build (including libc):  
`cargo build --target=x86_64-unknown-linux-musl --release`  

### Usage:
Convert gml file to graphml:
- `./target/release/graphconverter tests/data/test_simple.gml simple.graphml`  

Convert graphml file to gml:
- `./target/release/graphconverter tests/data/simple.graphml simple.gml`  

### Todo:
- Better error messages when files not found.
- Add generator to create large file (larger than allowed memory) to prove bufreading abilities.
- Add unit-test to check that input/output gml/graphml are the same (use petgraph to check).
- use NamedTempFile for tempfile naming instead of UUID.
- remove quick-xml