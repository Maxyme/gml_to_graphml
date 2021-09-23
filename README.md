## GML - Graphml file converter

### How to build:
Normal build, where the host must have glibc=2.33 in order to run:  
`cargo build --release`  

Statically linked build including libc:  
`cargo build --target=x86_64-unknown-linux-musl --release`  

### How to use:
Convert gml to graphml:
- `./graphconverter -i input_path.gml -o output_path.graphml`  

Convert graphml to gml:
- `./graphconverter -i input_path.graphml -o output_path.gml`  
