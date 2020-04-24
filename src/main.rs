use madl::{Config, user_inputs, init_output, Laststate, check_state, write_test_definition,
    DefFile, TcState, read_tc_log,
    end_of_test, write_test_start, write_test_loss_end, testloose_inputs, write_test_loss};
use std::path::PathBuf;
use std::process;
use std::collections::HashMap;
use madl::Cli;
use structopt::StructOpt;
use std::io::prelude::*;
use std::io;
use std::sync::mpsc;
use hotwatch::{Hotwatch, Event};

fn start_test_definition<'a>(config: &Config, output: &'a HashMap<&'a str, String>) -> HashMap<&'a str, String> {
    let output = output.to_owned();
    let deffile = DefFile::new(&config);
    let output =  deffile.read_temp_output(output).unwrap();
    let output = match user_inputs(&config, output) {
        Err(e) => {
            eprintln!("Application error: {}", e);
            process::exit(1);
        },
        Ok(val) => val,
    };
    deffile.write_temp_output(&output).unwrap();
    output
}

fn start_change_timeloss<'a>(config: &Config, output: &'a HashMap<&'a str, String>) -> HashMap<&'a str, String> {
    let output = output.to_owned();
    let mut output = init_output(&config, &output).unwrap();
    let last_state = check_state(&output);

    match last_state {
        Laststate::IN(vec_data) => {
            if vec_data[0].contains("Test Start") {
                end_of_test(&config, &mut output, false).unwrap();
            } else {
                write_test_loss_end(&config, &vec_data).unwrap();
            }
            let out = testloose_inputs(&config, &mut output).unwrap();
            write_test_loss(&config, out).unwrap();
        },
        Laststate::OUT(_) => {
            let out = testloose_inputs(&config, &mut output).unwrap();
            write_test_loss(&config, out).unwrap();
        },
        Laststate::EMPTY => {
            let out = testloose_inputs(&config, &mut output).unwrap();
            write_test_loss(&config, out).unwrap();
        },
    }
    output
}

// Get definition of test
fn test_start_measurement<'a>(config: &Config, output: &'a HashMap<&'a str, String>) -> HashMap<&'a str, String> {
    let output = output.to_owned();
    let deffile = DefFile::new(&config);
    let output = deffile.read_temp_output(output).unwrap();
    let (tx, rx) = mpsc::channel();
    let tcroot_folder = config.get_tc_log_folder_path();
    // watch_folder(tcroot_folder, tx);

    println!("Checking log folder");
    let mut hotwatch = Hotwatch::new().expect("hotwatch failed to initialize!");
    hotwatch.watch(tcroot_folder, move |event: Event| {
        if let Event::Write(path) = event {
            println!("Log file: {:?} changed!", path.display());
            tx.send(read_tc_log(path)).unwrap();
        }
    }).expect("failed to watch file!");

    for received in rx {
        let mut output = init_output(&config, &output).unwrap();
        let last_state = check_state(&output);

        match received {
            TcState::Start(_) => {
                match last_state {
                    Laststate::IN(ref vec_data) => {
                        if vec_data[0].contains("Test Start") {
                            println!("\n!!Last log data are from start of test!!\nNeeded to define end of previous test!\n");
                            end_of_test(&config, &mut output, true).unwrap();
                            write_test_definition(&config, &output).unwrap();
                            write_test_start(&config).unwrap();
                        } else {
                            write_test_loss_end(&config, &vec_data).unwrap();
                            write_test_definition(&config, &output).unwrap();
                            write_test_start(&config).unwrap();
                        }
                    }
                    Laststate::OUT(_) => {
                        write_test_definition(&config, &output).unwrap();
                        write_test_start(&config).unwrap();
                    },
                    Laststate::EMPTY => {
                        write_test_start(&config).unwrap();
                    },
                };
                deffile.remove_temp_file().unwrap();
            },
            TcState::End(_) => {
                match last_state {
                    Laststate::IN(ref vec_data) => {
                        if vec_data[0].contains("Test Start") {
                            if end_of_test(&config, &mut output, false).unwrap() {
                                println!("Continue in testing");
                                continue
                            };
                        } else {
                            write_test_loss_end(&config, vec_data).unwrap();
                        }
                    }
                    Laststate::OUT(ref vec_data) => println!("\nLast activity is already stoped: {}->{}->{}\n", vec_data[0], vec_data[1], vec_data[2]),
                    Laststate::EMPTY => panic!("\n!!No record from previous measurement. Start testing again!!\n"),
                }
                break
            },
            TcState::Empty => {
                println!("Empty line in TC log");
                break
            },
        };
    }
    output
}

fn main() {
    let cli = Cli::from_args();

    let stand_nm = cli.cell;
    let stand_nm = match stand_nm {
        1..=4 => stand_nm,
        _ => panic!("We have only 4 stands!"),
    };
    println!("\nRunning measurement on test stand nm: {}\n", stand_nm);

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

    // last_line - is last line from log to check last status
    let mut output: HashMap<&str, String> = HashMap::new();
    output.entry("InterlockStatus").or_default();
    output.entry("TR_Number").or_default();
    output.entry("Specimen ID").or_default();
    output.entry("Test Request type").or_default();
    output.entry("Testing_Category").or_default();
    output.entry("Technician").or_default();
    output.entry("Available Time").or_default();
    output.entry("last_line").or_default();
    output.entry("opened_window").or_insert("No".to_string());

    let output = init_output(&config, &output).unwrap();
    //println!("{:?}", output);

    if output[&"opened_window"] == "Yes" {
        panic!("First close previous window!!!!");
    }

    loop {
        println!("\nWelcome in MADL choose from options below:");
        println!("d - Define test.");
        println!("c - Change time loss classification.");
        println!("s - Start test duration measurment");
        println!("e - Exit\n");
        print!(">>");
        let mut str_input = String::new();
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut str_input)
            .expect("Failed to read.");
        let answer = str_input;
        let answer = answer.trim().to_lowercase().clone();
        println!("Answer: {}", &answer);
        match  answer.as_ref() {
            "change" | "c" => start_change_timeloss(&config, &output),
            "define" | "d" => start_test_definition(&config, &output),
            "start"  | "s" => test_start_measurement(&config, &output),
            "exit"  | "e" => break,
            _ => {
                println!("Inserted wrong value, please insert again!");
                continue;
            },
        };
    }
}





