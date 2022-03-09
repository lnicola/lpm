#![forbid(unsafe_code)]

const EXTRACTION_OUTPUT_PATH: &str = "/var/cache/lpm";

pub mod extraction;
pub mod installation;
pub mod pkg;
pub mod validation;

pub trait ExtractionTasks {
    fn start_extraction(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn get_pkg_output_path(&self) -> String;
    fn half_extract(&self) -> Result<(), std::io::Error>;
    fn extract_meta_and_program(&self) -> Result<(), std::io::Error>;
    fn read_pkg_data(&mut self);
    fn cleanup(&self) -> Result<(), std::io::Error>;
}

pub trait InstallationTasks {
    fn start_installation(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn install_program(&self) -> Result<(), std::io::Error>;
}

pub trait ValidationTasks {
    fn start_validations(&self) -> Result<(), Box<dyn std::error::Error>>;
}
