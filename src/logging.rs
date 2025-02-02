use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

pub fn app_config(file_name: &str, level: LevelFilter) -> Config {
    let log_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {l} - {m}{n}")))
        .build(file_name)
        .unwrap();

    Config::builder()
        .appender(Appender::builder().build("file", Box::new(log_file)))
        .build(Root::builder().appender("file").build(level))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;
    use std::fs::{remove_file, File};
    use std::io::Read;
    use std::path::Path;

    #[test]
    fn test_logger_config_creation() {
        let file_name = "test.file";
        let _handle = log4rs::init_config(app_config(file_name, LevelFilter::Info)).unwrap();
        log::info!("test logging");

        let path = Path::new(file_name);
        let display = path.display();
        assert!(path.exists());
        let mut file = match File::open(path) {
            Err(why) => panic!("couldn't open {}: {}", display, why),
            Ok(file) => file,
        };

        let mut s = String::new();
        match file.read_to_string(&mut s) {
            Err(why) => panic!("couldn't read {}: {}", display, why),
            Ok(_) => {
                print!("{} contains:\n{}", display, s);
                assert!(s.contains("test logging"));
                let reg_match = Regex::new("\\d{4}-\\d{2}-\\d{1,2}T\\d{1,2}:\\d{1,2}:\\d{1,2}\\.\\d+\\+\\d{2}:\\d{2} - INFO - test logging").unwrap();
                assert!(reg_match.is_match(&s))
            }
        }
        remove_file(path).unwrap_or_else(|_| panic!("unable to remove file: {}", file_name));
    }

    // #[test]
    //TODO: fix this test so wrong config gets verified
    fn test_use_wrong_level() {
        let file_name = "test.file";
        log4rs::init_config(app_config(file_name, LevelFilter::Error)).unwrap();
        log::info!("test logging");

        let path = Path::new(file_name);

        let mut file = match File::open(path) {
            Err(why) => panic!("couldn't open file: {}", why),
            Ok(file) => file,
        };
        let mut s = String::new();
        file.read_to_string(&mut s).expect("couldn't read file");
        assert_eq!(s.len(), 0);
    }
}
