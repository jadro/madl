use madl::{Config, user_inputs, init_output, Laststate, check_state, write_test_definition,
    DefFile,
    end_of_test, write_test_start, write_test_loss_end, testloose_inputs, write_test_loss};
use std::path::PathBuf;
use std::process;
use std::collections::HashMap;
use madl::Cli;
use structopt::StructOpt;

fn main() {
    let cli = Cli::from_args();

    let stand_nm = cli.cell;
    let stand_nm = match stand_nm {
        1..=4 => stand_nm,
        _ => panic!("We have only 4 stands!"),
    };
    println!("\nRunning measurement on test stand nm: {}\n", stand_nm);
    let choose_stand = format!("Teststand{}", stand_nm);


    let config = Config {
        settings_dir: PathBuf::from("C:\\Utilization Tool"),
        teststand_dir: PathBuf::from(choose_stand),
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

    let deffile = DefFile::new(&config);
    let mut output = init_output(&config, output).unwrap();
    //println!("{:?}", output);

    if output[&"opened_window"] == "Yes" {
        panic!("First close previous window!!!!");
    }

    let last_state = check_state(&output);

    if cli.define {
        let output =  deffile.read_temp_output(&mut output).unwrap();
        let output = match user_inputs(&config, output) {
            Err(e) => {
                eprintln!("Application error: {}", e);
                process::exit(1);
            },
            Ok(val) => val,
        };
        deffile.write_temp_output(&output).unwrap();

    } else if cli.start {
        match last_state {
            Laststate::IN(vec_data) => {
                if vec_data[0].contains("Test Start") {
                    println!("\n!!Last log data are from start of test!!\nNeeded to define end of previous test!\n");
                    end_of_test(&config, &mut output, true).unwrap();
                    let output = deffile.read_temp_output(&mut output).unwrap();
                    write_test_definition(&config, &output).unwrap();
                    deffile.remove_temp_file().unwrap();
                    write_test_start(&config).unwrap();
                } else {
                    write_test_loss_end(&config, vec_data).unwrap();
                    let output =  deffile.read_temp_output(&mut output).unwrap();
                    write_test_definition(&config, &output).unwrap();
                    deffile.remove_temp_file().unwrap();
                    write_test_start(&config).unwrap();
                }

            }
            Laststate::OUT(_) => {
                let output =  deffile.read_temp_output(&mut output).unwrap();
                write_test_definition(&config, &output).unwrap();
                deffile.remove_temp_file().unwrap();
                write_test_start(&config).unwrap();
            },
            Laststate::EMPTY => write_test_start(&config).unwrap(),
        }
    } else if cli.end {
        match last_state {
            Laststate::IN(vec_data) => {
                if vec_data[0].contains("Test Start") {
                    end_of_test(&config, &mut output, false).unwrap();
                } else {
                    write_test_loss_end(&config, vec_data).unwrap();
                }
            }
            Laststate::OUT(vec_data) => println!("\nLast activity is already stoped: {}->{}->{}\n", vec_data[0], vec_data[1], vec_data[2]),
            Laststate::EMPTY => panic!("\n!!No record from previous measurement. Start testing again!!\n"),
        }
    } else if cli.change_timeloss {
        match last_state {
            Laststate::IN(vec_data) => {
                if vec_data[0].contains("Test Start") {
                    end_of_test(&config, &mut output, false).unwrap();
                } else {
                    write_test_loss_end(&config, vec_data).unwrap();
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
    }
}
