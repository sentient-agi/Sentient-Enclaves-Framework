# File System Monitor

A Rust-based file system monitoring tool that tracks file changes in real-time using the `notify` crate.

## Overview

This tool monitors file system events in a specified directory and its subdirectories, tracking:
- File modifications
- File/directory creation
- File close events

## Features
- **Recursive Monitoring**: Automatically watches all subdirectories
- **Duplicate Event Prevention**: Tracks modified files to prevent duplicate notifications
- **Event Types**:
  - File modifications
  - File creation
  - File close events (with write)
  - Other file system events
Each event is logged with its type and the affected file path. A ~~`HashMap`~~`DashMap` is used to track modified files and save hashes of files.

## Implementation
The monitor uses the `notify` crate which provides a cross-platform file system monitoring solution.  It:
1. Sets up a watcher on the specified directory, this is done recursively.
2. Processes events through a channel.
3. Filters and saves relevant file system events in a ~~`HashMap`~~`DashMap` with the path as the key and a `FileInfo` struct as the value.
4. The `FileInfo` struct keeps getting updated based on the event type.
5. Once a file has been downloaded it is made immutable and its hash is saved in the ~~`HashMap`~~`DashMap`.
6. When hash of a file is requested, it is looked up in the ~~`HashMap`~~`DashMap` and if found, the hash is returned.

No information about a directory is saved in the ~~`HashMap`~~`DashMap`. When a directory is created, all files in it are watched. For hashing the directory concatenation of it's contents is returned. This can be modified based on the use case.

## Known Issues

- ~~Slow lookup of already hashed files from the `HashMap` -> Needs multiple threads to speed it up.~~ -> Improved lookup logic
- ~~Slow hashing of files -> Needs multiple threads to speed it up.~~ -> Used independent threads for hashing calculations
- Manually adding folders to be excluded from monitoring -> A configuration file would be better.
- Deletion of files is not tracked -> Clean up the ~~`HashMap`~~`DashMap` when a file is deleted. For directories no need to do anything.
- Renaming of files is not tracked -> Update the `Key` in the ~~`HashMap`~~`DashMap` when a file is renamed.
