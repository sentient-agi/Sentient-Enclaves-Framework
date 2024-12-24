// Copyright 2019-2021 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
// Author: Andrei Trandafir <aatrand@amazon.com>
// Author: Andrei Cipu <acipu@amazon.com>

//! ***NitroSecureModule multithread test for Rust API***
//! # Overview
//! This module implements a aggresive run-time test for the
//! NSM Rust API.

use aws_nitro_enclaves_nsm_api::api::{Request, Response};
use aws_nitro_enclaves_nsm_api::driver::{nsm_exit, nsm_init, nsm_process_request};
use serde_bytes::ByteBuf;
use std::convert::TryInto;
use std::sync::atomic;
use std::sync::Arc;
use std::thread;
use std::time;
use threadpool::ThreadPool;

enum ErrorCode {
    AttestationDocumentEmpty = 1,
    AttestationInvalidResponse = 2,
}

/// *Argument 2 (input)*: The NSM description.
fn extend_pcr(ctx: i32, j: usize) {
    let pcr: u16 = ((16 + j) & 15).try_into().unwrap();
    let one: u8 = ((j >> 24) & 0xFF).try_into().unwrap();
    let two: u8 = ((j >> 16) & 0xFF).try_into().unwrap();
    let three: u8 = ((j >> 8) & 0xFF).try_into().unwrap();
    let four: u8 = (j & 0xFF).try_into().unwrap();
    let dummy_data: Vec<u8> = vec![one, two, three, four];
    let mut _response: Response;

    // Extend the remaining PCRs multiple times.
    for _loop_idx in 0..2 {
        let data_copy = dummy_data.clone();
        _response = nsm_process_request(
            ctx,
            Request::ExtendPCR {
                index: pcr,
                data: data_copy,
            },
        );
    }
}

/// Check a single attestation operation.  
/// *Argument 1 (input)*: Context from `nsm_init()`.  
/// *Argument 2 (input)*: Optional user data.  
/// *Argument 3 (input)*: Optional nonce data.  
/// *Argument 4 (input)*: Optional public key.
/// Returns Ok(()) in case of success
fn check_single_attestation(
    ctx: i32,
    user_data: Option<ByteBuf>,
    nonce: Option<ByteBuf>,
    public_key: Option<ByteBuf>,
) -> Result<(), ErrorCode> {
    let response = nsm_process_request(
        ctx,
        Request::Attestation {
            user_data,
            nonce,
            public_key,
        },
    );
    match response {
        Response::Attestation { document } => {
            if document.is_empty() {
                println!("[Error] Attestation document is empty.");
                return Err(ErrorCode::AttestationDocumentEmpty);
            }
        }
        _ => {
            println!(
                "[Error] Request::Attestation got invalid response: {:?}",
                response
            );
            return Err(ErrorCode::AttestationInvalidResponse);
        }
    }
    Ok(())
}

/// Check multiple attestation operations.  
/// *Argument 1 (input)*: Context from `nsm_init()`.
/// Returns Ok(()) in case of success
fn check_attestation(ctx: i32, lp: usize) -> Result<(), ErrorCode> {
    const DATA_LEN: usize = 1024;
    let dummy_data: Vec<u8> = vec![128; DATA_LEN];
    let mut now = time::Instant::now();

    check_single_attestation(ctx, None, None, None)?;
    println!(
        "attestation loop={} wo/data took {} ns",
        lp,
        now.elapsed().as_nanos()
    );
    now = time::Instant::now();

    check_single_attestation(ctx, Some(ByteBuf::from(&dummy_data[..])), None, None)?;
    println!(
        "attestation loop={} w/data took {} ns",
        lp,
        now.elapsed().as_nanos()
    );
    now = time::Instant::now();

    check_single_attestation(
        ctx,
        Some(ByteBuf::from(&dummy_data[..])),
        Some(ByteBuf::from(&dummy_data[..])),
        None,
    )?;
    println!(
        "attestation loop={} w/data, nonce took {} ns",
        lp,
        now.elapsed().as_nanos()
    );
    now = time::Instant::now();

    check_single_attestation(
        ctx,
        Some(ByteBuf::from(&dummy_data[..])),
        Some(ByteBuf::from(&dummy_data[..])),
        Some(ByteBuf::from(&dummy_data[..])),
    )?;
    println!(
        "attestation loop={} w/user_data, nonce, PK took {} ns",
        lp,
        now.elapsed().as_nanos()
    );

    Ok(())
}

fn main() {
    println!("NSM test started.");

    let term = Arc::new(atomic::AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))
        .expect("Failed to register signal hook");

    let ctx = nsm_init();
    assert!(ctx >= 0, "[Error] NSM initialization returned {}.", ctx);

    // 90 threads is the limit for ~200M of memory
    let index = 90;

    let exit_code = Arc::new(atomic::AtomicI32::new(0));
    let pool = ThreadPool::new(index);
    let mut j = 0;

    // Wait until the sigterm
    while !term.load(atomic::Ordering::Relaxed) {
        if exit_code.load(atomic::Ordering::Relaxed) != 0 {
            break;
        }

        let waiting = pool.queued_count();
        if waiting >= 1 {
            println!("{} waiting", waiting);
            thread::sleep(time::Duration::from_millis(100));
        }
        let exit_code_t = Arc::clone(&exit_code);
        pool.execute(move || {
            let exit_code = check_attestation(ctx, j);
            if let Err(e) = exit_code {
                if let Err(er) = exit_code_t.compare_exchange(
                    0,
                    e as i32,
                    atomic::Ordering::Relaxed,
                    atomic::Ordering::Relaxed,
                ) {
                    println!("{:?}", er);
                }
            }
        });
        j += 1;
        pool.execute(move || {
            extend_pcr(ctx, j);
        });
        j += 1;
    } //while

    pool.join();
    nsm_exit(ctx);

    println!(
        "NSM test finished. Exitcode: {}",
        exit_code.load(atomic::Ordering::Relaxed)
    );

    std::process::exit(exit_code.load(atomic::Ordering::Relaxed));
}
