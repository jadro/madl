extern crate chrono;

use std::fmt;
use std::error::Error;
use std::{fs, path};
use std::fs::File;
use std::io::prelude::*;
use std::io;
use structopt::StructOpt;
use std::path::PathBuf;
use chrono::{DateTime, Local};
use std::collections::{HashSet, HashMap, BTreeMap};
use std::iter::FromIterator;

/// Read config file
pub fn read_config(path: &PathBuf) -> Result<String, Box<dyn Error>> {
    println!("Reading config: {:?}", path);
    let mut f = File::open(path).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents)?;
    Ok(contents)
}

/// Parse string to Vec of Vec of String
pub fn parse_config(raw_string: &str) -> Vec<Vec<String>> {
    let main_vec = raw_string.lines()
                     .map(|s| s.trim().split(',').map(String::from).collect::<Vec<_>>())
                     .collect::<Vec<_>>();
    main_vec
}

struct TestInfo {
    pub cfg_file: path::PathBuf,
    pub values: Vec<String>,
}

impl TestInfo {
    pub fn new(path: &path::PathBuf) ->  Result<TestInfo, Box<dyn Error>> {
        let config_str = read_config(path)?;
        let data = &parse_config(&config_str);
        let out: Vec<String> = data[0].to_owned();

        Ok(TestInfo{cfg_file: path.to_owned(), values: out})
    }

    pub fn choose_value(&self) -> Result<String, Box<dyn Error>> {
        let mut str_input = String::new();
        println!("{}", self);
        print!(">>");
        io::stdout().flush()?;
        io::stdin().read_line(&mut str_input)
                .expect("Failed to read Specimen ID");
        let index: usize = str_input.trim().parse()?;
        Ok(self.values[index].to_owned())
    }
}

impl fmt::Display for TestInfo {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = &self.values;
        for (i, v) in val.into_iter().enumerate() {
            write!(f, "{}. {}\n", i, v)?;
        }
        write!(f, "")
    }
}

struct TestCategory {
    cfg_file: path::PathBuf,
    values: BTreeMap<String, String>,
}

impl TestCategory {
    pub fn new(path: &path::PathBuf) ->  Result<TestCategory, Box<dyn Error>> {
        let config_str = read_config(&path)?;
        let data = &parse_config(&config_str);
        let mut out: BTreeMap<String, String> = BTreeMap::new();

        for value in data[0].to_owned().into_iter() {
            let word: Vec<&str> = value.trim().split("*").collect();
            let key: String = word[0].to_string();
            let element: String = word[1].to_string();
            out.insert(key, element);
        }

        Ok(TestCategory{cfg_file: path.to_owned(), values: out})
    }

    pub fn choose_value(&self) -> Result<(&String, &String), Box<dyn Error>> {
        let mut str_input = String::new();
        println!("{}", self);
        print!(">>");
        io::stdout().flush()?;
        io::stdin().read_line(&mut str_input)
                .expect("Failed to read Specimen ID");
        let index: usize = str_input.trim().parse()?;
        let (key, val) = self.values.iter().nth(index).unwrap();
        Ok((key, val))
    }
}

impl fmt::Display for TestCategory {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = &self.values;
        for (i, (v, time)) in val.into_iter().enumerate() {
            write!(f, "{}. {} in duration {}\n", i, v, time)?;
        }
        write!(f, "")
    }
}

struct TestLossClass {
    cfg_file: path::PathBuf,
    values: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

impl TestLossClass {
    pub fn new(path: &path::PathBuf) ->  Result<TestLossClass, Box<dyn Error>> {
        let config_str = read_config(&path)?;
        let data = &parse_config(&config_str);
        let mut level2: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();

        for line in data.to_owned().into_iter() {
            level2.insert(line[0].clone(), BTreeMap::new());
            let level1 = level2.get_mut(&line[0]).unwrap();
            level1.insert(line[1].clone(), Vec::new());
            let level0 = level1.get_mut(&line[1]).unwrap();
            level0.push(line[2].clone());
        }

        Ok(TestLossClass{cfg_file: path.to_owned(), values: level2})
    }

    pub fn display_enumer(&self, values: &Vec<String>) -> () {
        for (i, v) in values.into_iter().enumerate() {
            println!("{}. {}", i, v)
        }
        ()
    }

    fn read_input(&self, values: &Vec<String>) -> Result<String, Box<dyn Error>> {
        let mut str_input = String::new();
        self.display_enumer(&values);
        print!(">>");
        io::stdout().flush()?;
        io::stdin().read_line(&mut str_input)
                .expect("Failed to read Input");
        let index: usize = str_input.trim().parse()?;
        Ok(values[index].clone())
    }

    pub fn choose_value(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut out: Vec<String> = Vec::new();
        println!("Choose classification:");
        let firstlevel: Vec<String> = self.values.keys().cloned().collect();
        let first = self.read_input(&firstlevel)?;

        println!("Choose sub-classification:");
        let secondlevelval = self.values.get(&first).unwrap();
        let secondlevel: Vec<String> = secondlevelval.keys().cloned().collect();
        let second = self.read_input(&secondlevel)?;

        println!("Choose sub-classification:");
        let thirdlevel = secondlevelval.get(&second).unwrap();
        let third = self.read_input(&thirdlevel)?;

        out.push(first);
        out.push(second);
        out.push(third);

        Ok(out)
    }
}

impl fmt::Display for TestLossClass {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = &self.values;
        for (i, (k, v)) in val.into_iter().enumerate() {
            write!(f, "{}. {}\n", i, k)?;
            for (j, (l, h)) in v.into_iter().enumerate() {
                write!(f, "\t{}. {}\n", j, l)?;
                for (e, u) in h.into_iter().enumerate() {
                    write!(f, "\t\t{}. {}\n", e, u)?;
                }
            }
        }
        write!(f, "")
    }
}

pub struct Config {
    pub settings_dir: path::PathBuf,
    pub teststand_dir: path::PathBuf,
    pub flag_dir: path::PathBuf,
    pub log_dir: path::PathBuf,
    pub config_dir: path::PathBuf,
    pub operator_list_cfg: path::PathBuf,
    pub test_category_cfg: path::PathBuf,
    pub test_request_type_cfg: path::PathBuf,
    pub test_bench_id_cfg: path::PathBuf,
    pub test_stop_reason_list_cfg: path::PathBuf,
    pub timeloss_classification_cfg: path::PathBuf,
    pub user_data_cfg: path::PathBuf,
    pub user_preference_cfg: path::PathBuf,
}

impl Config {
    /// Return path to operator list configuration file
    pub fn get_config_file_path(&self, conf_file: &path::PathBuf) -> path::PathBuf {
        self.settings_dir
            .join(&self.teststand_dir)
            .join(&self.config_dir)
            .join(conf_file)
    }

    fn get_log_dir_path(&self) -> path::PathBuf {
        self.settings_dir
            .join(&self.teststand_dir)
            .join(&self.log_dir)
    }

    // Get file path with date filename
    // let local: DateTime<Local> = Local::now();
    pub fn get_log_file_path(&self, date: DateTime<Local>) -> Result<path::PathBuf, Box<dyn Error>> {
        let dir_path = self.get_log_dir_path();
        let test_bench_id_config_str = read_config(&self.test_bench_id_cfg)?;
        let test_bench_id = &parse_config(&test_bench_id_config_str)[0][0];

        let date_string = date.format("%d%m%y").to_string();

        let filename = format!("{}_{}.txt", test_bench_id, date_string);
        let log_file_path = dir_path.join(filename);
        println!("Log file path: {}", date_string);
        Ok(log_file_path)
    }
}

fn pub get_log_data(fpath: path::PathBuf, mut output: HashMap<&str, String>) -> Result<HashMap<&str, String>, Box<dyn Error>> {
    let mut keys: HashSet<&str> = output.keys().map(|e| *e).collect();
    println!("Reading config: {:?}", fpath);
    let mut f = File::open(fpath).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents)?;

    for line in contents.lines().rev() {
        let linevec: Vec<&str> = line.trim().split("::").collect();
        let key: &str = keys.take(linevec[0]).unwrap();
        output.insert(key, linevec[1].to_string());
        if keys.is_empty() {
            break
        }
    }
    Ok(output)
}

fn last_mod_log(current_dir: path::PathBuf) -> Result<path::PathBuf, Box<dyn Error>> {
    println!(
        "Entries modified in the last 24 hours in {:?}:",
        current_dir
    );
    let mut out: path::PathBuf = path::PathBuf::new();

    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

        let metadata = fs::metadata(&path)?;
        let last_modified = metadata.modified()?.elapsed()?.as_secs();

        if last_modified < 24 * 3600 && metadata.is_file() {
            println!(
                "Last modified: {:?} seconds, filename: {:?}",
                last_modified,
                path.file_name().ok_or("No filename")?
            );
            out.push(path.to_path_buf());
            break
        }
    }

    Ok(out)
}

pub struct Output {
    pub tr_number: String,
    pub specimen_id: String,
    pub request_type: String,
    pub test_category: String,
    pub operator: String,
    pub InterlockStatus: bool,
    pub avalaible_time: u8,
    pub test_state: String,
    pub stop_reason: String,
    pub comments: String,
    pub classification: String,
    pub sub_classification: String,
    pub sub_category: String,
}

fn confirm_output_info(output: &mut HashMap<&str, String>) -> Result<String, Box<dyn Error>> {
    let mut str_input = String::new();
    println!("TR number: {},", output.entry(&"tr_number").or_insert("".to_string()));
    println!("Specimen ID: {},", output.entry(&"specimen_id").or_insert("".to_string()));
    println!("Request type: {},", output.entry(&"request_type").or_insert("".to_string()));
    println!("Test category: {}", output.entry(&"test_category").or_insert("".to_string()));
    println!("Operator: {}", output.entry(&"operator").or_insert("".to_string()));
    print!("\nConfirm data Yes/No >>");
    io::stdout().flush()?;
    io::stdin().read_line(&mut str_input)
        .expect("Failed to read.");
    Ok(str_input)
}

pub fn user_inputs(config: Config, mut output: HashMap<&str, String>) -> Result<HashMap<&str, String>, Box<dyn Error>> {

    println!("\nUse previous values?:");
    let answer = confirm_output_info(&mut output)?;
    let answer = answer.trim().to_lowercase().clone();
    println!("Answer: {}", &answer);
    match  answer.as_ref() {
        "yes" | "y" => return Ok(output),
        _ => (),
    }

    loop {
        let mut str_input = String::new();
        println!("Write TR number:");
        print!(">>");
        io::stdout().flush()?;
        io::stdin().read_line(&mut str_input)
            .expect("Failed to read TR number");
        output.insert("tr_number", str_input.trim().to_string());

        let mut str_input = String::new();
        println!("\nWrite Specimen ID:");
        print!(">>");
        io::stdout().flush()?;
        io::stdin().read_line(&mut str_input)
                .expect("Failed to read Specimen ID");
        output.insert("specimen_id", str_input.trim().to_string());

        println!("\nChoose request type:");
        let path = config.get_config_file_path(&config.test_request_type_cfg);
        let test_request = TestInfo::new(&path)?;
        output.insert("request_type", test_request.choose_value()?);

        println!("\nChoose test category:");
        let path = config.get_config_file_path(&config.test_category_cfg);
        let test_category = TestCategory::new(&path)?;
        let (category, time) = test_category.choose_value()?;
        output.insert("test_category", category.to_owned());
        output.insert("avalaible_time", time.to_owned());

        println!("\nChoose operator:");
        let path = config.get_config_file_path(&config.operator_list_cfg);
        let test_operator = TestInfo::new(&path)?;
        output.insert("operator", test_operator.choose_value()?);

        println!("\nCheck values:");
        let answer = confirm_output_info(&mut output)?;
        let answer = answer.trim().to_lowercase().clone();
        println!("Answer: {}", &answer);
        match  answer.as_ref() {
            "yes" | "y" => break,
            "no" | "n" => continue,
            _ => continue,
        }
    }

    Ok(output)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let config = Config {
            settings_dir: PathBuf::from("C:\\Utilization Tool"),
            teststand_dir: PathBuf::from("Teststand1"),
            flag_dir: PathBuf::from("Utilization Flag"),
            log_dir: PathBuf::from("Utilization Log"),
            config_dir: PathBuf::from("Utilization Config"),
            operator_list_cfg: PathBuf::from("Operator List.cfg"),
            test_category_cfg: PathBuf::from("Test category.cfg"),
            test_request_type_cfg: PathBuf::from("Test Request type.cfg"),
            test_bench_id_cfg: PathBuf::from("TestBench ID.cfg"),
            test_stop_reason_list_cfg: PathBuf::from("TestStop Reason List.cfg"),
            timeloss_classification_cfg: PathBuf::from("Timeloss Classification.cfg"),
            user_data_cfg: PathBuf::from("User data.cfg"),
            user_preference_cfg: PathBuf::from("User preference.cfg"),
        };
        assert_eq!(config.get_config_file_path(&config.operator_list_cfg), PathBuf::from("C:\\Utilization Tool\\Teststand1\\Utilization Config\\Operator List.cfg"));
    }

}