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


#[derive(StructOpt)]
#[structopt(name = "Madl",
about = "Measure laboratory ")]
pub struct Cli {
    /// Number of test stand
    #[structopt(
        //short = "c",
        //long = "cell",
        help="Test cell number")]
    pub cell: u8,

    /// Define test setup
    #[structopt(short = "d",
        long = "define",
        help="Define TR parameters")
    ]
    pub define: bool,

    /// Start of test duration measurement
    #[structopt(short = "s",
        long = "start",
        help="Start of test duration measurement")
    ]
    pub start: bool,

    /// End of test duration measurement
    #[structopt(short = "e",
        long = "end",
        help="End of test duration measurement")
    ]
    pub end: bool,
}

pub enum Laststate {
    IN(Vec<String>),
    OUT(Vec<String>),
    EMPTY,
}

pub fn check_state(output: &HashMap<&str, String>) -> Result<Laststate, Box<dyn Error>> {
    let last_line = match output.get(&"last_line") {
        Some(text) => text,
        None => panic!("Not defined last line in Output HashMap!"),
    };
    let state_vec: Vec<String> = last_line.trim().split("::").map(String::from).collect();
    println!("Check_state: {:?}", state_vec);
    match state_vec[0].as_ref() {
        "IN" => return Ok(Laststate::IN(state_vec[2..].to_vec())),
        "OUT" => return Ok(Laststate::OUT(state_vec[2..].to_vec())),
        _ => return Ok(Laststate::EMPTY),
    }
}

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

pub fn get_log_data(fpath: path::PathBuf, mut output: HashMap<&str, String>) -> Result<HashMap<&str, String>, Box<dyn Error>> {
    println!("file path for output: {:?}", fpath);
    let mut keys: HashSet<&str> = output.keys().map(|e| *e).collect();
    let contents = read_config(&fpath)?;

    for (l, line) in contents.lines().rev().enumerate() {
        println!("{}", &line);
        if l == 0 {
            let _val = output.insert(&"last_line", line.to_string());
        }

        let linevec: Vec<&str> = line.trim().split("::").collect();
        //let mut key: &str = "";
        match keys.take(linevec[0]) {
            Some(key) => {let _result: Option<String> = output.insert(key, linevec[1].to_string());},
            None => continue,
        }
        //output.insert(key, linevec[1].to_string());
        if keys.is_empty() {
            break
        }
    }
    Ok(output)
}

fn last_modified_log(current_dir: &path::PathBuf) -> Result<path::PathBuf, Box<dyn Error>> {
    println!(
        "Entries modified in the last 24 hours in {:?}:",
        current_dir
    );
    let mut out: path::PathBuf = path::PathBuf::new();
    let mut timediff: Option<u64> = None;

    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

        let metadata = fs::metadata(&path)?;
        let last_modified = metadata.modified()?.elapsed()?.as_secs();

        match timediff {
            Some(diff) => if {last_modified < diff} && metadata.is_file() {
                out = path.to_path_buf();
                timediff = Some(last_modified);
            },
            None => {
                timediff = Some(last_modified);
                out = path.to_path_buf();
            },
        }
    }

    Ok(out)
}

pub fn init_output<'a>(config: &Config, output: HashMap<&'a str, String>) -> Result<HashMap<&'a str, String>, Box<dyn Error>> {
    let dirpath = config.get_log_dir_path();
    match last_modified_log(&dirpath) {
        Ok(file_path) => {
            let output = get_log_data(file_path, output)?;
            return Ok(output);
        },
        Err(e) => {
            println!("Empty dir or no file: {:?}\nError {:?}", &dirpath, e);
            return Ok(output);
        },
    }
}

fn format_output(output: &HashMap<&str, String>) -> String {

    let text = format!("\
MADL_Version::2.5
InterlockStatus::Enabled
TR_Number::{}
Specimen ID::{}
Test Request type::{}
Testing_Category::{}
Technician::{}
Available Time::{}\n",
    output[&"TR_Number"], output[&"Specimen ID"], output[&"Test Request type"],
    output[&"Testing_Category"], output[&"Technician"], output[&"Available Time"]);

    text
}

// Write TR specification and start time of testing.
pub fn start_of_test(config: &Config, output: &HashMap<&str, String>) -> Result<(), Box<dyn Error>> {

    let mut text = format_output(&output);
    let local: DateTime<Local> = Local::now();
    let date_string = local.format("%d/%m/%Y %H:%M:%S").to_string();
    let text_time = format!("IN::{}::Test Start\n", date_string);
    text.push_str(&text_time);

    let filename = config.get_log_file_path(local)?;
    append_file(filename, text)?;

    Ok(())
}

struct TestInfo {
    pub values: Vec<String>,
}

impl TestInfo {
    pub fn new(path: &path::PathBuf) ->  Result<TestInfo, Box<dyn Error>> {
        let config_str = read_config(path)?;
        let data = &parse_config(&config_str);
        let out: Vec<String> = data[0].to_owned();

        Ok(TestInfo{values: out})
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

        Ok(TestCategory{values: out})
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

        Ok(TestLossClass{values: level2})
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
        println!("\nChoose classification:");
        let firstlevel: Vec<String> = self.values.keys().cloned().collect();
        let first = self.read_input(&firstlevel)?;

        println!("\nChoose sub-classification:");
        let secondlevelval = self.values.get(&first).unwrap();
        let secondlevel: Vec<String> = secondlevelval.keys().cloned().collect();
        let second = self.read_input(&secondlevel)?;

        println!("\nChoose sub-classification:");
        let thirdlevel = secondlevelval.get(&second).unwrap();
        let third = self.read_input(&thirdlevel)?;

        let out = vec!(first, second, third);
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

    pub fn get_log_dir_path(&self) -> path::PathBuf {
        self.settings_dir
            .join(&self.teststand_dir)
            .join(&self.log_dir)
    }

    // Get file path with date filename
    // let local: DateTime<Local> = Local::now();
    pub fn get_log_file_path(&self, date: DateTime<Local>) -> Result<path::PathBuf, Box<dyn Error>> {
        let dir_path = self.get_log_dir_path();
        let config_path = self.get_config_file_path(&self.test_bench_id_cfg);
        let test_bench_id_config_str = read_config(&config_path)?;
        let test_bench_id = &parse_config(&test_bench_id_config_str)[0][0];

        let date_string = date.format("%d%m%y").to_string();

        let filename = format!("{}_{}.txt", test_bench_id, date_string);
        let log_file_path = dir_path.join(filename);
        println!("Log file path: {:?}", log_file_path);
        Ok(log_file_path)
    }
}

fn confirm_output_info(output: &mut HashMap<&str, String>) -> Result<String, Box<dyn Error>> {
    println!("TR number: {},", output.entry(&"TR_Number").or_insert("".to_string()));
    println!("Specimen ID: {},", output.entry(&"Specimen ID").or_insert("".to_string()));
    println!("Request type: {},", output.entry(&"Test Request type").or_insert("".to_string()));
    println!("Test category: {}", output.entry(&"Testing_Category").or_insert("".to_string()));
    println!("Operator: {}", output.entry(&"Technician").or_insert("".to_string()));
    print!("\nConfirm data Yes/No >>");
    let mut str_input = String::new();
    io::stdout().flush()?;
    io::stdin().read_line(&mut str_input)
        .expect("Failed to read.");
    Ok(str_input)
}

fn append_file(path: PathBuf, text: String) -> Result<(), Box<dyn Error>> {
    let display = path.display();
    let mut file = match fs::OpenOptions::new().append(true).create(true).open(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why.description()),
        Ok(file) => file,
    };

    match file.write_all(text.as_bytes()) {
        Err(why) => panic!("couldn't write to {}: {}", display, why.description()),
        Ok(_) => println!("successfully wrote to {}", display),
    }
    Ok(())
}



pub fn user_inputs(config: Config, mut output: HashMap<&str, String>) -> Result<HashMap<&str, String>, Box<dyn Error>> {

    println!("\nUse previous values?:");
    let answer = confirm_output_info(&mut output)?;
    let answer = answer.trim().to_lowercase().clone();
    println!("Answer: {}", &answer);
    match  answer.as_ref() {
        "yes" | "y" => {
            return Ok(output)},
        _ => (),
    }

    loop {
        output.insert("opened_window", "Yes".to_string());

        let mut str_input = String::new();
        println!("Write TR number:");
        print!(">>");
        io::stdout().flush()?;
        io::stdin().read_line(&mut str_input)
            .expect("Failed to read TR number");
        output.insert(&"TR_Number", str_input.trim().to_string());

        let mut str_input = String::new();
        println!("\nWrite Specimen ID:");
        print!(">>");
        io::stdout().flush()?;
        io::stdin().read_line(&mut str_input)
                .expect("Failed to read Specimen ID");
        output.insert(&"Specimen ID", str_input.trim().to_string());

        println!("\nChoose request type:");
        let path = config.get_config_file_path(&config.test_request_type_cfg);
        let test_request = TestInfo::new(&path)?;
        output.insert(&"Test Request type", test_request.choose_value()?);

        println!("\nChoose test category:");
        let path = config.get_config_file_path(&config.test_category_cfg);
        let test_category = TestCategory::new(&path)?;
        let (category, time) = test_category.choose_value()?;
        output.insert("test_category", category.to_owned());
        output.insert(&"Available Time", time.to_owned());

        println!("\nChoose operator:");
        let path = config.get_config_file_path(&config.operator_list_cfg);
        let test_operator = TestInfo::new(&path)?;
        output.insert(&"Technician", test_operator.choose_value()?);

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
    output.insert("opened_window", "No".to_string());
    Ok(output)
}

fn write_continue(config: &Config) -> Result<(), Box<dyn Error>> {
    let local: DateTime<Local> = Local::now();
    let date_string = local.format("%d/%m/%Y %H:%M:%S").to_string();
    let text = format!("OUT::{}::Test Stopped::Select::Running Continuous\n", date_string);

    let filename = config.get_log_file_path(local)?;
    append_file(filename, text)?;
    Ok(())
}

fn write_test_end(config: &Config) -> Result<(), Box<dyn Error>> {
    let local: DateTime<Local> = Local::now();
    let date_string = local.format("%d/%m/%Y %H:%M:%S").to_string();
    let text = format!("OUT::{}::Test Stopped::Test Completed::none\n", date_string);

    let filename = config.get_log_file_path(local)?;
    append_file(filename, text)?;
    Ok(())
}

fn write_test_loss(config: &Config, data: Vec<String>) -> Result<(), Box<dyn Error>> {
    let local: DateTime<Local> = Local::now();
    let date_string = local.format("%d/%m/%Y %H:%M:%S").to_string();
    let text = format!("IN::{}::{}::{}::{}\n", date_string, data[0], data[1], data[2]);

    let filename = config.get_log_file_path(local)?;
    append_file(filename, text)?;
    Ok(())
}

pub fn write_test_loss_end(config: &Config, data: Vec<String>) -> Result<(), Box<dyn Error>> {
    let local: DateTime<Local> = Local::now();
    let date_string = local.format("%d/%m/%Y %H:%M:%S").to_string();
    let text = format!("OUT::{}::{}::{}::{}\n", date_string, data[0], data[1], data[2]);

    let filename = config.get_log_file_path(local)?;
    append_file(filename, text)?;
    Ok(())
}

pub fn end_of_test(config: &Config, output: &mut HashMap<&str, String>) -> Result<(), Box<dyn Error>> {
    println!("\nEnd of test or continue? (End/Con):");
    let mut str_input = String::new();
    io::stdout().flush()?;
    io::stdin().read_line(&mut str_input)
        .expect("Failed to read.");
    let answer = str_input;
    let answer = answer.trim().to_lowercase().clone();
    println!("Answer: {}", &answer);
    let _res = match  answer.as_ref() {
        "con" | "c" => write_continue(&config),
        "end" | "e" => {
            write_test_end(&config)?;
            let out = testloose_inputs(&config, output)?;
            write_test_loss(&config, out)
        },
        _ => Ok(()),
    };
    Ok(())
}

pub fn testloose_inputs(config: &Config, output: &mut HashMap<&str, String>) -> Result<Vec<String>, Box<dyn Error>> {

    loop {
        output.insert("opened_window", "Yes".to_string());
        println!("\nChoose time loss clasification:");
        let path = config.get_config_file_path(&config.timeloss_classification_cfg);
        let test_request = TestLossClass::new(&path)?;
        let testclass = test_request.choose_value()?;

        println!("\nCheck values:");
        println!("Time loss classification: {}", testclass[0]);
        println!("Time loss sub classification: {}", testclass[1]);
        println!("Time loss sub category: {}", testclass[0]);

        let mut str_input = String::new();
        print!("\nConfirm data Yes/No >>");
        io::stdout().flush()?;
        io::stdin().read_line(&mut str_input)
            .expect("Failed to read.");
        let answer = str_input.trim().to_lowercase().clone();
        println!("Answer: {}", &answer);
        match  answer.as_ref() {
            "yes" | "y" => {
                output.insert("opened_window", "No".to_string());
                return Ok(testclass);
            },
            "no" | "n" => continue,
            _ => continue,
        }
    }
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