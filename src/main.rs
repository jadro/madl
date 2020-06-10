use madl::{Config, user_inputs, Laststate, TestSpec,
    TcState, UpdateLog, create_config_files, end_of_test, testloose_inputs};
use std::process;
use std::collections::HashMap;
use madl::Cli;
use structopt::StructOpt;
use std::io::prelude::*;
use std::io;
use std::sync::mpsc;
use hotwatch::{Hotwatch, Event};


fn start_test_definition<'a>(config: &'a Config) {
    println!("Def started");
    let mut testspec = TestSpec::new(config);
    testspec.update_from_log().unwrap();
    println!("Def log update");
    testspec.update_from_tmp().unwrap();
    println!("Def tmp update");
    let output = match user_inputs(&config) {
        Err(e) => {
            eprintln!("Application error: {}", e);
            process::exit(1);
        },
        Ok(val) => val,
    };
    testspec.update_value(&output);
    println!("Def value update");
    testspec.write_temp_output().unwrap();
    println!("Def written");
}

fn start_change_timeloss<'a>(config: &'a Config) {
    let mut testspec = TestSpec::new(config);
    testspec.update_from_log().unwrap();
    let output = testspec.get_value();

    let last_state = testspec.check_state();
    let updatelog = UpdateLog::new(&config);

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
fn test_start_measurement<'a>(config: &Config) {
    let mut testspec = TestSpec::new(&config);
    testspec.update_from_tmp().unwrap();
    let (tx, rx) = mpsc::channel();
    let tcroot_folder = config.get_tc_log_folder_path();
    let updatelog = UpdateLog::new(&config);

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

        testspec.update_from_log().unwrap();
        let last_state = testspec.check_state();

        match received {
            TcState::Start(_) => {
                match last_state {
                    Laststate::IN(ref vec_data) => {
                        if vec_data[0].contains("Test Start") {
                            println!("\n!!Last log data are from start of test!!\n");
                            updatelog.write_missing_test_end().unwrap();
                            updatelog.write_test_definition(&testspec).unwrap();
                            updatelog.write_test_start().unwrap();
                        } else {
                            updatelog.write_test_loss_end(&vec_data).unwrap();
                            updatelog.write_test_definition(&testspec).unwrap();
                            updatelog.write_test_start().unwrap();
                        }
                    }
                    Laststate::OUT(_) => {
                        updatelog.write_test_definition(&testspec).unwrap();
                        updatelog.write_test_start().unwrap();
                    },
                    Laststate::EMPTY => {
                        updatelog.write_test_definition(&testspec).unwrap();
                        updatelog.write_test_start().unwrap();
                    },
                };
                println!("Measurment started!\n");
                testspec.remove_temp_file().unwrap();
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
            "change" | "c" => start_change_timeloss(&config),
            "define" | "d" => start_test_definition(&config),
            "start"  | "s" => test_start_measurement(&config),
            "exit"  | "e" => break,
            _ => {
                println!("Inserted wrong value, please insert again!");
                continue;
            },
        };
    }
}





