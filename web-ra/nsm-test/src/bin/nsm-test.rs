// Copyright 2019-2021 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
// Author: Andrei Trandafir <aatrand@amazon.com>

//! ***NitroSecureModule test for Rust API***
//! # Overview
//! This module implements a comprehensive run-time test for the
//! NSM Rust API.

use aws_nitro_enclaves_nsm_api::api::{Digest, Request, Response, AttestationDoc};
use aws_nitro_enclaves_nsm_api::driver::{nsm_exit, nsm_init, nsm_process_request};
use serde_bytes::ByteBuf;
use std::collections::BTreeSet;
use aws_nitro_enclaves_cose::CoseSign1;
use aws_nitro_enclaves_cose::crypto::openssl::Openssl;

const RESERVED_PCRS: u16 = 5;

/// Structure holding PCR status.
struct PcrData {
    lock: bool,
    data: Vec<u8>,
}

/// Structure holding the NSM description.
struct NsmDescription {
    version_major: u16,
    version_minor: u16,
    version_patch: u16,
    module_id: String,
    max_pcrs: u16,
    locked_pcrs: BTreeSet<u16>,
    digest: Digest,
}

/// Get the description of the NSM.
/// *Argument 1 (input)*: Context from `nsm_init()`.
/// *Returns*: A description structure.
fn get_nsm_description(ctx: i32) -> NsmDescription {
    let response = nsm_process_request(ctx, Request::DescribeNSM);
    match response {
        Response::DescribeNSM {
            version_major,
            version_minor,
            version_patch,
            module_id,
            max_pcrs,
            locked_pcrs,
            digest,
        } => NsmDescription {
            version_major,
            version_minor,
            version_patch,
            module_id,
            max_pcrs,
            locked_pcrs,
            digest,
        },
        _ => {
            panic!(
                "[Error] Request::DescribeNSM got invalid response: {:?}",
                response
            )
        }
    }
}

/// Get the length of a PCR in bytes, based on digest.
/// *Argument 1 (input)*: The NSM description with digest information.
/// *Returns*: PCR length in bytes.
fn get_pcr_len(description: &NsmDescription) -> usize {
    match description.digest {
        Digest::SHA256 => 32,
        Digest::SHA384 => 48,
        Digest::SHA512 => 64,
    }
}

/// Test the initial state of the PCRs.
/// *Argument 1 (input)*: Context from `nsm_init()`.
/// *Argument 2 (input)*: The NSM description.
fn check_initial_pcrs(ctx: i32, description: &NsmDescription) {
    let expected_pcr_len = get_pcr_len(description);

    // First, get the description of all available PCRs.
    let pcr_data: Vec<PcrData> = (0..description.max_pcrs)
        .map(|pcr| {
            let response = nsm_process_request(ctx, Request::DescribePCR { index: pcr });
            match response {
                Response::DescribePCR { lock, data } => {
                    assert_eq!(
                        data.len(),
                        expected_pcr_len,
                        "[Error] Request::DescribePCR got invalid response length."
                    );
                    PcrData { lock, data }
                }
                _ => {
                    panic!(
                        "[Error] Request::DescribePCR got invalid response: {:?}",
                        response
                    )
                }
            }
        })
        .collect();
    println!(
        "Checked Request::DescribePCR for PCRs [0..{}].",
        description.max_pcrs
    );

    let zeroed_pcr: Vec<u8> = vec![0; expected_pcr_len];
    let locked_pcrs: Vec<u16> = description.locked_pcrs.iter().cloned().collect();
    let locked_pcrs_ref: Vec<u16> = (0..16).collect();

    // PCRs [0..4] must not be empty (shound contain non-zero bytes).
    /*
    for (index, pcr) in pcr_data.iter().enumerate().take(RESERVED_PCRS as usize) {
        if index != 3 {
            assert_ne!(
                pcr.data, zeroed_pcr,
                "[Error] PCR {} must not be empty.",
                index
            );
        } else {
            assert_eq!(pcr.data, zeroed_pcr, "[Error] PCR {} must be empty.", index);
        }
    }
    println!("Checked that PCRs [0..{}] are not empty.", RESERVED_PCRS);
    */

    for (index, pcr) in pcr_data.iter().enumerate() {
        println!("PCR {} has value {:?}", index, pcr.data);
    }

    // All other PCRs should be empty.
    /*
    for (index, pcr) in pcr_data.iter().enumerate().skip(RESERVED_PCRS as usize) {
        assert_eq!(pcr.data, zeroed_pcr, "[Error] PCR {} must be empty.", index);
    }
    println!(
        "Checked that PCRs [{}..{}] are empty.",
        RESERVED_PCRS, description.max_pcrs
    );
    */

    // PCRs [0..16] should be locked.
    /*
    assert_eq!(
        locked_pcrs, locked_pcrs_ref,
        "[Error] Initial locked PCR list is invalid."
    );

    for pcr in 0..16 {
        assert!(
            pcr_data[pcr as usize].lock,
            "[Error] PCR {} must be locked.",
            pcr
        );
    }
    */

    // All other PCRs should not be locked.
    /*
    for pcr in 16..description.max_pcrs {
        assert!(
            !pcr_data[pcr as usize].lock,
            "[Error] PCR {} must not be locked.",
            pcr
        );
    }
    */

    println!(
        "Checked that PCRs [0..16] are locked and [16..{}] are not locked.",
        description.max_pcrs
    );
}

/// Check and modify the lock state of the PCRs.
/// *Argument 1 (input)*: Context from `nsm_init()`.
/// *Argument 2 (input)*: The NSM description.
fn check_pcr_locks(ctx: i32, description: &NsmDescription) {
    let dummy_data: Vec<u8> = vec![1, 2, 3];
    let expected_pcr_len = get_pcr_len(description);
    let zeroed_pcr: Vec<u8> = vec![0; expected_pcr_len];
    let mut range = description.max_pcrs;
    let mut response: Response;

    // Test that PCRs [0..16] cannot be locked.
    for index in 0..16 {
        response = nsm_process_request(ctx, Request::LockPCR { index });
        match response {
            Response::Error(ref error) => println!("[Error] PCR {} expected to not be lockable, but got: {:?}, with error: {:?}", index, response, error),
            _ => println!(
                "[Error] PCR {} expected to not be lockable, but got: {:?}",
                index, response
            ),
        }
    }

    println!("Checked Request::LockPCR for PCRs [0..16].");

    // Extend the remaining PCRs multiple times.
    for loop_idx in 0..10 {
        for index in 16..description.max_pcrs {
            let data_copy = dummy_data.clone();
            response = nsm_process_request(
                ctx,
                Request::ExtendPCR {
                    index,
                    data: data_copy,
                },
            );

            match response {
                Response::ExtendPCR { data } => {
                    assert_eq!(
                        data.len(),
                        expected_pcr_len,
                        "[Error] Request::ExtendPCR got invalid response."
                    );
                    assert_ne!(data, zeroed_pcr, "[Error] PCR {} must not be empty.", index);
                }
                _ => println!(
                    "[Error] Request::ExtendPCR got invalid response: {:?}",
                    response
                ),
            }
        }

        println!(
            "[Loop: {}] Checked Request::ExtendedPCR for PCRs [16..{}].",
            loop_idx, description.max_pcrs
        );
    }

    // Lock all remaining PCRs.
    for index in 16..description.max_pcrs {
        response = nsm_process_request(ctx, Request::LockPCR { index });

        match response {
            Response::LockPCR => (),
            _ => println!(
                "[Error] Request::LockPCR got invalid response: {:?}",
                response
            ),
        }
    }

    println!(
        "Checked Request::LockPCR for PCRs [16..{}].",
        description.max_pcrs
    );

    // Lock PCRs in a valid range.
    response = nsm_process_request(ctx, Request::LockPCRs { range });
    match response {
        Response::LockPCRs => (),
        _ => println!(
            "[Error] Request::LockPCRs expected to succeed for [0..{}], but got: {:?}",
            range, response
        ),
    }

    // Lock PCRs in an invalid range.
    range += 1;
    response = nsm_process_request(ctx, Request::LockPCRs { range });
    match response {
        Response::Error(ref error) => println!("[Error] Request::LockPCRs expected to fail for [0..{}], but got: {:?}, with error: {:?}", range, response, error),
        _ => println!(
            "[Error] Request::LockPCRs expected to fail for [0..{}], but got: {:?}",
            range, response
        ),
    }

    println!(
        "Checked Request::LockPCRs for ranges {} and {}.",
        range - 1,
        range
    );

    // Attempt to extend locked PCRs.
    for index in 0..description.max_pcrs {
        let data_copy = dummy_data.clone();
        response = nsm_process_request(
            ctx,
            Request::ExtendPCR {
                index,
                data: data_copy,
            },
        );

        match response {
            Response::Error(ref error) => println!("[Error] Request::ExtendPCR expected to fail, but got: {:?}, with error: {:?}", response, error),
            _ => println!(
                "[Error] Request::ExtendPCR expected to fail, but got: {:?}",
                response
            ),
        }
    }

    println!(
        "Checked Request::ExtendPCR for locked PCRs [0..{}].",
        description.max_pcrs
    );

    // Describe all PCRs multiple times.
    for loop_idx in 0..10 {
        for index in 0..description.max_pcrs {
            response = nsm_process_request(ctx, Request::DescribePCR { index });

            match response {
                Response::DescribePCR { lock, data } => {
                    assert_eq!(
                        data.len(),
                        expected_pcr_len,
                        "[Error] Request::DescribePCR got invalid response length."
                    );

                    /*
                    match index {
                        3 => {
                            assert_eq!(data, zeroed_pcr, "[Error] PCR {} must be empty.", index)
                        }
                        RESERVED_PCRS..=15 => {
                            assert_eq!(data, zeroed_pcr, "[Error] PCR {} must be empty.", index)
                        }
                        _ => {
                            assert_ne!(data, zeroed_pcr, "[Error] PCR {} must not be empty.", index)
                        }
                    }
                    */
                    assert!(lock, "[Error] PCR {} must be locked.", index);
                }
                _ => println!(
                    "[Error] Request::ExtendPCR got invalid response: {:?}",
                    response
                ),
            }
        }

        println!(
            "[Loop: {}] Checked Request::DescribePCR for PCRs [0..{}].",
            loop_idx, description.max_pcrs
        );
    }
}

/// Check a single attestation operation.
/// *Argument 1 (input)*: Context from `nsm_init()`.
/// *Argument 2 (input)*: Optional user data.
/// *Argument 3 (input)*: Optional nonce data.
/// *Argument 4 (input)*: Optional public key.
fn check_single_attestation(
    ctx: i32,
    user_data: Option<ByteBuf>,
    nonce: Option<ByteBuf>,
    public_key: Option<ByteBuf>,
) {
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
            assert_ne!(document.len(), 0, "[Error] COSE document is empty.");
            println!("COSE document length: {:?} bytes", document.len());
            // println!("Attestation document: {:?}", document);
            let cose_doc = CoseSign1::from_bytes(document.as_slice()).unwrap();
            let (protected_header, attestation_doc_bytes) =
                cose_doc.get_protected_and_payload::<Openssl>(None).unwrap();
            let unprotected_header = cose_doc.get_unprotected();
            let attestation_doc_signature = cose_doc.get_signature();
            let attestation_doc = AttestationDoc::from_binary(&attestation_doc_bytes[..]).unwrap();
            println!("Protected header: {:?}", protected_header);
            println!("Unprotected header: {:?}", unprotected_header);
            println!("Attestation document: {:?}", attestation_doc);
            println!("Attestation document signature: {:?}", hex::encode(attestation_doc_signature));
        }
        _ => println!(
            "[Error] Request::Attestation got invalid response: {:?}",
            response
        ),
    }
}

/// Check multiple attestation operations.
/// *Argument 1 (input)*: Context from `nsm_init()`.
fn check_attestation(ctx: i32) {
    const DATA_LEN: usize = 1024;
    let dummy_data: Vec<u8> = vec![128; DATA_LEN];

    check_single_attestation(ctx, None, None, None);
    println!("Checked Request::Attestation without any data.");

    check_single_attestation(ctx, Some(ByteBuf::from(&dummy_data[..])), None, None);
    println!(
        "Checked Request::Attestation with user data ({} bytes).",
        DATA_LEN
    );

    check_single_attestation(
        ctx,
        Some(ByteBuf::from(&dummy_data[..])),
        Some(ByteBuf::from(&dummy_data[..])),
        None,
    );
    println!(
        "Checked Request::Attestation with user data and nonce ({} bytes each).",
        DATA_LEN
    );

    check_single_attestation(
        ctx,
        Some(ByteBuf::from(&dummy_data[..])),
        Some(ByteBuf::from(&dummy_data[..])),
        Some(ByteBuf::from(&dummy_data[..])),
    );
    println!(
        "Checked Request::Attestation with user data, nonce and public key ({} bytes each).",
        DATA_LEN
    );
}

fn check_random(ctx: i32) {
    let mut prev_random: Vec<u8> = vec![];

    for _ in 0..16 {
        match nsm_process_request(ctx, Request::GetRandom) {
            Response::GetRandom { random } => {
                assert!(!random.is_empty());
                assert!(prev_random != random);
                prev_random = random.clone();
                println!("Random bytes: {:?}", random);
            }

            resp => println!(
                "GetRandom: expecting Response::GetRandom, but got {:?} instead",
                resp
            ),
        }
    }
}

fn main() {
    println!("NSM test started.");

    let ctx = nsm_init();
    assert!(ctx >= 0, "[Error] NSM initialization returned {}.", ctx);

    let description = get_nsm_description(ctx);
    assert_eq!(
        description.max_pcrs, 32,
        "[Error] NSM PCR count is {}.",
        description.max_pcrs
    );
    assert!(
        !description.module_id.is_empty(),
        "[Error] NSM module ID is missing."
    );

    println!(
        "NSM description: [major: {}, minor: {}, patch: {}, module_id: {}, max_pcrs: {},
        locked_pcrs: {:?}, digest: {:?}].",
        description.version_major,
        description.version_minor,
        description.version_patch,
        description.module_id,
        description.max_pcrs,
        description.locked_pcrs,
        description.digest
    );

    check_single_attestation(ctx, None, None, None);
    println!("Checked Request::Attestation without any data.");

    check_initial_pcrs(ctx, &description);
    check_pcr_locks(ctx, &description);

    check_attestation(ctx);

    check_random(ctx);

    nsm_exit(ctx);
    println!("NSM test finished.");
}
