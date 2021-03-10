extern crate yaml_rust;

use yaml_rust::{YamlLoader, YamlEmitter, Yaml};
use std::env;
use std::io::{BufReader, Read, BufWriter};
use std::fs::File;
use ron::ser::{PrettyConfig, to_string_pretty, to_writer, to_writer_pretty};
use std::string;
use serde::{Serialize, Serializer};
use linked_hash_map::LinkedHashMap;
use crate::YamlSerialize::BadValue;

#[derive(Clone, PartialEq, PartialOrd, Debug, Eq, Ord, Hash,Serialize)]
pub enum YamlSerialize {
    /// Float types are stored as String and parsed on demand.
    /// Note that f64 does NOT implement Eq trait and can NOT be stored in BTreeMap.
    Real(string::String),
    /// YAML int is stored as i64.
    Integer(i64),
    /// YAML scalar.
    String(string::String),
    /// YAML bool, e.g. `true` or `false`.
    Boolean(bool),
    /// YAML array, can be accessed as a `Vec`.
    Array(self::Array),
    /// YAML hash, can be accessed as a `LinkedHashMap`.
    ///
    /// Insertion order will match the order of insertion into the map.
    Node(self::Hash),
    /// Alias, not fully supported yet.
    Alias(usize),
    /// YAML null, e.g. `null` or `~`.
    Null,
    /// Accessing a nonexistent node via the Index trait returns `BadValue`. This
    /// simplifies error handling in the calling code. Invalid type conversion also
    /// returns `BadValue`.
    BadValue,
}
pub type Array = Vec<YamlSerialize>;
pub type Hash = LinkedHashMap<YamlSerialize, YamlSerialize>;

pub fn get_file_content(filename:&String)->Result<String,String>{
    let file = File::open(filename);
    if file.is_err() {
        return Err(format!("Could not open file {}", filename));

    }
    let mut buffer = Vec::new();
    // read the whole file
    let f = BufReader::new(file.unwrap()).read_to_end(&mut buffer);
    if f.is_err() {
        return Err(format!("Failed to read the file {}", filename));

    }
    match std::str::from_utf8(&*buffer) {
        Ok(v) => Ok(v.to_owned()),
        Err(e) => Err(format!("Invalid UTF-8 sequence: {}", e))
    }
}

pub fn load_yml_str(content:&str)->Result<Vec<Yaml>,String>{
    let doc=YamlLoader::load_from_str(content);
    if doc.is_err(){
        return Err(format!("Invalid yml file: {}",doc.unwrap_err()));
    }
    let doc:Vec<Yaml>=doc.unwrap();
    let mut out_str = String::new();{
        let mut emitter = YamlEmitter::new(&mut out_str);
        emitter.dump(doc.get(0).unwrap()).unwrap(); // dump the YAML object to a String
    }
    println!("{:?}", doc);
    Ok(doc)
}

pub fn create_ron_file(filename:&String)->Result<BufWriter<File>,String>{
    let ron_filename=filename.split(".").next();
    if ron_filename.is_none(){
        return Err(format!("Failed to get the ron filename {}",filename));
    }
    let ron_filename=ron_filename.unwrap().to_owned()+".ron";

    let ron_file = File::create(&ron_filename);
    if ron_file.is_err(){
        return Err(format!("Failed to create the ron file {}",ron_filename));
    }
    Ok(BufWriter::new(ron_file.unwrap()))
}

fn main() ->Result<(), String>{
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(format!("Usage is : ./yron <filename>"));
    }
    let filename = args.get(1).unwrap();
    let content=get_file_content(filename)?;
    let doc=load_yml_str(&*content)?;
    let converted=convert(doc);
    let ron_buffer=create_ron_file(filename)?;

    let pretty = PrettyConfig::new()
        .with_separate_tuple_members(true)
        .with_enumerate_arrays(true);
    let r=to_writer_pretty(ron_buffer,&converted , pretty);
    if r.is_err(){
        return Err(format!("Serialization failed for {:?}",&converted));
    }
    Ok(())
}


pub fn convert(yml:Vec<Yaml>)->Vec<YamlSerialize>{
    let mut res:Vec<YamlSerialize>=Vec::with_capacity(yml.capacity());
    for item in yml{
       res.push(match item {
           Yaml::Real(x) => {YamlSerialize::Real(x)}
           Yaml::Integer(x) => {YamlSerialize::Integer(x)}
           Yaml::String(x) => {YamlSerialize::String(x)}
           Yaml::Boolean(x) => {YamlSerialize::Boolean(x)}
           Yaml::Alias(x) => {YamlSerialize::Alias(x)}
           Yaml::Null => {YamlSerialize::Null}
           Yaml::BadValue => {YamlSerialize::BadValue}
           Yaml::Array(x) => {
               YamlSerialize::Array(convert(x))
           }
           // Yaml::Hash(x)=>{YamlSerialize::BadValue}
           Yaml::Hash(x) => {
               let mut map:LinkedHashMap<YamlSerialize,YamlSerialize>=Hash::new();

               for (k,v) in x{
                   map.insert(convert(vec![k]).remove(0),convert(vec![v]).remove(0));
               }
               YamlSerialize::Node(map)
           }
       } )
    }
    res
}

