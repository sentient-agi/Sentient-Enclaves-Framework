use serde::{Deserialize, Serialize};
use std::process::Output;
use tracing::{debug, error, info};

use clap::ArgMatches;

use crate::error::{PipelineError, Result};

#[derive(Debug, Clone)]
pub struct ListenArgs {
    pub port: u32,
}

impl ListenArgs {
    pub fn new_with(args: &ArgMatches) -> Result<Self> {
        debug!("Parsing ListenArgs from command line arguments");
        let port = parse_port(args)?;
        info!(port = port, "ListenArgs parsed successfully");
        Ok(ListenArgs { port })
    }
}

#[derive(Debug, Clone)]
pub struct RunArgs {
    pub cid: u32,
    pub port: u32,
    pub command: String,
    pub no_wait: bool,
}

impl RunArgs {
    pub fn new_with(args: &ArgMatches) -> Result<Self> {
        debug!("Parsing RunArgs from command line arguments");
        let cid = parse_cid(args)?;
        let port = parse_port(args)?;
        let command = parse_command(args)?;
        let no_wait = parse_no_wait(args);
        info!(cid = cid, port = port, command = %command, no_wait = no_wait, "RunArgs parsed successfully");
        Ok(RunArgs {
            cid,
            port,
            command,
            no_wait,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FileArgs {
    pub cid: u32,
    pub port: u32,
    pub localfile: String,
    pub remotefile: String,
}

impl FileArgs {
    pub fn new_with(args: &ArgMatches) -> Result<Self> {
        debug!("Parsing FileArgs from command line arguments");
        let cid = parse_cid(args)?;
        let port = parse_port(args)?;
        let localfile = parse_localfile(args)?;
        let remotefile = parse_remotefile(args)?;
        info!(cid = cid, port = port, localfile = %localfile, remotefile = %remotefile, "FileArgs parsed successfully");
        Ok(FileArgs {
            cid,
            port,
            localfile,
            remotefile,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DirArgs {
    pub cid: u32,
    pub port: u32,
    pub localdir: String,
    pub remotedir: String,
}

impl DirArgs {
    pub fn new_with(args: &ArgMatches) -> Result<Self> {
        debug!("Parsing DirArgs from command line arguments");
        let cid = parse_cid(args)?;
        let port = parse_port(args)?;
        let localdir = parse_localdir(args)?;
        let remotedir = parse_remotedir(args)?;
        info!(cid = cid, port = port, localdir = %localdir, remotedir = %remotedir, "DirArgs parsed successfully");
        Ok(DirArgs {
            cid,
            port,
            localdir,
            remotedir,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub rc: Option<i32>,
}

impl CommandOutput {
    pub fn new(stdout: String, stderr: String, code: i32) -> Self {
        debug!(code = code, "Creating new CommandOutput");
        CommandOutput {
            stdout,
            stderr,
            rc: Some(code),
        }
    }

    pub fn new_from(output: Output) -> Result<Self> {
        debug!("Creating CommandOutput from process Output");
        let stdout = String::from_utf8(output.stdout).map_err(|e| {
            error!(error = %e, "Failed to convert stdout to UTF-8");
            PipelineError::Utf8Error(e.utf8_error())
        })?;
        let stderr = String::from_utf8(output.stderr).map_err(|e| {
            error!(error = %e, "Failed to convert stderr to UTF-8");
            PipelineError::Utf8Error(e.utf8_error())
        })?;
        let rc = output.status.code();
        debug!(rc = ?rc, "CommandOutput created successfully");
        Ok(CommandOutput { stdout, stderr, rc })
    }
}

fn parse_cid(args: &ArgMatches) -> Result<u32> {
    debug!("Parsing cid argument");
    let cid_str = args.value_of("cid").ok_or_else(|| {
        error!("Could not find cid argument");
        PipelineError::ArgumentError("Could not find cid argument".to_string())
    })?;
    let cid = cid_str.parse().map_err(|e| {
        error!(value = cid_str, error = %e, "cid is not a valid number");
        PipelineError::ParseError {
            field: "cid".to_string(),
            message: format!("'{}' is not a valid number: {}", cid_str, e),
        }
    })?;
    debug!(cid = cid, "cid parsed successfully");
    Ok(cid)
}

fn parse_port(args: &ArgMatches) -> Result<u32> {
    debug!("Parsing port argument");
    let port_str = args.value_of("port").ok_or_else(|| {
        error!("Could not find port argument");
        PipelineError::ArgumentError("Could not find port argument".to_string())
    })?;
    let port = port_str.parse().map_err(|e| {
        error!(value = port_str, error = %e, "port is not a valid number");
        PipelineError::ParseError {
            field: "port".to_string(),
            message: format!("'{}' is not a valid number: {}", port_str, e),
        }
    })?;
    debug!(port = port, "port parsed successfully");
    Ok(port)
}

fn parse_command(args: &ArgMatches) -> Result<String> {
    debug!("Parsing command argument");
    let command = args.value_of("command").ok_or_else(|| {
        error!("Could not find command argument");
        PipelineError::ArgumentError("Could not find command argument".to_string())
    })?;
    debug!(command = %command, "command parsed successfully");
    Ok(String::from(command))
}

fn parse_no_wait(args: &ArgMatches) -> bool {
    let no_wait = args.is_present("no-wait");
    debug!(no_wait = no_wait, "no-wait flag parsed");
    no_wait
}

fn parse_localfile(args: &ArgMatches) -> Result<String> {
    debug!("Parsing localpath argument");
    let localfile = args.value_of("localpath").ok_or_else(|| {
        error!("Could not find localpath argument");
        PipelineError::ArgumentError("Could not find localpath argument".to_string())
    })?;
    debug!(localfile = %localfile, "localpath parsed successfully");
    Ok(String::from(localfile))
}

fn parse_remotefile(args: &ArgMatches) -> Result<String> {
    debug!("Parsing remotepath argument");
    let remotefile = args.value_of("remotepath").ok_or_else(|| {
        error!("Could not find remotepath argument");
        PipelineError::ArgumentError("Could not find remotepath argument".to_string())
    })?;
    debug!(remotefile = %remotefile, "remotepath parsed successfully");
    Ok(String::from(remotefile))
}

fn parse_localdir(args: &ArgMatches) -> Result<String> {
    debug!("Parsing localdir argument");
    let localdir = args.value_of("localdir").ok_or_else(|| {
        error!("Could not find localdir argument");
        PipelineError::ArgumentError("Could not find localdir argument".to_string())
    })?;
    debug!(localdir = %localdir, "localdir parsed successfully");
    Ok(String::from(localdir))
}

fn parse_remotedir(args: &ArgMatches) -> Result<String> {
    debug!("Parsing remotedir argument");
    let remotedir = args.value_of("remotedir").ok_or_else(|| {
        error!("Could not find remotedir argument");
        PipelineError::ArgumentError("Could not find remotedir argument".to_string())
    })?;
    debug!(remotedir = %remotedir, "remotedir parsed successfully");
    Ok(String::from(remotedir))
}
