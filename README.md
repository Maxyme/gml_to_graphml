## GML - Graphml file converter

### How to build:
    # build by statically linking all libraries (including libc)
    `cargo build --target=x86_64-unknown-linux-musl --release`
### How to use:
    ## Convert gml to graphml:
        `graphconverter input_path.gml output_path.graphml`
    ## Convert graphml to gml:
        `graphconverter input_path.graphml output_path.gml`