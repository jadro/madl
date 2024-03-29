extern crate chrono;
extern crate rev_lines;

use std::env;
use std::fmt;
use std::error::Error;
use std::{fs, path};
use std::fs::File;
use std::io::prelude::*;
use std::io;
use std::sync::mpsc;
use structopt::StructOpt;
use std::path::PathBuf;
use chrono::{DateTime, Local};
use std::collections::{HashSet, HashMap, BTreeMap};
use std::io::ErrorKind;
use hotwatch::{Hotwatch, Event};
use rev_lines::RevLines;
use serde::{Serialize, Deserialize};


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
}

///State of measurement from last line in log
pub enum Laststate {
    IN(Vec<String>),
    OUT(Vec<String>),
    EMPTY,
}

/// Temporary definition file created with flag -d
/// Data from file written to lag after start of measurement, flag -s
pub struct DefFile {
    path: path::PathBuf,
}

impl DefFile {
    pub fn new(config: &Config) -> DefFile {
        let mut dir = env::temp_dir();
        dir.push(&config.teststand_dir);
        let mut dir: path::PathBuf = match fs::create_dir(&dir) {
            Ok(_) => dir,
            Err(ref error) if error.kind() == ErrorKind::AlreadyExists => dir,
            Err(error) => {
                panic!(
                    "There was a problem opening the file: {:?}",
                    error
                )
            },
        };
        dir.push(&config.temp_file);
        DefFile{path: dir}
    }

    /// Remove tempfile after data written to log file.
    pub fn remove_temp_file(&self) -> Result<(), Box<dyn Error>> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
            Ok(())
        } else {
            //eprintln!("Temp file not exist! {:?}", self.path);
            Ok(())
        }
    }

    ///Write output data to temp file
    pub fn write_temp_output(&self, output: &HashMap<&str, String>) -> Result<(), Box<dyn Error>> {
        let f = fs::OpenOptions::new().write(true).create(true).open(&self.path)?;
        serde_yaml::to_writer(f, output)?;
        Ok(())
    }

    // if temp file exist modify output from him if not return w/o change
    pub fn read_temp_output<'a>(&self, mut output: HashMap<&'a str, String>) -> std::io::Result<HashMap<&'a str, String>> {
        //println!("{:?}", self.path);
        if self.path.exists() {
            let file = File::open(&self.path)?;
            let reader = io::BufReader::new(file);
            let value: HashMap<String, String>  = match serde_yaml::from_reader(reader) {
                Ok(val) => val,
                Err(_) => return Ok(output),
            };
            for (i, val) in output.iter_mut() {
                *val = value.get::<str>(&i).unwrap().to_string();
            }
        }
        Ok(output)
    }
}

/// Create config file structure if was not defined before
pub fn create_config_files(config: &Config) -> () {
    let dir = config.settings_dir
            .join(&config.teststand_dir)
            .join(&config.config_dir);
    let log_dir = config.get_log_dir_path();
    let flag_dir = config.settings_dir
            .join(&config.teststand_dir)
            .join(&config.flag_dir);

    for d in vec!(&dir, &log_dir, &flag_dir) {
        if !d.exists() {
            match fs::create_dir_all(&d) {
                Ok(_) => d,
                Err(ref error) if error.kind() == ErrorKind::AlreadyExists => d,
                Err(error) => {
                    panic!(
                        "There was a problem create dir structure for config files: {:?}",
                        error
                    )
                },
            };
        };

        TestInfo::create_empty(dir.join(&config.operator_list_cfg)).unwrap();
        TestInfo::create_empty(dir.join(&config.test_bench_id_cfg)).unwrap();
        TestCategory::create_empty(dir.join(&config.test_category_cfg)).unwrap();
        TestInfo::create_empty(dir.join(&config.test_request_type_cfg)).unwrap();
        TestInfo::create_empty(dir.join(&config.test_stop_reason_list_cfg)).unwrap();
        TestLossClass::create_empty(dir.join(&config.timeloss_classification_cfg)).unwrap();
        TestInfo::create_empty(dir.join(&config.user_data_cfg)).unwrap();
        TestInfo::create_empty(dir.join(&config.user_preference_cfg)).unwrap();
    };

    ()
}

/// Check state from last line in log file. (If measurement started or etc.)
pub fn check_state(output: &HashMap<&str, String>) -> Laststate {
    let last_line = match output.get(&"last_line") {
        Some(text) => text,
        None => panic!("Not defined last line in Output HashMap!"),
    };
    let state_vec: Vec<String> = last_line.trim().split("::").map(String::from).collect();
    //println!("Check_state: {:?}", state_vec);
    match state_vec[0].as_ref() {
        "IN" => return Laststate::IN(state_vec[2..].to_vec()),
        "OUT" => return Laststate::OUT(state_vec[2..].to_vec()),
        _ => return Laststate::EMPTY,
    }
}

/// Read file to string and return string
pub fn read_text_file(path: &PathBuf) -> Result<String, Box<dyn Error>> {
    //println!("Reading config: {:?}", path);
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(e) => panic!("Cant open file path: {:?}\nError: {:?}", path, e),
    };
    let mut contents = String::new();
    f.read_to_string(&mut contents)?;
    Ok(contents)
}

/// Parse string to Vec of Vec of String
pub fn parse_config(raw_string: &str) -> Vec<Vec<String>> {
    raw_string.lines()
        .map(|s| s.trim().split(',').map(String::from).collect::<Vec<_>>())
        .collect::<Vec<_>>()
}

/// Read last test setting from log file
pub fn get_log_data(fpath: path::PathBuf, mut output: HashMap<&str, String>) -> Result<HashMap<&str, String>, Box<dyn Error>> {
    //println!("file path for output: {:?}", fpath);
    let mut keys: HashSet<&str> = output.keys().map(|e| *e).collect();
    if !fpath.exists() {
        return Ok(output);
    }
    let contents = read_text_file(&fpath)?;

    for (l, line) in contents.lines().rev().enumerate() {
        //println!("{}", &line);
        if l == 0 {
            let _val = output.insert(&"last_line", line.to_string());
        }

        let linevec: Vec<&str> = line.trim().split("::").collect();

        match keys.take(linevec[0]) {
            Some(key) => output.insert(key, linevec[1].to_string()),
            None => continue,
        };

        if keys.is_empty() {
            break
        }
    }
    Ok(output)
}

/// Get last modified log path
fn last_modified_log(config: &Config, current_dir: &path::PathBuf) -> Result<path::PathBuf, Box<dyn Error>> {
    //println!("Entries modified in the last 24 hours in {:?}:", current_dir);
    let local: DateTime<Local> = Local::now();
    let filename = config.get_log_file_path(local)?;
    let mut out: path::PathBuf = filename;
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

/// Initialize output with data from log file
pub fn update_output<'a>(config: &Config, output: &HashMap<&'a str, String>) -> Result<HashMap<&'a str, String>, Box<dyn Error>> {
    let dirpath = config.get_log_dir_path();
    match last_modified_log(config, &dirpath) {
        Ok(file_path) => {
            let output = get_log_data(file_path, output.to_owned())?;
            return Ok(output);
        },
        Err(e) => {
            eprintln!("Empty dir or no file: {:?}\nError {:?}", &dirpath, e);
            return Ok(output.to_owned());
        },
    }
}

/// Format test definition
fn format_output(output: &HashMap<&str, String>) -> String {

    let text = format!("\
MADL_Version::2.5\r\n\
InterlockStatus::Enabled\r\n\
TR_Number::{}\r\n\
Specimen ID::{}\r\n\
Test Request type::{}\r\n\
Testing_Category::{}\r\n\
Technician::{}\r\n\
Available Time::{}\r\n",
    output[&"TR_Number"], output[&"Specimen ID"], output[&"Test Request type"],
    output[&"Testing_Category"], output[&"Technician"], output[&"Available Time"]);

    text
}

/// One line config data
struct TestInfo {
    pub values: Vec<String>,
}

impl TestInfo {
    pub fn new(path: &path::PathBuf) ->  Result<TestInfo, Box<dyn Error>> {
        let config_str = read_text_file(path)?;
        let data = parse_config(&config_str);
        let out: Vec<String> = data[0].to_owned();

        Ok(TestInfo{values: out})
    }

    pub fn create_empty(path: PathBuf) -> Result<(), Box<dyn Error>> {
        if !path.exists() {
            let mut f = fs::OpenOptions::new().write(true).create(true).open(path)?;
            f.write_all(b"text, text, text")?;
        };
        Ok(())
    }

    pub fn choose_value(&self) -> Result<String, Box<dyn Error>> {
        loop {
            let mut str_input = String::new();
            println!("{}", self);
            print!(">>");
            io::stdout().flush()?;
            io::stdin().read_line(&mut str_input)
                    .expect("Failed to read line!");
            let lenght: usize = self.values.len();
            match str_input.trim().parse::<usize>() {
                Ok(num) => {
                    if num <= lenght {
                        return Ok(self.values[num].to_owned());
                    } else {
                        println!("Inserted wrong number: {}, please insert again!\n", str_input);
                        continue;
                    }
                },
                Err(e) => {
                    println!("Inserted wrong value: {}, please insert again!\n{:?}\n", str_input, e);
                    continue;
                },
            };
        }
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

/// One line config file with associated number with '*' operator
struct TestCategory {
    values: BTreeMap<String, String>,
}

impl TestCategory {
    pub fn new(path: &path::PathBuf) ->  Result<TestCategory, Box<dyn Error>> {
        let config_str = read_text_file(&path)?;
        let data = parse_config(&config_str);
        let mut out: BTreeMap<String, String> = BTreeMap::new();

        for value in data[0].to_owned().into_iter() {
            let word: Vec<&str> = value.trim().split("*").collect();
            let key: String = word[0].to_string();
            let element: String = word[1].to_string();
            out.insert(key, element);
        }

        Ok(TestCategory{values: out})
    }

    pub fn create_empty(path: PathBuf) -> Result<(), Box<dyn Error>> {
        if !path.exists() {
            let mut f = fs::OpenOptions::new().write(true).create(true).open(path)?;
            f.write_all(b"performance*8, endurance*16, fatiuque*24")?;
        };
        Ok(())
    }

    pub fn choose_value(&self) -> Result<(&String, &String), Box<dyn Error>> {

        loop {
            let mut str_input = String::new();
            println!("{}", self);
            print!(">>");
            io::stdout().flush()?;
            io::stdin().read_line(&mut str_input)
                    .expect("Failed to read input.");
            let lenght: usize = self.values.len();
            match str_input.trim().parse::<usize>() {
                Ok(num) => {
                    if num <= lenght {
                        let (key, val) = self.values.iter().nth(num).unwrap();
                        return Ok((key, val));
                    } else {
                        println!("Inserted wrong number: {}, please insert again!\n", str_input);
                        continue;
                    }
                },
                Err(e) => {
                    println!("Inserted wrong value: {}, please insert again!\n{:?}\n", str_input, e);
                    continue;
                },
            };
        }
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

/// Multi line config file with dependent values
struct TestLossClass {
    values: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

impl TestLossClass {
    pub fn new(path: &path::PathBuf) ->  Result<TestLossClass, Box<dyn Error>> {
        let config_str = read_text_file(&path)?;
        let data = parse_config(&config_str);
        let mut level2: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();

        for line in data.to_owned().into_iter() {
            let level1 = level2.entry(line[0].clone()).or_insert(BTreeMap::new());
            let level0 = level1.entry(line[1].clone()).or_insert(Vec::new());
            level0.push(line[2].clone());
        }

        Ok(TestLossClass{values: level2})
    }

    pub fn create_empty(path: PathBuf) -> Result<(), Box<dyn Error>> {
        if !path.exists() {
            let mut f = fs::OpenOptions::new().write(true).create(true).open(path)?;
            f.write_all(
b"Planned DownTime,Maintenance of any utilities,Pump Inspection\r\n\
Unplanned DownTime,Breakdown of utilities,Air Cool Fail\r\n\
Idle Time,No test sample,Sample Shortage\r\n")?;
        };
        Ok(())
    }

    pub fn display_enumer(&self, values: &Vec<&String>) -> () {
        for (i, v) in values.into_iter().enumerate() {
            println!("{}. {}", i, v)
        }
        ()
    }

    fn read_input(&self, values: &Vec<&String>) -> Result<String, Box<dyn Error>> {

        loop {
            let mut str_input = String::new();
            self.display_enumer(values);
            print!(">>");
            io::stdout().flush()?;
            io::stdin().read_line(&mut str_input)
                    .expect("Failed to read Input");
            let lenght: usize = values.len();
            match str_input.trim().parse::<usize>() {
                Ok(num) => {
                    if num <= lenght {
                        return Ok(values[num].to_owned());
                    } else {
                        println!("Inserted wrong number: {}, please insert again!\n", str_input);
                        continue;
                    }
                },
                Err(e) => {
                    println!("Inserted wrong value: {}, please insert again!\n{:?}\n", str_input, e);
                    continue;
                },
            };
        }
    }

    pub fn choose_value(&self) -> Result<Vec<String>, Box<dyn Error>> {
        println!("\nChoose classification:");
        let firstlevel: Vec<&String> = self.values.keys().collect();
        let first = self.read_input(&firstlevel)?;

        println!("\nChoose sub-classification:");
        let secondlevelval = self.values.get(&first).unwrap();
        let secondlevel: Vec<&String> = secondlevelval.keys().collect();
        let second = self.read_input(&secondlevel)?;

        println!("\nChoose sub-classification:");
        let thirdlevel = secondlevelval.get(&second).unwrap();
        let thirdlevel: Vec<&String> = thirdlevel.iter().collect();
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

#[derive(Debug, Serialize, Deserialize)]
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
    pub temp_file: path::PathBuf,
    pub tc_root_folder: path::PathBuf,
    pub tc_log_folder: path::PathBuf,
}

impl Config {

    pub fn new(stand_nm: u8) -> Result<Config, Box<dyn Error>> {
        let filename = PathBuf::from("madl.cfg");
        if filename.exists() {
            return Ok(Config::read_config(filename)?)
        } else {
            let config = Config {
                settings_dir: PathBuf::from("C:\\Utilization Tool"),
                teststand_dir: PathBuf::from(format!("Teststand{}", stand_nm)),
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
                temp_file: PathBuf::from("madl_temporary_file.txt"),
                tc_root_folder: PathBuf::from("c:\\TCRoot"),
                tc_log_folder: PathBuf::from(format!("station{}\\logs", stand_nm)),
            };
            let f = fs::OpenOptions::new().write(true).create(true).open(&filename)?;
            serde_yaml::to_writer(f, &config)?;
            return Ok(config);
        }
    }

    fn read_config(path: PathBuf) -> Result<Config, Box<dyn Error>> {
        let file = File::open(&path)?;
        let reader = io::BufReader::new(file);
        let config: Config = match serde_yaml::from_reader(reader) {
            Ok(val) => val,
            Err(_) => panic!("Cant read config file."),
        };
        Ok(config)
    }


    /// Return path to configuration file
    pub fn get_config_file_path(&self, conf_file: &path::PathBuf) -> path::PathBuf {
        self.settings_dir
            .join(&self.teststand_dir)
            .join(&self.config_dir)
            .join(conf_file)
    }

    /// Return path to log file
    pub fn get_log_dir_path(&self) -> path::PathBuf {
        self.settings_dir
            .join(&self.teststand_dir)
            .join(&self.log_dir)
    }

    /// Get log file path with date filename
    pub fn get_log_file_path(&self, date: DateTime<Local>) -> Result<path::PathBuf, Box<dyn Error>> {
        let dir_path = self.get_log_dir_path();
        let config_path = self.get_config_file_path(&self.test_bench_id_cfg);
        let test_bench_id_config_str = read_text_file(&config_path)?;
        let test_bench_id = &parse_config(&test_bench_id_config_str)[0][0];

        let date_string = date.format("%d%m%y").to_string();

        let filename = format!("{}_{}.txt", test_bench_id, date_string);
        let log_file_path = dir_path.join(filename);
        //println!("Log file path: {:?}", log_file_path);
        Ok(log_file_path)
    }

    pub fn get_tc_log_folder_path(&self) -> path::PathBuf {
        self.tc_root_folder.join(&self.tc_log_folder)
    }
}

/// Confirm inserted data for request definition.
fn confirm_output_info(output: &mut HashMap<&str, String>) -> Result<String, Box<dyn Error>> {
    println!("TR number: {},", output.entry(&"TR_Number").or_insert("".to_string()));
    println!("Specimen ID: {},", output.entry(&"Specimen ID").or_insert("".to_string()));
    println!("Request type: {},", output.entry(&"Test Request type").or_insert("".to_string()));
    println!("Test category: {}", output.entry(&"Testing_Category").or_insert("".to_string()));
    println!("Operator: {}", output.entry(&"Technician").or_insert("".to_string()));
    loop {
        print!("\nConfirm data Yes/No >>");
        let mut str_input = String::new();
        io::stdout().flush()?;
        io::stdin().read_line(&mut str_input)
            .expect("Failed to read.");
        match str_input.trim().to_lowercase().as_ref() {
            "y" | "yes" => return Ok(str_input),
            "n" | "no" => return Ok(str_input),
            _ => {
                println!("Inserted wrong value, please insert again!");
                continue;
            },
        };
    }
}

/// Append string to a file
fn append_file(path: PathBuf, text: String) -> () {
    let display = path.display();
    let mut file = match fs::OpenOptions::new().append(true).create(true).open(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why.description()),
        Ok(file) => file,
    };

    match file.write_all(text.as_bytes()) {
        Err(why) => panic!("couldn't write to {}: {}", display, why.description()),
        Ok(_) => return (),
    }
}

/// Get user input for test definition
pub fn user_inputs<'a>(config: &Config, mut output: HashMap<&'a str, String>) -> Result<HashMap<&'a str, String>, Box<dyn Error>> {

    println!("\nUse previous values?:");
    let answer = confirm_output_info(&mut output)?;
    let answer = answer.trim().to_lowercase().clone();
    match  answer.as_ref() {
        "yes" | "y" => {
            return Ok(output)},
        _ => (),
    }

    loop {

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
        output.insert("Testing_Category", category.to_owned());
        output.insert(&"Available Time", time.to_owned());

        println!("\nChoose operator:");
        let path = config.get_config_file_path(&config.operator_list_cfg);
        let test_operator = TestInfo::new(&path)?;
        output.insert(&"Technician", test_operator.choose_value()?);

        println!("\nCheck values:");
        let answer = confirm_output_info(&mut output)?;
        let answer = answer.trim().to_lowercase().clone();
        match  answer.as_ref() {
            "yes" | "y" => break,
            "no" | "n" => continue,
            _ => continue,
        }
    }
    Ok(output)
}

/// Write test definition to log file
pub fn write_test_definition(config: &Config, output: &HashMap<&str, String>) -> Result<(), Box<dyn Error>> {
    let text = format_output(&output);
    let local: DateTime<Local> = Local::now();
    let filename = config.get_log_file_path(local)?;
    append_file(filename, text);

    Ok(())
}

fn write_log_line(config: &Config, text_vec: Vec<&str>) -> Result<(), Box<dyn Error>> {
    let local: DateTime<Local> = Local::now();
    let date_string = local.format("%d/%m/%Y %H:%M:%S").to_string();
    let mut text_iter = text_vec.iter();
    let mut out_text: String = String::new();

    out_text.push_str(text_iter.next().unwrap());
    out_text.push_str("::");
    out_text.push_str(&date_string);
    for i in text_iter {
        out_text.push_str("::");
        out_text.push_str(i);
    }
    out_text.push_str("\r\n");

    let filename = config.get_log_file_path(local)?;
    append_file(filename, out_text);

    Ok(())
}

// Write TR specification and start time of testing.
pub fn write_test_start(config: &Config) -> Result<(), Box<dyn Error>> {
    write_log_line(config, vec!("IN", "Test Start"))
}

/// Write test continue log line
pub fn write_continue(config: &Config) -> Result<(), Box<dyn Error>> {
    write_log_line(config, vec!("OUT", "Test Stopped", "Select", "Running Continuous"))
}

/// Write test completed log line
pub fn write_test_end(config: &Config, reason: String) -> Result<(), Box<dyn Error>> {
    write_log_line(config, vec!("OUT", "Test Stopped", &reason, "none"))
}

/// Write test completed log line
pub fn write_missing_test_end(config: &Config) -> Result<(), Box<dyn Error>> {
    write_log_line(config, vec!("OUT", "Test Stopped", "Select", "Missing previous end of test"))
}

/// Write test loss start log line
pub fn write_test_loss(config: &Config, data: Vec<String>) -> Result<(), Box<dyn Error>> {
    write_log_line(config, vec!("IN", &data[0], &data[1], &data[2]))
}

/// Write test loss end log line
pub fn write_test_loss_end(config: &Config, data: &Vec<String>) -> Result<(), Box<dyn Error>> {
    write_log_line(config, vec!("OUT", &data[0], &data[1], &data[2]))
}

fn test_end_input(config: &Config) -> Result<String, Box<dyn Error>> {
    println!("\nChoose test end reason:");
    let path = config.get_config_file_path(&config.test_stop_reason_list_cfg);
    let test_end = TestInfo::new(&path)?;
    let out = test_end.choose_value()?;

    Ok(out)
}

pub fn end_of_test(config: &Config, testloss_skip: bool) -> Result<bool, Box<dyn Error>> {
    loop {
        println!("\nEnd of test or continue? (End/Con):");
        print!(">>");
        let mut str_input = String::new();
        io::stdout().flush()?;
        io::stdin().read_line(&mut str_input)
            .expect("Failed to read.");
        let answer = str_input;
        let answer = answer.trim().to_lowercase().clone();
        let _res = match  answer.as_ref() {
            "con" | "c" => {
                write_continue(&config).unwrap();
                return Ok(true)
            },
            "end" | "e" => {
                let end_reson = test_end_input(&config)?;
                write_test_end(&config, end_reson)?;
                if !testloss_skip {
                    let out = testloose_inputs(&config)?;
                    write_test_loss(&config, out)?;
                }
                return Ok(false)
            },
            _ => {
                println!("Inserted wrong value, please insert again!");
                continue;
            },
        };
    }
}

pub fn testloose_inputs(config: &Config) -> Result<Vec<String>, Box<dyn Error>> {

    loop {
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
        match  answer.as_ref() {
            "yes" | "y" => {
                return Ok(testclass);
            },
            "no" | "n" => continue,
            _ => continue,
        }
    }
}

pub enum TcState {
    Start(String),
    End(String),
    Empty,
}

pub fn watch_folder(folder: PathBuf) -> mpsc::Receiver<TcState> {
    let (tx, rx) = mpsc::channel();
    println!("Started watch folder");
    let mut hotwatch = Hotwatch::new().expect("hotwatch failed to initialize!");
    hotwatch.watch(folder, move |event: Event| {
        if let Event::Write(path) = event {
            println!("Log file: {:?} changed!", path.display());
            tx.send(read_tc_log(path)).unwrap();
        }
    }).expect("failed to watch file!");
    rx
}

// Detect words in line "Test_start" and "Test_end" in log file
pub fn read_tc_log(path: PathBuf) -> TcState {
    let file = File::open(path).unwrap();
    let rev_lines = RevLines::new(io::BufReader::new(file)).unwrap();

    for line in rev_lines {
        if line.contains("Test_start") {
            return TcState::Start(line);
        } else if line.contains("Test_end") {
            return TcState::End(line);
        } else {
            continue;
        }
    }
    TcState::Empty
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let config = Config {
            settings_dir: PathBuf::from("C:\\Utilization Tool"),
            teststand_dir: PathBuf::from(format!("Teststand{}", 1)),
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
            temp_file: PathBuf::from("madl_temporary_file.txt"),
            tc_root_folder: PathBuf::from("c:\\TCRoot"),
            tc_log_folder: PathBuf::from(format!("station{}\\logs", 1)),
        };
        assert_eq!(config.get_config_file_path(&config.operator_list_cfg), PathBuf::from("C:\\Utilization Tool\\Teststand1\\Utilization Config\\Operator List.cfg"));
    }

}