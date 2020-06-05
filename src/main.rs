use madl::{Config, user_inputs, Laststate, TestSpec,
    DefFile, TcState, UpdateLog, create_config_files, end_of_test, testloose_inputs};
use std::process;
use std::collections::HashMap;
use madl::Cli;
use structopt::StructOpt;
use std::io::prelude::*;
use std::io;
use std::sync::mpsc;
use hotwatch::{Hotwatch, Event};

fn init_output<'a>() -> HashMap<&'a str, String> {
    // last_line - is last line from log to check last status
    let mut output: HashMap<&'a str, String> = HashMap::new();
    output.entry("InterlockStatus").or_default();
    output.entry("TR_Number").or_default();
    output.entry("Specimen ID").or_default();
    output.entry("Test Request type").or_default();
    output.entry("Testing_Category").or_default();
    output.entry("Technician").or_default();
    output.entry("Available Time").or_default();
    output.entry("last_line").or_default();

    output
}

fn start_test_definition<'a>(config: &'a Config, output: &'a HashMap<&'a str, String>) {
    let mut testspec = TestSpec::new(output.to_owned(), config);
    testspec.update_output().unwrap();
    let output = testspec.get_value();
    let deffile = DefFile::new(&config);
    let output =  deffile.read_temp_output(output.to_owned()).unwrap();
    let output = match user_inputs(&config, output) {
        Err(e) => {
            eprintln!("Application error: {}", e);
            process::exit(1);
        },
        Ok(val) => val,
    };
    deffile.write_temp_output(&output).unwrap();
}

fn start_change_timeloss<'a>(config: &'a Config, output: &'a HashMap<&'a str, String>) {
    let mut testspec = TestSpec::new(output.to_owned(), config);
    testspec.update_output().unwrap();
    let output = testspec.get_value();

    let last_state = testspec.check_state();
    let updatelog = UpdateLog::new(&config, &output);

    match last_state {
        Laststate::IN(vec_data) => {
            if vec_data[0].contains("Test Start") {
                end_of_test(&updatelog, false).unwrap();
            } else {
                updatelog.write_test_loss_end(&vec_data).unwrap();
            }
            let out = testloose_inputs(&config).unwrap();
            updatelog.write_test_loss(out).unwrap();
        },
        Laststate::OUT(_) => {
            let out = testloose_inputs(&config).unwrap();
            updatelog.write_test_loss(out).unwrap();
        },
        Laststate::EMPTY => {
            let out = testloose_inputs(&config).unwrap();
            updatelog.write_test_loss(out).unwrap();
        },
    }
}

// Get definition of test
fn test_start_measurement<'a>(config: &Config, output: &'a HashMap<&'a str, String>) {
    let output = output.to_owned();
    let deffile = DefFile::new(&config);
    let output = deffile.read_temp_output(output).unwrap();
    let (tx, rx) = mpsc::channel();
    let tcroot_folder = config.get_tc_log_folder_path();
    let updatelog = UpdateLog::new(&config, &output);

    // watch_folder(tcroot_folder, tx);

    //println!("Checking log folder");
    let mut hotwatch = Hotwatch::new().expect("hotwatch failed to initialize!");
    hotwatch.watch(tcroot_folder, move |event: Event| {
        if let Event::Write(path) = event {
            //println!("Log file: {:?} changed!", path.display());
            tx.send(TcState::read_log(path)).unwrap();
        }
    }).expect("failed to watch file!");

    for received in rx {
        let mut testspec = TestSpec::new(output.to_owned(), &config);
        testspec.update_output().unwrap();
        testspec.get_value();
        let last_state = testspec.check_state();

        match received {
            TcState::Start(_) => {
                match last_state {
                    Laststate::IN(ref vec_data) => {
                        if vec_data[0].contains("Test Start") {
                            println!("\n!!Last log data are from start of test!!\n");
                            updatelog.write_missing_test_end().unwrap();
                            updatelog.write_test_definition().unwrap();
                            updatelog.write_test_start().unwrap();
                        } else {
                            updatelog.write_test_loss_end(&vec_data).unwrap();
                            updatelog.write_test_definition().unwrap();
                            updatelog.write_test_start().unwrap();
                        }
                    }
                    Laststate::OUT(_) => {
                        updatelog.write_test_definition().unwrap();
                        updatelog.write_test_start().unwrap();
                    },
                    Laststate::EMPTY => {
                        updatelog.write_test_definition().unwrap();
                        updatelog.write_test_start().unwrap();
                    },
                };
                println!("Measurment started!\n");
                deffile.remove_temp_file().unwrap();
            },
            TcState::End(_) => {
                match last_state {
                    Laststate::IN(ref vec_data) => {
                        if vec_data[0].contains("Test Start") {
                            if end_of_test(&updatelog, false).unwrap() {
                                println!("Continue in testing");
                                continue
                            };
                        } else {
                            updatelog.write_test_loss_end(vec_data).unwrap();
                        }
                    }
                    Laststate::OUT(ref vec_data) => {
                        println!("\nLast activity is already stoped: {}->{}->{}\n", vec_data[0], vec_data[1], vec_data[2]);
                        continue
                    },
                    Laststate::EMPTY => panic!("\n!!No record from previous measurement. Start testing again!!\n"),
                }
                println!("Measurment end!\n");
                break
            },
            TcState::Empty => {
                println!("Empty line in TC log");
                break
            },
        };
    }
}

fn main() {
    let cli = Cli::from_args();

    let stand_nm = cli.cell;
    let stand_nm = match stand_nm {
        1..=4 => stand_nm,
        _ => panic!("We have only 4 stands!"),
    };
    println!("\nRunning measurement on test stand nm: {}\n", stand_nm);

    let config = Config::new(stand_nm).unwrap();
    create_config_files(&config);
    let output = init_output();
    //println!("{:?}", output);

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
        //println!("Answer: {}", &answer);
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





