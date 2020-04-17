use madl::{Config, user_inputs, init_output, Laststate, check_state, end_of_test, start_of_test, write_test_loss_end};
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
    println!("Writen stand nm: {}", stand_nm);
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

    let mut output = init_output(&config, output).unwrap();
    println!("{:?}", output);

    if output[&"opened_window"] == "Yes" {
        panic!("First close previous window!!!!");
    }

    let last_state = match check_state(&output) {
        Ok(state) => state,
        Err(e) => panic!("Check_state function error: {:?}", e)
    };

    if cli.define {

        if let Err(e) = user_inputs(config, output) {
            eprintln!("Application error: {}", e);
            process::exit(1);
        }
    } else if cli.start {
        match last_state {
            Laststate::IN(vec_data) => {
                if vec_data[2].contains("Test Start") {
                    println!("Last log data are from start of test.\nNeeded to define end of previous test!");
                    end_of_test(&config, &mut output).unwrap();
                    start_of_test(&config, &output).unwrap();
                } else {
                    write_test_loss_end(&config, vec_data).unwrap();
                    start_of_test(&config, &output).unwrap();
                }

            }
            Laststate::OUT(_) => start_of_test(&config, &output).unwrap(),
            Laststate::EMPTY => start_of_test(&config, &output).unwrap(),
        }
    } else if cli.end {
        match last_state {
            Laststate::IN(vec_data) => {
                if vec_data[0].contains("Test Start") {
                    end_of_test(&config, &mut output).unwrap();
                } else {
                    write_test_loss_end(&config, vec_data).unwrap();
                }
            }
            Laststate::OUT(vec_data) => println!("Last activity is stoped: {}->{}->{}", vec_data[0], vec_data[1], vec_data[2]),
            Laststate::EMPTY => panic!("No record from previous measurement. Start testing again!"),
        }
    }
}
