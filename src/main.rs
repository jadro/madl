use madl::{Config, user_inputs, get_log_data};
use std::path::PathBuf;
use std::process;
use std::collections::HashMap;

fn main() {
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

    let mut output: HashMap<&str, String> = HashMap::new();
    output.entry("InterlockStatus").or_default();
    output.entry("TR_Number").or_default();
    output.entry("Specimen ID").or_default();
    output.entry("Test Request type").or_default();
    output.entry("Testing_Category").or_default();
    output.entry("Technician").or_default();
    output.entry("Available Time").or_default();


    if let Err(e) = user_inputs(config, output) {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}
