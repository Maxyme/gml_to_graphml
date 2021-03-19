// Convert a gml file created by igraph to a format that networkx understands
// This means removing lines where there is an empty string or a NaN
// And unfolding dicts and lists
// Note this method isn't very good because we can't trust igraph to export to gml correctly (it ignores underscores in keys?? ie label_test 2 -> labeltest 2)

use std::fs::File;
use std::io::{BufWriter, BufReader, Write, BufRead};
use itertools::Itertools;
use regex::Regex;
use serde_json::Value;

fn add_value_with_increment(return_values: &mut Vec<String>, key: &String, item: &Value) {
    // add the values with an increment
    let increment = " ".repeat(6);
    if item.is_f64() {
        return_values.push(format!("{}{} {:?}", increment, key, item.as_f64().expect("")));
    } else if item.is_i64() {
        return_values.push(format!("{}{} {:?}", increment, key, item.as_i64().expect("")));
    } else if item.is_string() {
        return_values.push(format!("{}{} {:?}", increment, key, item.as_str().expect("")));
    } else {
        panic!("WTF");
    }
}

fn get_lines_from_dict(value: &str, dict_key: &str) -> Vec<String> {
    // Parse the value into a dict or list from json
    let increment = " ".repeat(4);
    let mut return_values: Vec<String> = Vec::new();
    let new_value = value.replace("\"{", "{").replace("}\"", "}");
    let map: Value = serde_json::from_str(new_value.as_str()).expect("Issue parsing into json");
    return_values.push(format!("{}{} [",increment, dict_key));

    // todo: there should only be one value?
    for (key, value) in map.as_object().expect("Value isn't valid a dict object") {
        match value {
            value if value.is_array() => {
                // add key [ into the lines
                for item in value.as_array().expect("").iter() {
                    // add value and incrementation (2 spaces)
                    add_value_with_increment(&mut return_values, key, item)
                }

            },
            _ => {
                // value is an object
                add_value_with_increment(&mut return_values, key, value)
            }
        }
    }
    // add closing bracket (]) into the lines
    return_values.push(format!("{}]", increment));
    return_values
}


pub fn gml_to_gml(input_gml: &File, output_file: &mut File) {

    let re_is_dict = Regex::new(r#"^"\{(.+)}"$"#).unwrap();

    let mut buf_writer = BufWriter::new(output_file);
    let buf_reader = BufReader::new(input_gml);

    for line in buf_reader.lines() {
        let line = line.expect("Unable to read line");
        let values = line
            .trim()
            .splitn(2, char::is_whitespace)
            //.map(|x| x.replace("\"", ""))
            .collect_vec()
            ;//.expect("Error parsing data");

        if values.len() == 2 {
            if values[1].is_empty() || values[1] == "NaN" || values[1] == "\"\"" {
                // Skip empty values or NaNs
                continue;
            }

            // check if the value is a dict or a list, which means it starts with "{ and ends with }"
            if re_is_dict.is_match(values[1]) {
                println!("is match {}", values[1]);
                let lines = get_lines_from_dict(values[1], values[0]);
                for line in lines {
                    buf_writer.write(line.as_bytes()).expect("Unable to write data");
                    buf_writer.write("\r\n".as_bytes()).expect("");
                }
                continue;
            }
        }


        buf_writer.write(line.as_bytes()).expect("Unable to write data");
        buf_writer.write("\r\n".as_bytes()).expect("");
    }
}